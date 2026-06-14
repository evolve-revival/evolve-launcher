use std::net::UdpSocket;
use std::time::Duration;

const STUN_MAGIC: u32 = 0x2112A442;
const STUN_MAGIC_PORT_XOR: u16 = 0x2112;
const STUN_TXN_ID: &[u8; 12] = b"evolve_stun!";

#[derive(Debug, Clone, serde::Serialize)]
pub struct NatInfo {
    pub external_ip: String,
    pub external_port: u16,
    /// "direct" if STUN succeeded, "relay-only" if it failed
    pub nat_type: String,
}

fn build_binding_request() -> [u8; 20] {
    let mut msg = [0u8; 20];
    msg[0] = 0x00;
    msg[1] = 0x01;
    // length = 0
    msg[4] = 0x21;
    msg[5] = 0x12;
    msg[6] = 0xA4;
    msg[7] = 0x42;
    msg[8..20].copy_from_slice(STUN_TXN_ID);
    msg
}

fn parse_xor_mapped_address(buf: &[u8]) -> Option<(String, u16)> {
    if buf.len() < 20 {
        return None;
    }
    if buf[0] != 0x01 || buf[1] != 0x01 {
        return None;
    }
    if buf[4] != 0x21 || buf[5] != 0x12 || buf[6] != 0xA4 || buf[7] != 0x42 {
        return None;
    }
    let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
    if buf.len() < 20 + msg_len {
        return None;
    }
    let mut offset = 20usize;
    while offset + 4 <= 20 + msg_len {
        let attr_type = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
        let attr_len = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
        if offset + 4 + attr_len > buf.len() {
            break;
        }
        if attr_type == 0x0020 && attr_len >= 8 {
            let val = &buf[offset + 4..offset + 4 + attr_len];
            if val[1] != 0x01 {
                break; // only IPv4
            }
            let port = u16::from_be_bytes([val[2], val[3]]) ^ STUN_MAGIC_PORT_XOR;
            let ip_int = u32::from_be_bytes([val[4], val[5], val[6], val[7]]) ^ STUN_MAGIC;
            let ip = std::net::Ipv4Addr::from(ip_int);
            return Some((ip.to_string(), port));
        }
        offset += 4 + ((attr_len + 3) & !3);
    }
    None
}

/// Send a STUN Binding Request to relay_host:relay_port and return the
/// caller's external IP and port as seen by the relay.
pub fn probe_stun(relay_host: &str, relay_port: u16) -> Result<NatInfo, String> {
    let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_secs(4)))
        .map_err(|e| e.to_string())?;

    let server_addr = format!("{relay_host}:{relay_port}");
    let request = build_binding_request();
    sock.send_to(&request, &server_addr)
        .map_err(|e| format!("STUN send failed: {e}"))?;

    let mut buf = [0u8; 512];
    let (n, _) = sock
        .recv_from(&mut buf)
        .map_err(|_| "STUN timeout — relay unreachable".to_string())?;

    let (ext_ip, ext_port) = parse_xor_mapped_address(&buf[..n])
        .ok_or_else(|| "Malformed STUN response".to_string())?;

    Ok(NatInfo {
        external_ip: ext_ip,
        external_port: ext_port,
        nat_type: "direct".to_string(),
    })
}

// ── Proxy ─────────────────────────────────────────────────────────────────

