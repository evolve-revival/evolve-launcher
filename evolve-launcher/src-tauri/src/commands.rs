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

    let active = cfg.active_version();
    if active.install_dir.is_empty() {
        return InstallStatus {
            state: "not-installed".to_string(),
            install_dir: active.install_dir.clone(),
            installed_build: None,
        };
    }

    let install_dir = PathBuf::from(&active.install_dir);

    if install_dir.join("progress.json").exists() {
        return InstallStatus {
            state: "paused".to_string(),
            install_dir: active.install_dir.clone(),
            installed_build: None,
        };
    }

    match InstallRecord::load(&install_dir) {
        None => InstallStatus {
            state: "not-installed".to_string(),
            install_dir: active.install_dir.clone(),
            installed_build: None,
        },
        Some(record) => InstallStatus {
            state: "ready".to_string(),
            install_dir: active.install_dir.clone(),
            installed_build: Some(record.build),
        },
    }
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<bool, String> {
    let cfg = Config::load(&app);
    if cfg.active_version().install_dir.is_empty() {
        return Ok(false);
    }
    let install_dir = PathBuf::from(&cfg.active_version().install_dir);
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

    let selected = &cfg.active_version().selected_components.clone();
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
    if let Some(v) = cfg.active_version_mut() {
        v.selected_components = Some(selected);
    }
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
            selected: cfg.active_version().selected_tier.as_deref() == Some(t.id.as_str()),
        })
        .collect())
}

#[tauri::command]
pub fn save_tier(app: AppHandle, tier_id: String, components: Vec<String>) -> Result<(), String> {
    let mut cfg = Config::load(&app);
    if let Some(v) = cfg.active_version_mut() {
        v.selected_tier = Some(tier_id);
        v.selected_components = Some(components);
    }
    cfg.save(&app)
}

// ── Install ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_install(
    app: AppHandle,
    install_dir: String,
    state: State<'_, AppDownloadState>,
) -> Result<(), String> {
    let mut cfg = Config::load(&app);
    if let Some(v) = cfg.active_version_mut() {
        v.install_dir = install_dir.clone();
    }
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
        let server_url = cfg_now.server_url.clone();
        let selected_files = manifest.filter_by_selection(&cfg_now.active_version().selected_components);

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

        if let Some(ref tier_id) = cfg_now.active_version().selected_tier.clone() {
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
    let install_dir = cfg.active_version().install_dir.clone();
    if install_dir.is_empty() {
        return Err("No install directory configured".to_string());
    }
    start_install(app, install_dir, state).await
}

// ── Repair ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_repair(
    app: AppHandle,
    state: State<'_, AppDownloadState>,
) -> Result<(), String> {
    let cfg = Config::load(&app);
    let repair_dir = cfg.active_version().install_dir.clone();
    if repair_dir.is_empty() {
        return Err("No install directory configured".to_string());
    }

    let cancelled = {
        let ds = state.0.lock().unwrap();
        ds.reset();
        Arc::clone(&ds.cancelled)
    };

    let dir = PathBuf::from(&repair_dir);
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

        ProgressRecord::delete(&dir);

        let cfg_now = Config::load(&app_clone);
        let selected_files = manifest.filter_by_selection(&cfg_now.active_version().selected_components);

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

        if let Some(ref tier_id) = cfg_now.active_version().selected_tier.clone() {
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
    let install_dir = cfg.active_version().install_dir.clone();
    if install_dir.is_empty() {
        return Err("No install directory configured".to_string());
    }
    start_install(app, install_dir, state).await
}

// ── Steam integration ─────────────────────────────────────────────────────

#[tauri::command]
pub fn list_steam_accounts() -> Result<Vec<crate::steam::SteamAccount>, String> {
    let root = crate::steam::find_steam_root()
        .ok_or_else(|| "Could not find Steam installation".to_string())?;
    Ok(crate::steam::list_accounts(&root))
}

#[tauri::command]
pub fn add_to_steam(app: AppHandle, steam_id: String) -> Result<(), String> {
    let cfg = Config::load(&app);
    if cfg.active_version().install_dir.is_empty() {
        return Err("No install directory configured".to_string());
    }
    let root = crate::steam::find_steam_root()
        .ok_or_else(|| "Could not find Steam installation".to_string())?;
    let launcher_exe = std::env::current_exe()
        .map_err(|e| format!("Could not resolve launcher path: {e}"))?;
    crate::steam::add_to_steam(&root, &steam_id, &launcher_exe)
}

// ── Donor / SDR setup ────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct DonorStatus {
    pub installed: bool,
    pub dll_ready: bool,
    pub donor_name: String,
    pub donor_app_id: u32,
}

