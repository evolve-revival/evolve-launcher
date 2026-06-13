use crate::config::Config;
use std::process::Command;

#[tauri::command]
pub fn get_config(app: tauri::AppHandle) -> Config {
    Config::load(&app)
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, config: Config) -> Result<(), String> {
    config.save(&app)
}

#[tauri::command]
pub fn launch_game(app: tauri::AppHandle) -> Result<(), String> {
    let cfg = Config::load(&app);
    if cfg.game_exe.is_empty() {
        return Err("Game executable path is not configured — open Settings first".to_string());
    }
    let exe_path = std::path::Path::new(&cfg.game_exe);
    let cwd = exe_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    Command::new(exe_path)
        .current_dir(cwd)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch {}: {}", cfg.game_exe, e))
}
