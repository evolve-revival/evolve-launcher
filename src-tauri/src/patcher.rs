use crate::downloader::download_with_retry;
use crate::install::Manifest;
use reqwest::Client;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Parse the hostname from a URL like "https://host:port/path" → "host"
pub fn extract_host(url: &str) -> String {
    // Strip scheme
    let without_scheme = url
        .find("://")
        .map(|i| &url[i + 3..])
        .unwrap_or(url);
    // Take up to first slash
    let host_and_port = without_scheme
        .find('/')
        .map(|i| &without_scheme[..i])
        .unwrap_or(without_scheme);
    // Strip port
    host_and_port
        .rfind(':')
        .map(|i| &host_and_port[..i])
        .unwrap_or(host_and_port)
        .to_string()
}

/// Generate EvolveLogging.ini content with the revival server host
pub fn generate_logging_ini(server_url: &str) -> String {
    let host = extract_host(server_url);
    format!(
        "[Logging]\nserver_domain={}\nserver_port=2000\nuse_internal_server=false\ninternal_server_dll=EvolveLegacyRebornServer.dll\n",
        host
    )
}

/// Download and apply all patch files, then write EvolveLogging.ini
pub async fn apply_patches(
    client: &Client,
    manifest: &Manifest,
    install_dir: &Path,
    server_url: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(), String> {
    for patch in &manifest.patches {
        let url = format!("{}{}", manifest.base_url, patch.path);
        let dest = install_dir.join(&patch.path);
        download_with_retry(client, &url, &dest, &patch.sha256, cancelled.clone()).await?;
    }

    // Generate and write EvolveLogging.ini
    std::fs::create_dir_all(install_dir).map_err(|e| e.to_string())?;
    let ini_content = generate_logging_ini(server_url);
    let ini_path = install_dir.join("EvolveLogging.ini");
    std::fs::write(&ini_path, ini_content)
        .map_err(|e| format!("Failed to write EvolveLogging.ini: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_host_from_https_url() {
        assert_eq!(extract_host("https://revival.example.com:8080"), "revival.example.com");
    }

    #[test]
    fn extracts_host_from_http_url() {
        assert_eq!(extract_host("http://192.168.1.1:2000"), "192.168.1.1");
    }

    #[test]
    fn extracts_host_with_no_port() {
        assert_eq!(extract_host("https://cdn.evolve-revival.com"), "cdn.evolve-revival.com");
    }

    #[test]
    fn generates_correct_ini_content() {
        let ini = generate_logging_ini("https://revival.example.com:8080");
        assert!(ini.contains("server_domain=revival.example.com"));
        assert!(ini.contains("server_port=2000"));
        assert!(ini.contains("use_internal_server=false"));
    }
}
