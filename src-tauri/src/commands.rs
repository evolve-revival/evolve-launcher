use crate::config::Config;
use crate::downloader::{fetch_manifest, run_downloads, DownloadState};
use crate::install::{apply_perf_config, Component, InstallRecord, ProgressRecord, Tier};
use crate::patcher::apply_patches;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

pub struct AppDownloadState(pub Mutex<DownloadState>);

// ── Config commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(app: AppHandle) -> Config {
    Config::load(&app)
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: Config) -> Result<(), String> {
    // Load first so install_dir (and any other fields not sent by the frontend) are preserved
    let mut existing = Config::load(&app);
    existing.server_url = config.server_url;
    existing.save(&app)
}

// ── Install state ─────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct InstallStatus {
    pub state: String,
    pub install_dir: String,
    pub installed_build: Option<u64>,
}

#[tauri::command]
pub fn check_install_state(app: AppHandle) -> InstallStatus {
    let cfg = Config::load(&app);

    if cfg.install_dir.is_empty() {
        return InstallStatus {
            state: "not-installed".to_string(),
            install_dir: cfg.install_dir,
            installed_build: None,
        };
    }

    let install_dir = PathBuf::from(&cfg.install_dir);

    // If progress.json exists, install was interrupted
    if install_dir.join("progress.json").exists() {
        return InstallStatus {
            state: "paused".to_string(),
            install_dir: cfg.install_dir,
            installed_build: None,
        };
    }

    match InstallRecord::load(&install_dir) {
        None => InstallStatus {
            state: "not-installed".to_string(),
            install_dir: cfg.install_dir,
            installed_build: None,
        },
        Some(record) => InstallStatus {
            state: "ready".to_string(),
            install_dir: cfg.install_dir,
            installed_build: Some(record.build),
        },
    }
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<bool, String> {
    let cfg = Config::load(&app);
    if cfg.install_dir.is_empty() {
        return Ok(false);
    }
    let install_dir = PathBuf::from(&cfg.install_dir);
    let record = match InstallRecord::load(&install_dir) {
        None => return Ok(false),
        Some(r) => r,
    };
    let client = Client::new();
    let manifest = fetch_manifest(&client).await?;
    Ok(manifest.build > record.build)
}

// ── Component selection ───────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ComponentState {
    pub id: String,
    pub name: String,
    pub description: String,
    pub required: bool,
    pub enabled: bool,
    pub size_bytes: u64,
}

impl ComponentState {
    fn from_component(c: &Component, enabled: bool) -> Self {
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            description: c.description.clone(),
            required: c.required,
            enabled,
            size_bytes: c.size_bytes,
        }
    }
}

#[tauri::command]
pub async fn get_components(app: AppHandle) -> Result<Vec<ComponentState>, String> {
    let cfg = Config::load(&app);
    let client = Client::new();
    let manifest = fetch_manifest(&client).await?;

    if manifest.components.is_empty() {
        return Ok(vec![]);
    }

    let selected = &cfg.selected_components;
    let result = manifest
        .components
        .iter()
        .map(|c| {
            let enabled = match selected {
                None => c.required || c.default_enabled,
                Some(ids) => c.required || ids.contains(&c.id),
            };
            ComponentState::from_component(c, enabled)
        })
        .collect();
    Ok(result)
}

#[tauri::command]
pub fn save_components(app: AppHandle, selected: Vec<String>) -> Result<(), String> {
    let mut cfg = Config::load(&app);
    cfg.selected_components = Some(selected);
    cfg.save(&app)
}

// ── Tier selection ────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct TierState {
    pub id: String,
    pub name: String,
    pub description: String,
    pub components: Vec<String>,
    pub size_bytes: u64,
    pub recommended: bool,
    pub selected: bool,
}

#[tauri::command]
pub async fn get_tiers(app: AppHandle) -> Result<Vec<TierState>, String> {
    let cfg = Config::load(&app);
    let client = Client::new();
    let manifest = fetch_manifest(&client).await?;

    Ok(manifest
        .tiers
        .iter()
        .map(|t: &Tier| TierState {
            id: t.id.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            components: t.components.clone(),
            size_bytes: manifest.tier_size(t),
            recommended: t.recommended,
            selected: cfg.selected_tier.as_deref() == Some(t.id.as_str()),
        })
        .collect())
}

#[tauri::command]
pub fn save_tier(app: AppHandle, tier_id: String, components: Vec<String>) -> Result<(), String> {
    let mut cfg = Config::load(&app);
    cfg.selected_tier = Some(tier_id);
    cfg.selected_components = Some(components);
    cfg.save(&app)
}

