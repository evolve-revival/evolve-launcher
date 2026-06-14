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

// ── Proxy stub (implemented in Task 6) ───────────────────────────────────

pub struct ProxyHandle {
    pub shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ProxyHandle {
    pub fn stop(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub async fn start_proxy(
    _relay_host: String,
    _relay_port: u16,
) -> Result<ProxyHandle, String> {
    Ok(ProxyHandle {
        shutdown: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    })
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
