use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

fn default_server_url() -> String {
    "https://evolve.navitank.org".to_string()
}

fn default_active_version() -> String {
    "evolve".to_string()
}

fn default_versions() -> Vec<VersionProfile> {
    vec![VersionProfile::new("evolve", "Evolve")]
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub install_dir: String,
    #[serde(default)]
    pub selected_tier: Option<String>,
    #[serde(default)]
    pub selected_components: Option<Vec<String>>,
}

impl VersionProfile {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            install_dir: String::new(),
            selected_tier: None,
            selected_components: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default = "default_active_version")]
    pub active_version_id: String,
    #[serde(default = "default_versions")]
    pub versions: Vec<VersionProfile>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            active_version_id: default_active_version(),
            versions: default_versions(),
        }
    }
}

impl Config {
    /// Returns a reference to the active version profile.
    /// Falls back to the first version if active_version_id is not found.
    pub fn active_version(&self) -> &VersionProfile {
        self.versions
            .iter()
            .find(|v| v.id == self.active_version_id)
            .or_else(|| self.versions.first())
            .expect("Config has no versions")
    }

    /// Returns a mutable reference to the active version profile.
    pub fn active_version_mut(&mut self) -> Option<&mut VersionProfile> {
        let id = self.active_version_id.clone();
        self.versions.iter_mut().find(|v| v.id == id)
    }

    pub fn load(app: &tauri::AppHandle) -> Self {
        let Ok(dir) = app.path().app_data_dir() else {
            return Self::default();
        };
        let content = match fs::read_to_string(dir.join("config.json")) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };

        // Try new format first (has non-empty `versions` array)
        if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
            if !cfg.versions.is_empty() {
                return cfg;
            }
        }

        // Migrate old flat format → wrap into default version profile
        #[derive(Deserialize, Default)]
        struct LegacyConfig {
            #[serde(default)]
            install_dir: String,
            #[serde(default = "default_server_url")]
            server_url: String,
            #[serde(default)]
            selected_tier: Option<String>,
            #[serde(default)]
            selected_components: Option<Vec<String>>,
        }
        let legacy: LegacyConfig = serde_json::from_str(&content).unwrap_or_default();
        let mut profile = VersionProfile::new("evolve", "Evolve");
        profile.install_dir = legacy.install_dir;
        profile.selected_tier = legacy.selected_tier;
        profile.selected_components = legacy.selected_components;

        Config {
            server_url: legacy.server_url,
            active_version_id: "evolve".to_string(),
            versions: vec![profile],
        }
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
    fn default_config_has_evolve_version() {
        let cfg = Config::default();
        assert_eq!(cfg.active_version_id, "evolve");
        assert_eq!(cfg.versions.len(), 1);
        assert_eq!(cfg.versions[0].id, "evolve");
        assert_eq!(cfg.versions[0].name, "Evolve");
        assert_eq!(cfg.active_version().install_dir, "");
    }

    #[test]
    fn default_config_has_default_server_url() {
        let cfg = Config::default();
        assert_eq!(cfg.server_url, "https://evolve.navitank.org");
    }

    #[test]
    fn config_round_trips_through_json() {
        let mut cfg = Config::default();
        cfg.server_url = "http://example.com:8080".to_string();
        if let Some(v) = cfg.active_version_mut() {
            v.install_dir = "/home/user/Games/Evolve".to_string();
            v.selected_tier = Some("recommended".to_string());
        }
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_url, "http://example.com:8080");
        assert_eq!(back.active_version().install_dir, "/home/user/Games/Evolve");
        assert_eq!(
            back.active_version().selected_tier,
            Some("recommended".to_string())
        );
    }

    #[test]
    fn missing_fields_use_defaults_on_deserialize() {
        let cfg: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg.server_url, "https://evolve.navitank.org");
        assert_eq!(cfg.active_version_id, "evolve");
    }

    #[test]
    fn new_format_with_versions_array_deserializes() {
        let json = r#"{
            "server_url": "http://myserver:8080",
            "active_version_id": "evolve",
            "versions": [
                {"id": "evolve", "name": "Evolve", "install_dir": "/games/evolve"}
            ]
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.active_version().install_dir, "/games/evolve");
        assert_eq!(cfg.server_url, "http://myserver:8080");
    }

    #[test]
    fn active_version_mut_updates_correct_profile() {
        let mut cfg = Config::default();
        if let Some(v) = cfg.active_version_mut() {
            v.install_dir = "/new/path".to_string();
        }
        assert_eq!(cfg.active_version().install_dir, "/new/path");
    }
}
