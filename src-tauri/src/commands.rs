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
    Command::new(&cfg.game_exe)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch {}: {}", cfg.game_exe, e))
}
