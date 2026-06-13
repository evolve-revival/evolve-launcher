use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub game_exe: String,
    #[serde(default = "default_server_url")]
    pub server_url: String,
}

fn default_server_url() -> String {
    "http://localhost:8080".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            game_exe: String::new(),
            server_url: default_server_url(),
        }
    }
}

impl Config {
    pub fn load(app: &tauri::AppHandle) -> Self {
        let Ok(dir) = app.path().app_data_dir() else {
            return Self::default();
        };
        fs::read_to_string(dir.join("config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(dir.join("config.json"), json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_empty_exe_and_default_url() {
        let cfg = Config::default();
        assert_eq!(cfg.game_exe, "");
        assert_eq!(cfg.server_url, "http://localhost:8080");
    }

    #[test]
    fn config_round_trips_through_json() {
        let cfg = Config {
            game_exe: "C:\\Evolve\\Evolve.exe".to_string(),
            server_url: "http://example.com:8080".to_string(),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.game_exe, cfg.game_exe);
        assert_eq!(back.server_url, cfg.server_url);
    }

    #[test]
    fn missing_fields_use_defaults_on_deserialize() {
        let json = r#"{}"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.game_exe, "");
        assert_eq!(cfg.server_url, "http://localhost:8080");
    }
}
