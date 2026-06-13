use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

// ── Manifest ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_enabled: bool,
    pub size_bytes: u64,
}

fn default_component_id() -> String {
    "core".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManifestFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default = "default_component_id")]
    pub component: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub version: String,
    pub build: u64,
    pub base_url: String,
    #[serde(default)]
    pub components: Vec<Component>,
    pub files: Vec<ManifestFile>,
    pub patches: Vec<ManifestFile>,
}

impl Manifest {
    /// Return files that should be downloaded given a component selection.
    /// - If the manifest has no component definitions, returns all files.
    /// - Required components are always included regardless of `selected`.
    /// - `selected = None` means "not yet customized" → use `default_enabled` per component.
    pub fn filter_by_selection(&self, selected: &Option<Vec<String>>) -> Vec<ManifestFile> {
        if self.components.is_empty() {
            return self.files.clone();
        }

        let required: HashSet<&str> = self
            .components
            .iter()
            .filter(|c| c.required)
            .map(|c| c.id.as_str())
            .collect();

        let enabled: HashSet<&str> = match selected {
            None => self
                .components
                .iter()
                .filter(|c| c.required || c.default_enabled)
                .map(|c| c.id.as_str())
                .collect(),
            Some(ids) => ids
                .iter()
                .map(|s| s.as_str())
                .chain(required.iter().copied())
                .collect(),
        };

        self.files
            .iter()
            .filter(|f| enabled.contains(f.component.as_str()))
            .cloned()
            .collect()
    }

    /// Compute total selected bytes (for display before download starts).
    pub fn selected_bytes(&self, selected: &Option<Vec<String>>) -> u64 {
        if self.components.is_empty() {
            return self.files.iter().map(|f| f.size).sum();
        }
        let files = self.filter_by_selection(selected);
        files.iter().map(|f| f.size).sum()
    }
}

// ── Install record ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallRecord {
    pub version: String,
    pub build: u64,
}

impl InstallRecord {
    pub fn load(install_dir: &Path) -> Option<Self> {
        fs::read_to_string(install_dir.join("install.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn save(&self, install_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(install_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(install_dir.join("install.json"), json).map_err(|e| e.to_string())
    }
}

// ── Progress record ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProgressRecord {
    pub completed: HashSet<String>,
}

impl ProgressRecord {
    pub fn load(install_dir: &Path) -> Self {
        fs::read_to_string(install_dir.join("progress.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, install_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(install_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(install_dir.join("progress.json"), json).map_err(|e| e.to_string())
    }

    pub fn mark_complete(&mut self, path: &str, install_dir: &Path) -> Result<(), String> {
        if self.completed.insert(path.to_string()) {
            self.save(install_dir)?;
        }
        Ok(())
    }

    pub fn is_complete(&self, path: &str) -> bool {
        self.completed.contains(path)
    }

    pub fn delete(install_dir: &Path) {
        let _ = fs::remove_file(install_dir.join("progress.json"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn install_record_round_trips() {
        let dir = tempdir().unwrap();
        let record = InstallRecord {
            version: "1.0.0".to_string(),
            build: 1,
        };
        record.save(dir.path()).unwrap();
        let loaded = InstallRecord::load(dir.path()).unwrap();
        assert_eq!(loaded.version, "1.0.0");
        assert_eq!(loaded.build, 1);
    }

    #[test]
    fn progress_record_tracks_completed() {
        let dir = tempdir().unwrap();
        let mut progress = ProgressRecord::default();
        assert!(!progress.is_complete("Game/UI_Assets.pak"));
        progress.mark_complete("Game/UI_Assets.pak", dir.path()).unwrap();
        assert!(progress.is_complete("Game/UI_Assets.pak"));
        assert!(!progress.is_complete("Engine/Engine.pak"));

        // Reload from disk
        let reloaded = ProgressRecord::load(dir.path());
        assert!(reloaded.is_complete("Game/UI_Assets.pak"));
    }

    #[test]
    fn manifest_parses_from_json() {
        let json = r#"{
            "version": "1.0.0",
            "build": 1,
            "base_url": "https://cdn.example.com/files/1.0.0/",
            "files": [
                { "path": "Game/UI_Assets.pak", "size": 45000000, "sha256": "abc123" }
            ],
            "patches": [
                { "path": "bin64_SteamRetail/dbghelp.dll", "size": 117417, "sha256": "def456" }
            ]
        }"#;
        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.patches.len(), 1);
        assert_eq!(manifest.files[0].path, "Game/UI_Assets.pak");
    }
}
