use crate::install::{Manifest, ManifestFile, ProgressRecord};
use futures::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub const MANIFEST_URL: &str = "https://cdn.evolve-revival.com/manifest.json";
const MAX_CONCURRENT: usize = 4;
const MAX_RETRIES: u32 = 3;

// ── SHA-256 verification ──────────────────────────────────────────────────

pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let actual = hex::encode(result);
    actual == expected_hex.to_lowercase()
}

// ── Progress event ────────────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub current_file: String,
    pub speed_bps: u64,
    pub eta_secs: u64,
}

// ── Download state (stored in Tauri managed state) ────────────────────────

pub struct DownloadState {
    pub cancelled: Arc<AtomicBool>,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl DownloadState {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

// ── Manifest fetch ────────────────────────────────────────────────────────

pub async fn fetch_manifest(client: &Client) -> Result<Manifest, String> {
    let bytes = client
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Manifest server error: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read manifest body: {}", e))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Failed to parse manifest: {}", e))
}

// ── Single file download ──────────────────────────────────────────────────

pub async fn download_file(
    client: &Client,
    url: &str,
    dest: &Path,
    expected_sha256: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(), String> {
    // Fix 5: append .tmp rather than replacing extension to avoid collisions
    let tmp = PathBuf::from(format!("{}.tmp", dest.display()));
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed for {}: {}", url, e))?
        .error_for_status()
        .map_err(|e| format!("Server error for {}: {}", url, e))?;

    let mut file = fs::File::create(&tmp)
        .await
        .map_err(|e| format!("Failed to create tmp file: {}", e))?;

    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();

    // Fix 3: check cancelled inside the streaming loop for fast mid-stream pause
    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::Relaxed) {
            drop(file);
            let _ = fs::remove_file(&tmp).await;
            return Err("paused".to_string());
        }
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
    }
    file.flush().await.map_err(|e| format!("Flush error: {}", e))?;
    drop(file);

    let actual = hex::encode(hasher.finalize());
    if actual != expected_sha256.to_lowercase() {
        let _ = fs::remove_file(&tmp).await;
        return Err(format!(
            "SHA-256 mismatch for {}: expected {} got {}",
            url, expected_sha256, actual
        ));
    }

    fs::rename(&tmp, dest)
        .await
        .map_err(|e| format!("Failed to rename tmp file: {}", e))?;

    Ok(())
}

// ── Download with retry ───────────────────────────────────────────────────

pub async fn download_with_retry(
    client: &Client,
    url: &str,
    dest: &Path,
    sha256: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(), String> {
    let mut last_err = String::new();
    for attempt in 1..=MAX_RETRIES {
        match download_file(client, url, dest, sha256, cancelled.clone()).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = e;
                // Fix 4: don't retry on SHA-256 mismatch (wrong content from server)
                // Fix 3: don't retry on pause/cancel
                if last_err == "paused"
                    || last_err.contains("SHA-256 mismatch")
                    || attempt == MAX_RETRIES
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(attempt as u64)).await;
            }
        }
    }
    Err(format!("Failed after {} retries: {}", MAX_RETRIES, last_err))
}

// ── Parallel download orchestration ──────────────────────────────────────

pub async fn run_downloads(
    app: AppHandle,
    client: Client,
    files: Vec<ManifestFile>,
    base_url: String,
    install_dir: PathBuf,
    cancelled: Arc<AtomicBool>,
) -> Result<(), String> {
    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    let progress = ProgressRecord::load(&install_dir);

    let already_done: u64 = files
        .iter()
        .filter(|f| progress.is_complete(&f.path))
        .map(|f| f.size)
        .sum();

    let downloaded = Arc::new(AtomicU64::new(already_done));
    let pending: Vec<ManifestFile> = files
        .into_iter()
        .filter(|f| !progress.is_complete(&f.path))
        .collect();

    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT));
    // Fix 2: use JoinSet so we can abort_all() on error instead of detaching tasks
    let mut join_set = tokio::task::JoinSet::new();
    let start = Instant::now();

    for file in pending {
        if cancelled.load(Ordering::SeqCst) {
            return Err("paused".to_string());
        }

        // Guard against path traversal attacks via a compromised manifest
        if std::path::Path::new(&file.path).is_absolute() {
            return Err(format!("Manifest contains absolute path: {}", file.path));
        }
        let dest_check = install_dir.join(&file.path);
        if !dest_check.starts_with(&install_dir) {
            return Err(format!("Manifest path escapes install dir: {}", file.path));
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = format!("{}{}", base_url, file.path);
        let dest = install_dir.join(&file.path);
        let sha256 = file.sha256.clone();
        let size = file.size;
        let path = file.path.clone();
        let downloaded = downloaded.clone();
        let app = app.clone();
        let total_bytes = total_bytes;
        let start = start;
        let cancelled = cancelled.clone();

        // Fix 1: task returns the path string so the orchestrator can call mark_complete
        join_set.spawn(async move {
            let _permit = permit;
            if cancelled.load(Ordering::SeqCst) {
                return Err("paused".to_string());
            }

            download_with_retry(&client, &url, &dest, &sha256, cancelled).await?;

            let done = downloaded.fetch_add(size, Ordering::SeqCst) + size;
            let elapsed = start.elapsed().as_secs_f64().max(0.001);
            let speed = (done as f64 / elapsed) as u64;
            let remaining = total_bytes.saturating_sub(done);
            let eta = if speed > 0 { remaining / speed } else { 0 };

            let _ = app.emit(
                "download-progress",
                DownloadProgress {
                    downloaded_bytes: done,
                    total_bytes,
                    current_file: path.clone(),
                    speed_bps: speed,
                    eta_secs: eta,
                },
            );

            // Return path for the orchestrator to mark_complete serially (Fix 1)
            Ok::<String, String>(path)
        });
    }

    // Fix 1 + Fix 2: load progress once, mark_complete serially as tasks finish.
    // join_next() returns tasks as they complete (out of order), which is ideal.
    let mut progress = ProgressRecord::load(&install_dir);
    while let Some(result) = join_set.join_next().await {
        match result {
            Err(e) => {
                // JoinError (task panicked or was aborted)
                join_set.abort_all();
                return Err(e.to_string());
            }
            Ok(Err(e)) => {
                // Task returned an application error
                join_set.abort_all();
                return Err(e);
            }
            Ok(Ok(completed_path)) => {
                progress.mark_complete(&completed_path, &install_dir)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_sha256_accepts_correct_hash() {
        // SHA-256 of b"hello world" — verified with: echo -n "hello world" | sha256sum
        let bytes = b"hello world";
        let hex = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_sha256(bytes, hex));
    }

    #[test]
    fn verify_sha256_rejects_wrong_hash() {
        let bytes = b"hello world";
        assert!(!verify_sha256(
            bytes,
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }
}
