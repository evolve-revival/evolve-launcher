use std::process::Command;

const GAME_EXE: &str = "/home/navitank/Desktop/EvolveFilesLegacy/bin64_SteamRetail/Evolve.exe";

#[tauri::command]
pub fn get_config(app: tauri::AppHandle) -> crate::config::Config {
    crate::config::Config::load(&app)
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, config: crate::config::Config) -> Result<(), String> {
    config.save(&app)
}

#[tauri::command]
pub fn launch_game() -> Result<(), String> {
    let exe_path = std::path::Path::new(GAME_EXE);
    let cwd = exe_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    Command::new(exe_path)
        .current_dir(cwd)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch Evolve: {}", e))
}