// ── Install ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_install(
    app: AppHandle,
    install_dir: String,
    state: State<'_, AppDownloadState>,
) -> Result<(), String> {
    // Save install_dir to config immediately
    let mut cfg = Config::load(&app);
    cfg.install_dir = install_dir.clone();
    cfg.save(&app)?;

    let cancelled = {
        let ds = state.0.lock().unwrap();
        ds.reset();
        Arc::clone(&ds.cancelled)
    };

    let dir = PathBuf::from(&install_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let app_clone = app.clone();
    tokio::spawn(async move {
        let client = Client::new();
        let manifest = match fetch_manifest(&client).await {
            Ok(m) => m,
            Err(e) => {
                let _ = app_clone.emit("install-error", e);
                return;
            }
        };

        let cfg_now = Config::load(&app_clone);
        let server_url = cfg_now.server_url;
        let selected_files = manifest.filter_by_selection(&cfg_now.selected_components);

        match run_downloads(
            app_clone.clone(),
            client.clone(),
            selected_files,
            manifest.base_url.clone(),
            dir.clone(),
            cancelled.clone(),
        )
        .await
        {
            Err(e) if e == "paused" => {
                let _ = app_clone.emit("install-paused", ());
                return;
            }
            Err(e) => {
                let _ = app_clone.emit("install-error", e);
                return;
            }
            Ok(_) => {}
        }

        if let Err(e) = apply_patches(&client, &manifest, &dir, &server_url, cancelled).await {
            let _ = app_clone.emit("install-error", e);
            return;
        }

        if let Some(ref tier_id) = cfg_now.selected_tier {
            if let Some(tier) = manifest.tiers.iter().find(|t| t.id == *tier_id) {
                if let Err(e) = apply_perf_config(&dir, &tier.perf_config) {
                    eprintln!("Warning: could not apply tier perf config: {}", e);
                }
            }
        }

        let record = InstallRecord {
            version: manifest.version.clone(),
            build: manifest.build,
        };
        if let Err(e) = record.save(&dir) {
            let _ = app_clone.emit("install-error", e);
            return;
        }

        ProgressRecord::delete(&dir);
        let _ = app_clone.emit("install-complete", ());
    });

    Ok(())
}

#[tauri::command]
pub fn pause_install(state: State<'_, AppDownloadState>) {
    state.0.lock().unwrap().cancel();
}

#[tauri::command]
pub async fn resume_install(
    app: AppHandle,
    state: State<'_, AppDownloadState>,
) -> Result<(), String> {
    let cfg = Config::load(&app);
    if cfg.install_dir.is_empty() {
        return Err("No install directory configured".to_string());
    }
    start_install(app, cfg.install_dir, state).await
}

// ── Repair ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_repair(
    app: AppHandle,
    state: State<'_, AppDownloadState>,
) -> Result<(), String> {
    let cfg = Config::load(&app);
    if cfg.install_dir.is_empty() {
        return Err("No install directory configured".to_string());
    }

    let cancelled = {
        let ds = state.0.lock().unwrap();
        ds.reset();
        Arc::clone(&ds.cancelled)
    };

    let dir = PathBuf::from(&cfg.install_dir);
    let server_url = cfg.server_url.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        let client = Client::new();
        let manifest = match fetch_manifest(&client).await {
            Ok(m) => m,
            Err(e) => {
                let _ = app_clone.emit("install-error", e);
                return;
            }
        };

        // Repair: clear progress record so all files get re-downloaded
        ProgressRecord::delete(&dir);

        let cfg_now = Config::load(&app_clone);
        let selected_files = manifest.filter_by_selection(&cfg_now.selected_components);

        match run_downloads(
            app_clone.clone(),
            client.clone(),
            selected_files,
            manifest.base_url.clone(),
            dir.clone(),
            cancelled.clone(),
        )
        .await
        {
            Err(e) if e == "paused" => {
                let _ = app_clone.emit("install-paused", ());
                return;
            }
            Err(e) => {
                let _ = app_clone.emit("install-error", e);
                return;
            }
            Ok(_) => {}
        }

        if let Err(e) = apply_patches(&client, &manifest, &dir, &server_url, cancelled).await {
            let _ = app_clone.emit("install-error", e);
            return;
        }

        if let Some(ref tier_id) = cfg_now.selected_tier {
            if let Some(tier) = manifest.tiers.iter().find(|t| t.id == *tier_id) {
                if let Err(e) = apply_perf_config(&dir, &tier.perf_config) {
                    eprintln!("Warning: could not apply tier perf config: {}", e);
                }
            }
        }

        let record = InstallRecord {
            version: manifest.version.clone(),
            build: manifest.build,
        };
        if let Err(e) = record.save(&dir) {
            let _ = app_clone.emit("install-error", e);
            return;
        }

        ProgressRecord::delete(&dir);
        let _ = app_clone.emit("repair-complete", ());
    });

    Ok(())
}

// ── Update ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_update(
    app: AppHandle,
    state: State<'_, AppDownloadState>,
) -> Result<(), String> {
    let cfg = Config::load(&app);
    if cfg.install_dir.is_empty() {
        return Err("No install directory configured".to_string());
    }
    start_install(app, cfg.install_dir, state).await
}

// ── Launch ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn launch_game(app: AppHandle) -> Result<(), String> {
    let cfg = Config::load(&app);
    if cfg.install_dir.is_empty() {
        return Err("Game is not installed".to_string());
    }
    let exe = PathBuf::from(&cfg.install_dir).join("bin64_SteamRetail/Evolve.exe");
    let cwd = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
    std::process::Command::new(&exe)
        .current_dir(cwd)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch Evolve: {}", e))
}