#[tauri::command]
pub fn check_donor_game(app: AppHandle) -> DonorStatus {
    let cfg = Config::load(&app);
    let donor_name = format!("{} (App ID {})", crate::donor::DONOR_NAME, crate::donor::DONOR_APP_ID);

    let steam_root = match crate::steam::find_steam_root() {
        Some(r) => r,
        None => return DonorStatus {
            installed: false,
            dll_ready: false,
            donor_name,
            donor_app_id: crate::donor::DONOR_APP_ID,
        },
    };

    let donor_dir = crate::steam::find_donor_game_dir(&steam_root, crate::donor::DONOR_APP_ID);
    let installed = donor_dir.is_some();

    let install_dir = &cfg.active_version().install_dir;
    let dll_ready = if install_dir.is_empty() {
        false
    } else {
        PathBuf::from(install_dir)
            .join("bin64_SteamRetail")
            .join(crate::donor::REAL_STEAM_API_DLL)
            .exists()
    };

    DonorStatus {
        installed,
        dll_ready,
        donor_name,
        donor_app_id: crate::donor::DONOR_APP_ID,
    }
}

// ── Version management ────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct VersionInfo {
    pub id: String,
    pub name: String,
    pub install_dir: String,
    pub state: String,
    pub installed_build: Option<u64>,
    pub is_active: bool,
}

#[tauri::command]
pub fn get_versions(app: AppHandle) -> Vec<VersionInfo> {
    let cfg = Config::load(&app);
    cfg.versions.iter().map(|v| {
        let (state, installed_build) = if v.install_dir.is_empty() {
            ("not-installed".to_string(), None)
        } else {
            let dir = PathBuf::from(&v.install_dir);
            if dir.join("progress.json").exists() {
                ("paused".to_string(), None)
            } else {
                match crate::install::InstallRecord::load(&dir) {
                    Some(r) => ("ready".to_string(), Some(r.build)),
                    None => ("not-installed".to_string(), None),
                }
            }
        };
        VersionInfo {
            is_active: v.id == cfg.active_version_id,
            id: v.id.clone(),
            name: v.name.clone(),
            install_dir: v.install_dir.clone(),
            state,
            installed_build,
        }
    }).collect()
}

#[tauri::command]
pub fn switch_version(app: AppHandle, id: String) -> Result<InstallStatus, String> {
    let mut cfg = Config::load(&app);
    if !cfg.versions.iter().any(|v| v.id == id) {
        return Err(format!("Unknown version: {id}"));
    }
    cfg.active_version_id = id;
    cfg.save(&app)?;
    Ok(check_install_state(app))
}

#[tauri::command]
pub fn open_steam_store(app_id: u32) {
    let url = format!("steam://store/{app_id}");
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/c", "start", "", &url]).spawn();
    #[cfg(not(target_os = "windows"))]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
}