pub struct ProxyHandle {
    pub shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ProxyHandle {
    pub fn stop(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Start a local UDP proxy on 127.0.0.1:47584.
///
/// Goldberg's custom_broadcasts.txt points here. The proxy forwards all
/// Goldberg packets to the VPS relay and delivers packets from relay/peers
/// back to Goldberg. When the relay sends a "PUNCH <addr>" signal the proxy
/// fires a hole-punch UDP to that address and records it as a direct peer for
/// future packets.
///
/// Returns `(ProxyHandle, Option<SocketAddr>)` where the second value is the
/// external endpoint of the relay socket as seen by the STUN server (useful
/// for peer registration). It is `None` if the STUN probe through the relay
/// socket fails.
pub async fn start_proxy(
    relay_host: String,
    relay_port: u16,
) -> Result<(ProxyHandle, Option<std::net::SocketAddr>), String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    // Bind local socket for Goldberg.
    let local = Arc::new(
        tokio::net::UdpSocket::bind("127.0.0.1:47584")
            .await
            .map_err(|e| format!("Proxy: cannot bind 127.0.0.1:47584 — {e}"))?,
    );

    // Bind outbound socket (any port) for relay + direct peers.
    let relay_sock = Arc::new(
        tokio::net::UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Proxy: relay socket bind failed — {e}"))?,
    );

    // Fix 2: resolve hostname via DNS instead of .parse() (which only works
    // for numeric IPs).
    let relay_addr = tokio::net::lookup_host(format!("{relay_host}:{relay_port}"))
        .await
        .map_err(|e| format!("Proxy: DNS resolution failed — {e}"))?
        .next()
        .ok_or_else(|| "Proxy: relay hostname resolved to nothing".to_string())?;

    // Fix 4: send a STUN probe through the relay socket *before* spawning the
    // forwarding tasks so neither task steals our response.  This gives us the
    // actual external endpoint of the relay socket rather than the ephemeral
    // socket used by probe_stun().
    let relay_external: Option<std::net::SocketAddr> = {
        let stun_req = build_binding_request();
        relay_sock.send_to(&stun_req, relay_addr).await.ok();

        let mut stun_buf = [0u8; 512];
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(4),
            relay_sock.recv_from(&mut stun_buf),
        )
        .await
        {
            Ok(Ok((n, _))) => parse_xor_mapped_address(&stun_buf[..n])
                .and_then(|(ip, port)| format!("{ip}:{port}").parse().ok()),
            _ => None,
        }
    };

    let shutdown = Arc::new(AtomicBool::new(false));

    // Shared state between the two tasks.
    let goldberg_addr: Arc<Mutex<Option<std::net::SocketAddr>>> = Arc::new(Mutex::new(None));
    let direct_peers: Arc<Mutex<Vec<std::net::SocketAddr>>> = Arc::new(Mutex::new(Vec::new()));

    // ── Task A: local → relay/direct ──────────────────────────────────────
    {
        let local_r = Arc::clone(&local);
        let relay_w = Arc::clone(&relay_sock);
        let ga_w = Arc::clone(&goldberg_addr);
        let dp_r = Arc::clone(&direct_peers);
        let sd = Arc::clone(&shutdown);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 65507];
            while !sd.load(Ordering::Relaxed) {
                let Ok((n, from)) = local_r.recv_from(&mut buf).await else {
                    break;
                };
                *ga_w.lock().unwrap() = Some(from);

                let packet = buf[..n].to_vec();
                let peers = dp_r.lock().unwrap().clone();

                if peers.is_empty() {
                    // No direct peers yet — send to relay.
                    let _ = relay_w.send_to(&packet, relay_addr).await;
                } else {
                    // Direct path: send to every known peer.
                    for peer in &peers {
                        let _ = relay_w.send_to(&packet, peer).await;
                    }
                }
            }
        });
    }

    // ── Task B: relay/direct → local ──────────────────────────────────────
    {
        let relay_r = Arc::clone(&relay_sock);
        let local_w = Arc::clone(&local);
        let ga_r = Arc::clone(&goldberg_addr);
        let dp_w = Arc::clone(&direct_peers);
        let sd = Arc::clone(&shutdown);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 65507];
            while !sd.load(Ordering::Relaxed) {
                let Ok((n, from)) = relay_r.recv_from(&mut buf).await else {
                    break;
                };

                // Detect PUNCH signal from relay: "PUNCH <ip>:<port>"
                if n > 6 && &buf[..6] == b"PUNCH " {
                    let addr_str = std::str::from_utf8(&buf[6..n])
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if let Ok(peer_addr) = addr_str.parse::<std::net::SocketAddr>() {
                        // Fire hole-punch packet to open NAT path.
                        let _ = relay_r.send_to(b"PUNCH_ACK", peer_addr).await;
                        dp_w.lock().unwrap().push(peer_addr);
                    }
                    continue;
                }

                // Ignore PUNCH_ACK confirmations from peers.
                if n == 9 && &buf[..9] == b"PUNCH_ACK" {
                    let mut peers = dp_w.lock().unwrap();
                    if !peers.contains(&from) {
                        peers.push(from);
                    }
                    continue;
                }

                // Game packet — forward to Goldberg.
                let ga_addr = *ga_r.lock().unwrap();
                if let Some(ga) = ga_addr {
                    let _ = local_w.send_to(&buf[..n], ga).await;
                }
            }
        });
    }

    Ok((ProxyHandle { shutdown }, relay_external))
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_binding_request_has_magic_cookie() {
        let req = build_binding_request();
        assert_eq!(&req[0..2], &[0x00, 0x01], "method bytes");
        assert_eq!(&req[4..8], &[0x21, 0x12, 0xA4, 0x42], "magic cookie");
    }

    #[test]
    fn parse_xor_mapped_address_rejects_short() {
        assert!(parse_xor_mapped_address(&[0u8; 10]).is_none());
    }

    #[test]
    fn parse_xor_mapped_address_rejects_wrong_type() {
        let mut buf = [0u8; 32];
        // Not a success response (0x0101)
        buf[0] = 0x00;
        buf[1] = 0x01;
        buf[4] = 0x21;
        buf[5] = 0x12;
        buf[6] = 0xA4;
        buf[7] = 0x42;
        assert!(parse_xor_mapped_address(&buf).is_none());
    }

    #[test]
    fn stun_round_trip_encode_decode() {
        // Build a fake STUN success response by hand and verify decode.
        let ip: u32 = u32::from(std::net::Ipv4Addr::new(203, 0, 113, 5));
        let port: u16 = 54321;
        let xor_ip = ip ^ STUN_MAGIC;
        let xor_port = port ^ STUN_MAGIC_PORT_XOR;

        let mut buf = vec![0u8; 32];
        buf[0] = 0x01;
        buf[1] = 0x01;
        buf[2] = 0x00;
        buf[3] = 0x0C; // attr len 12
        buf[4] = 0x21;
        buf[5] = 0x12;
        buf[6] = 0xA4;
        buf[7] = 0x42;
        // transaction ID bytes 8-19: zeros
        // XOR-MAPPED-ADDRESS attr
        buf[20] = 0x00;
        buf[21] = 0x20;
        buf[22] = 0x00;
        buf[23] = 0x08;
        buf[24] = 0x00; // reserved
        buf[25] = 0x01; // IPv4
        buf[26..28].copy_from_slice(&xor_port.to_be_bytes());
        buf[28..32].copy_from_slice(&xor_ip.to_be_bytes());

        let (got_ip, got_port) = parse_xor_mapped_address(&buf).expect("should parse");
        assert_eq!(got_ip, "203.0.113.5");
        assert_eq!(got_port, 54321);
    }
}
