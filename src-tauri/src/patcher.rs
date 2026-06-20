use crate::donor;
use crate::downloader::download_with_retry;
use crate::install::Manifest;
use reqwest::Client;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Parse the hostname from a URL like "https://host:port/path" → "host"
pub fn extract_host(url: &str) -> String {
    let without_scheme = url.find("://").map(|i| &url[i + 3..]).unwrap_or(url);
    let host_and_port = without_scheme
        .find('/')
        .map(|i| &without_scheme[..i])
        .unwrap_or(without_scheme);
    host_and_port
        .rfind(':')
        .map(|i| &host_and_port[..i])
        .unwrap_or(host_and_port)
        .to_string()
}

/// Parse the port from a URL. Falls back to 443 for https, 80 for http.
pub fn extract_port(url: &str) -> u16 {
    let without_scheme = url.find("://").map(|i| &url[i + 3..]).unwrap_or(url);
    let host_and_port = without_scheme
        .find('/')
        .map(|i| &without_scheme[..i])
        .unwrap_or(without_scheme);
    if let Some(i) = host_and_port.rfind(':') {
        if let Ok(p) = host_and_port[i + 1..].parse::<u16>() {
            return p;
        }
    }
    if url.starts_with("https://") {
        443
    } else {
        80
    }
}

/// Generate EvolveLogging.ini content pointing at the revival server,
/// Generate EvolveLogging.ini for the revival server.
/// emu_steam = true: Pinenut uses its Goldberg emulator for Steam identity
/// (hooks GetEnvironmentVariableA + RegQueryValueExA so SteamAPI_Init works
/// without the game being in Steam's library).
pub fn generate_logging_ini(server_url: &str) -> String {
    let host = extract_host(server_url);
    format!(
        "[server]\n\
         server_domain = {host}\n\
         server_port = 443\n\
         use_internal_server = false\n\
         \n\
         [steam]\n\
         emu_steam = true\n\
         dll_path = GoldbergNewEvolveEmu.dll\n"
    )
}

/// Content for steam_appid.txt — real Steamworks reads this to determine App ID at startup.
pub fn steam_appid_content() -> String {
    format!("{}\n", donor::DONOR_APP_ID)
}

/// Download and apply all patch files, then write EvolveLogging.ini and
/// steam_appid.txt so the game uses the donor App ID for Steam session auth.
pub async fn apply_patches(
    client: &Client,
    manifest: &Manifest,
    install_dir: &Path,
    server_url: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(), String> {
    for patch in &manifest.patches {
        // Guard against path traversal attacks via a compromised manifest
        if std::path::Path::new(&patch.path).is_absolute() {
            return Err(format!(
                "Manifest contains absolute patch path: {}",
                patch.path
            ));
        }
        let dest = install_dir.join(&patch.path);
        if !dest.starts_with(install_dir) {
            return Err(format!(
                "Manifest patch path escapes install dir: {}",
                patch.path
            ));
        }
        let url = format!("{}{}", manifest.base_url, patch.path);
        download_with_retry(client, &url, &dest, &patch.sha256, cancelled.clone()).await?;
    }

    let bin_dir = install_dir.join("bin64_SteamRetail");
    std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

    std::fs::write(
        bin_dir.join("EvolveLogging.ini"),
        generate_logging_ini(server_url),
    )
    .map_err(|e| format!("Failed to write EvolveLogging.ini: {e}"))?;

    std::fs::write(bin_dir.join("steam_appid.txt"), steam_appid_content())
        .map_err(|e| format!("Failed to write steam_appid.txt: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_host_from_https_url() {
        assert_eq!(
            extract_host("https://revival.example.com:8080"),
            "revival.example.com"
        );
    }

    #[test]
    fn extracts_host_from_http_url() {
        assert_eq!(extract_host("http://192.168.1.1:2000"), "192.168.1.1");
    }

    #[test]
    fn extracts_host_with_no_port() {
        assert_eq!(
            extract_host("https://cdn.evolve-revival.com"),
            "cdn.evolve-revival.com"
        );
    }

    #[test]
    fn extracts_port_explicit() {
        assert_eq!(extract_port("https://revival.example.com:8443/path"), 8443);
        assert_eq!(extract_port("http://192.168.1.1:2000"), 2000);
    }

    #[test]
    fn extracts_port_implicit() {
        assert_eq!(extract_port("https://cdn.evolve-revival.com"), 443);
        assert_eq!(extract_port("http://cdn.evolve-revival.com"), 80);
    }

    #[test]
    fn generates_correct_ini_content() {
        let ini = generate_logging_ini("https://revival.example.com:8443");
        assert!(ini.contains("[server]"));
        assert!(ini.contains("server_domain = revival.example.com"));
        assert!(ini.contains("server_port = 443"));
        assert!(ini.contains("use_internal_server = false"));
        assert!(ini.contains("[steam]"));
        assert!(ini.contains("emu_steam = true"));
        assert!(ini.contains("dll_path = GoldbergNewEvolveEmu.dll"));
    }

    #[test]
    fn generates_ini_with_default_https_port() {
        let ini = generate_logging_ini("https://play.evolve-community.net");
        assert!(ini.contains("server_port = 443"));
        assert!(ini.contains("emu_steam = true"));
    }

    #[test]
    fn extracts_host_for_broadcasts_file() {
        assert_eq!(
            extract_host("https://revival.example.com:8443/"),
            "revival.example.com"
        );
        assert_eq!(extract_host("http://192.168.1.50:8080"), "192.168.1.50");
        assert_eq!(
            extract_host("https://play.evolve-revival.com"),
            "play.evolve-revival.com"
        );
    }

    #[test]
    fn steam_appid_content_is_donor_id() {
        assert_eq!(steam_appid_content(), "480\n");
    }
}
