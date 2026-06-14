mod commands;
mod config;
mod donor;
mod downloader;
mod install;
mod nat;
mod patcher;
mod steam;

use commands::AppDownloadState;
use downloader::DownloadState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppDownloadState(Mutex::new(DownloadState::default())))
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::check_install_state,
            commands::check_for_updates,
            commands::get_components,
            commands::save_components,
            commands::get_tiers,
            commands::save_tier,
            commands::start_install,
            commands::pause_install,
            commands::resume_install,
            commands::start_repair,
            commands::start_update,
            commands::launch_game,
            commands::list_steam_accounts,
            commands::add_to_steam,
            commands::check_donor_game,
            commands::open_steam_store,
            commands::get_versions,
            commands::switch_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