// ── Launch ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn launch_game(app: AppHandle, server_state: tauri::State<'_, crate::LocalServerState>) -> Result<(), String> {
    let _ = std::fs::write("/tmp/evolve_launch.log", "launch_game: started\n");
    let cfg = Config::load(&app);
    let game_install_dir = cfg.active_version().install_dir.clone();
    if game_install_dir.is_empty() {
        return Err("Game is not installed".to_string());
    }

    let bin_dir = PathBuf::from(&game_install_dir).join("bin64_SteamRetail");
    let exe = bin_dir.join("Evolve.exe");
    let _ = std::fs::write("/tmp/evolve_launch.log", format!("step1: install_dir={game_install_dir}, exe={exe:?}\n"));

    // Pre-flight 1: ensure steam_api64_real.dll is present (copy if missing)
    let real_dll = bin_dir.join(crate::donor::REAL_STEAM_API_DLL);
    if !real_dll.exists() {
        let steam_root = crate::steam::find_steam_root()
            .ok_or_else(|| "Steam not found — install Steam to play".to_string())?;
        let donor_dir = crate::steam::find_donor_game_dir(&steam_root, crate::donor::DONOR_APP_ID)
            .ok_or_else(|| format!(
                "{} (App ID {}) is not installed — install it via steam://run/{}",
                crate::donor::DONOR_NAME,
                crate::donor::DONOR_APP_ID,
                crate::donor::DONOR_APP_ID,
            ))?;
        crate::steam::copy_steam_api_dll(&donor_dir, &bin_dir)?;
    }

    let _ = std::fs::write("/tmp/evolve_launch.log", "step2: dll check ok\n");

    // Pre-flight 2: remove precache.m2k so kando can't use Pinenut's stale service
    // config. precache epochs (~1781M) always beat our cache.m2k epochs (~1448M),
    // so every post-auth call (checkAppOwnership, grants, etc.) gets served from
    // Pinenut's bundled cache instead of hitting our live server. Without precache,
    // kando falls through to cache.m2k (our live doorman response) and makes fresh
    // requests we can actually answer.
    let precache_path = PathBuf::from(&game_install_dir).join("precache.m2k");
    if precache_path.exists() {
        let _ = std::fs::remove_file(&precache_path);
        let _ = std::fs::write("/tmp/evolve_launch.log", "step2a: removed precache.m2k\n");
    }

    // Pre-flight 3: ensure iptables redirects 127.0.0.1:443 → 4430 on Linux.
    // The kando client always dials port 443; our server binds 4430 to stay
    // unprivileged. The rule is idempotent (-C checks before -A adds).
    #[cfg(target_os = "linux")]
    {
        let rule = ["-t", "nat", "-A", "OUTPUT", "-o", "lo", "-p", "tcp",
                    "--dport", "443", "-d", "127.0.0.1", "-j", "REDIRECT", "--to-port", "4430"];
        let check = ["-t", "nat", "-C", "OUTPUT", "-o", "lo", "-p", "tcp",
                     "--dport", "443", "-d", "127.0.0.1", "-j", "REDIRECT", "--to-port", "4430"];
        let already = std::process::Command::new("sudo")
            .arg("iptables")
            .args(check)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !already {
            let ok = std::process::Command::new("sudo")
                .arg("iptables")
                .args(rule)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !ok {
                return Err(
                    "Could not set up port redirect (iptables 443→4430). \
                     Run once manually: sudo iptables -t nat -A OUTPUT -o lo -p tcp \
                     --dport 443 -d 127.0.0.1 -j REDIRECT --to-port 4430".to_string()
                );
            }
        }
        let _ = std::fs::write("/tmp/evolve_launch.log", "step2b: iptables redirect ok\n");
    }

    // Pre-flight 3: rewrite EvolveLogging.ini — always use 127.0.0.1 so pinenut's
    // GetAddrInfoW hook redirects *.my.2k.com to our local server, not the configured
    // external server_url (which may resolve to a remote IP, breaking cURL connections).
    let ini = crate::patcher::generate_logging_ini("https://127.0.0.1:443");
    std::fs::write(bin_dir.join("EvolveLogging.ini"), ini)
        .map_err(|e| format!("Failed to write EvolveLogging.ini: {e}"))?;

    // Pre-flight 3: ensure Steam is running (required for real Steamworks)
    #[cfg(target_os = "windows")]
    {
        let steam_running = std::process::Command::new("tasklist")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("steam.exe"))
            .unwrap_or(false);
        if !steam_running {
            return Err("Steam is not running — please start Steam and log in first".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::fs::write("/tmp/evolve_launch.log", "step3: checking steam\n");
        let steam_running = std::process::Command::new("pgrep")
            .arg("-x")
            .arg("steam")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let _ = std::fs::write("/tmp/evolve_launch.log", format!("step3: steam_running={steam_running}\n"));
        if !steam_running {
            return Err("Steam is not running — please start Steam and log in first".to_string());
        }
    }

    #[cfg(target_os = "windows")]
    {
        let cwd = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
        std::process::Command::new(&exe)
            .current_dir(cwd)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to launch Evolve: {e}"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let steam_root = crate::steam::find_steam_root().ok_or_else(|| {
            "Steam not found. Install Steam and Proton Experimental to play on Linux.".to_string()
        })?;
        let proton = crate::steam::find_proton(&steam_root).ok_or_else(|| {
            "Proton not found. Open Steam → Tools and install Proton Experimental.".to_string()
        })?;
        let compat_prefix = PathBuf::from(&game_install_dir).join("proton_prefix");
        std::fs::create_dir_all(&compat_prefix).map_err(|e| e.to_string())?;
        let cwd = exe.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf();

        // Start server only if not already running (it lives for the app's lifetime)
        let server_already_running = {
            let guard = server_state.0.lock().unwrap();
            guard.is_some()
        };
        if !server_already_running {
            let _ = std::fs::write("/tmp/evolve_launch.log", format!("step4: starting local server, game_root={game_install_dir}\n"));
            let server = crate::local_server::start(std::path::Path::new(&game_install_dir)).await
                .map_err(|e| { let _ = std::fs::write("/tmp/evolve_launch.log", format!("step4 FAILED: {e}\n")); e })?;
            let _ = std::fs::write("/tmp/evolve_launch.log", "step5: server ok, spawning proton\n");
            let mut guard = server_state.0.lock().unwrap();
            *guard = Some(server);
        } else {
            let _ = std::fs::write("/tmp/evolve_launch.log", "step4+5: server already running\n");
        }

        let mut child = tokio::process::Command::new(&proton)
            .arg("run")
            .arg(&exe)
            .env("STEAM_COMPAT_DATA_PATH", &compat_prefix)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &steam_root)
            .env("SteamAppId", crate::donor::DONOR_APP_ID.to_string())
            .current_dir(&cwd)
            .spawn()
            .map_err(|e| format!("Failed to launch via Proton ({proton:?}): {e}"))?;

        app.emit("game-launched", ()).ok();

        tokio::spawn(async move {
            let _ = child.wait().await;
            app.emit("game-exited", ()).ok();
        });

        Ok(())
    }
}
