use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const APP_NAME: &str = "Evolve";

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamAccount {
    pub steam_id: String,
    pub persona_name: String,
    pub account_name: String,
}

// ── Steam root detection ─────────────────────────────────────────────────────

pub fn find_steam_root() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").ok()?;
        let candidates = [
            PathBuf::from(&home).join(".steam/steam"),
            PathBuf::from(&home).join(".local/share/Steam"),
        ];
        return candidates.into_iter().find(|p| p.is_dir());
    }

    #[cfg(target_os = "windows")]
    {
        let candidates = [
            std::env::var("PROGRAMFILES(X86)")
                .map(|p| PathBuf::from(p).join("Steam"))
                .unwrap_or_default(),
            std::env::var("PROGRAMFILES")
                .map(|p| PathBuf::from(p).join("Steam"))
                .unwrap_or_default(),
            PathBuf::from(r"C:\Program Files (x86)\Steam"),
        ];
        return candidates.into_iter().find(|p| p.is_dir());
    }

    #[allow(unreachable_code)]
    None
}

// ── Account listing ──────────────────────────────────────────────────────────

pub fn list_accounts(steam_root: &Path) -> Vec<SteamAccount> {
    let loginusers = steam_root.join("config/loginusers.vdf");
    let content = match fs::read_to_string(&loginusers) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    parse_loginusers(&content, steam_root)
}

fn parse_loginusers(content: &str, steam_root: &Path) -> Vec<SteamAccount> {
    let mut accounts = Vec::new();
    let mut current_id: Option<String> = None;
    let mut persona = String::new();
    let mut account_name = String::new();
    let mut depth: i32 = 0;

    for line in content.lines() {
        let t = line.trim();

        if t == "{" {
            depth += 1;
            continue;
        }
        if t == "}" {
            // End of a user block: save if we have an ID and a matching userdata dir.
            if depth == 2 {
                if let Some(id) = current_id.take() {
                    if steam_root.join("userdata").join(&id).is_dir() {
                        accounts.push(SteamAccount {
                            steam_id: id,
                            persona_name: std::mem::take(&mut persona),
                            account_name: std::mem::take(&mut account_name),
                        });
                    }
                }
                persona.clear();
                account_name.clear();
            }
            depth -= 1;
            continue;
        }

        match depth {
            1 => {
                // Lines at this depth are SteamID64 object keys: "76561198XXXXXXX"
                if let Some(id) = extract_sole_quoted(t) {
                    // SteamID64 is always 17 digits starting with 7656.
                    if id.len() == 17
                        && id.starts_with("7656")
                        && id.chars().all(|c| c.is_ascii_digit())
                    {
                        current_id = Some(id);
                        persona.clear();
                        account_name.clear();
                    }
                }
            }
            2 => {
                if let Some((k, v)) = parse_kv(t) {
                    match k.to_lowercase().as_str() {
                        "personaname" => persona = v,
                        "accountname" => account_name = v,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    accounts
}

// Returns the string inside quotes if the line is just a single quoted token.
fn extract_sole_quoted(s: &str) -> Option<String> {
    if !s.starts_with('"') {
        return None;
    }
    let inner = &s[1..];
    let end = inner.find('"')?;
    let key = &inner[..end];
    let rest = inner[end + 1..].trim();
    // "sole" means nothing follows the closing quote.
    if rest.is_empty() {
        Some(key.to_string())
    } else {
        None
    }
}

// Parses `"key"\t"value"` or `"key" "value"`.
fn parse_kv(s: &str) -> Option<(String, String)> {
    if !s.starts_with('"') {
        return None;
    }
    let after_open = &s[1..];
    let key_end = after_open.find('"')?;
    let key = after_open[..key_end].to_string();
    let after_key = after_open[key_end + 1..].trim();
    if !after_key.starts_with('"') {
        return None;
    }
    let val_inner = &after_key[1..];
    let val_end = val_inner.rfind('"')?;
    Some((key, val_inner[..val_end].to_string()))
}

// ── Non-Steam app ID ─────────────────────────────────────────────────────────

// Algorithm used by Steam and every third-party launcher (Heroic, Lutris, etc.).
// Input: exe path + app name concatenated; output: CRC32 with top bit set.
pub fn non_steam_app_id(exe: &str, app_name: &str) -> u32 {
    let input = format!("{exe}{app_name}");
    let crc = crc32fast::hash(input.as_bytes());
    crc | 0x80000000
}

// ── Binary VDF (shortcuts.vdf) ───────────────────────────────────────────────

const VDF_MAP: u8 = 0x00;
const VDF_STR: u8 = 0x01;
const VDF_INT: u8 = 0x02;
const VDF_END: u8 = 0x08;

#[derive(Debug, Clone)]
enum Bvdf {
    Map(Vec<(String, Bvdf)>),
    Str(String),
    Int(u32),
}

fn bvdf_parse_item(data: &[u8], pos: &mut usize) -> Result<(String, Bvdf), String> {
    if *pos >= data.len() {
        return Err("unexpected EOF in binary VDF".into());
    }
    let type_byte = data[*pos];
    *pos += 1;
    let key = bvdf_read_cstr(data, pos)?;

    match type_byte {
        VDF_MAP => {
            let mut children = Vec::new();
            loop {
                if *pos >= data.len() {
                    break;
                }
                if data[*pos] == VDF_END {
                    *pos += 1;
                    break;
                }
                children.push(bvdf_parse_item(data, pos)?);
            }
            Ok((key, Bvdf::Map(children)))
        }
        VDF_STR => {
            let val = bvdf_read_cstr(data, pos)?;
            Ok((key, Bvdf::Str(val)))
        }
        VDF_INT => {
            if *pos + 4 > data.len() {
                return Err("truncated int32".into());
            }
            let val = u32::from_le_bytes(data[*pos..*pos + 4].try_into().unwrap());
            *pos += 4;
            Ok((key, Bvdf::Int(val)))
        }
        other => Err(format!(
            "unknown VDF type 0x{other:02x} at byte {}",
            *pos - 1
        )),
    }
}

fn bvdf_read_cstr(data: &[u8], pos: &mut usize) -> Result<String, String> {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0x00 {
        *pos += 1;
    }
    let s = String::from_utf8_lossy(&data[start..*pos]).into_owned();
    if *pos < data.len() {
        *pos += 1;
    } // consume null
    Ok(s)
}

fn bvdf_write_item(key: &str, val: &Bvdf, out: &mut Vec<u8>) {
    match val {
        Bvdf::Map(children) => {
            out.push(VDF_MAP);
            out.extend_from_slice(key.as_bytes());
            out.push(0x00);
            for (k, v) in children {
                bvdf_write_item(k, v, out);
            }
            out.push(VDF_END);
        }
        Bvdf::Str(s) => {
            out.push(VDF_STR);
            out.extend_from_slice(key.as_bytes());
            out.push(0x00);
            out.extend_from_slice(s.as_bytes());
            out.push(0x00);
        }
        Bvdf::Int(n) => {
            out.push(VDF_INT);
            out.extend_from_slice(key.as_bytes());
            out.push(0x00);
            out.extend_from_slice(&n.to_le_bytes());
        }
    }
}

fn shortcut_entry(app_id: u32, exe: &str, start_dir: &str) -> Bvdf {
    Bvdf::Map(vec![
        ("appid".into(), Bvdf::Int(app_id)),
        ("AppName".into(), Bvdf::Str(APP_NAME.into())),
        ("Exe".into(), Bvdf::Str(exe.into())),
        ("StartDir".into(), Bvdf::Str(start_dir.into())),
        ("icon".into(), Bvdf::Str(String::new())),
        ("ShortcutPath".into(), Bvdf::Str(String::new())),
        ("LaunchOptions".into(), Bvdf::Str(String::new())),
        ("IsHidden".into(), Bvdf::Int(0)),
        ("AllowDesktopConfig".into(), Bvdf::Int(1)),
        ("AllowOverlay".into(), Bvdf::Int(1)),
        ("openvr".into(), Bvdf::Int(0)),
        ("Devkit".into(), Bvdf::Int(0)),
        ("DevkitGameID".into(), Bvdf::Str(String::new())),
        ("DevkitOverrideAppID".into(), Bvdf::Int(0)),
        ("LastPlayTime".into(), Bvdf::Int(0)),
        ("FlatpakAppID".into(), Bvdf::Str(String::new())),
        ("tags".into(), Bvdf::Map(vec![])),
    ])
}

// Reads shortcuts.vdf, removes any existing Evolve entry, appends a fresh one.
// Returns the non-Steam app_id written.
pub fn write_shortcut(shortcuts_path: &Path, exe: &str, start_dir: &str) -> Result<u32, String> {
    let app_id = non_steam_app_id(exe, APP_NAME);

    let mut entries: Vec<(String, Bvdf)> = match fs::read(shortcuts_path) {
        Ok(bytes) if !bytes.is_empty() => {
            let mut pos = 0;
            let (root_key, root_val) = bvdf_parse_item(&bytes, &mut pos)
                .map_err(|e| format!("parse shortcuts.vdf: {e}"))?;
            if root_key.to_lowercase() != "shortcuts" {
                return Err(format!("unexpected shortcuts.vdf root key: '{root_key}'"));
            }
            match root_val {
                Bvdf::Map(c) => c,
                _ => return Err("shortcuts root is not a map".into()),
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => vec![],
        Err(e) => return Err(format!("read shortcuts.vdf: {e}")),
        Ok(_) => vec![], // empty file
    };

    // Remove any previous Evolve entry (idempotent upsert).
    entries.retain(|(_, v)| {
        !matches!(v, Bvdf::Map(fields) if fields.iter().any(|(k, v)| {
            k == "AppName" && matches!(v, Bvdf::Str(s) if s == APP_NAME)
        }))
    });

    // Append under the next sequential key.
    let next_key = entries
        .iter()
        .filter_map(|(k, _)| k.parse::<usize>().ok())
        .max()
        .map(|n| n + 1)
        .unwrap_or(0);
    entries.push((next_key.to_string(), shortcut_entry(app_id, exe, start_dir)));

    let mut out = Vec::new();
    bvdf_write_item("shortcuts", &Bvdf::Map(entries), &mut out);
    out.push(VDF_END); // trailing byte present in real shortcuts.vdf files

    fs::write(shortcuts_path, &out).map_err(|e| format!("write shortcuts.vdf: {e}"))?;
    Ok(app_id)
}

// ── Proton detection ─────────────────────────────────────────────────────────

// Returns the path to the `proton` script inside the best available Proton
// installation: Proton Experimental first, then the highest-versioned release.
pub fn find_proton(steam_root: &Path) -> Option<PathBuf> {
    let common = steam_root.join("steamapps/common");
    let mut candidates: Vec<_> = fs::read_dir(&common)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s.starts_with("Proton") && e.path().join("proton").exists()
        })
        .collect();

    // Priority: Proton 8.0 (best for Evolve) → Experimental → newest version.
    candidates.sort_by(|a, b| {
        let an = a.file_name();
        let bn = b.file_name();
        let as_ = an.to_string_lossy();
        let bs_ = bn.to_string_lossy();
        let rank = |s: &str| {
            if s.contains("8.0") {
                0
            } else if s.contains("Experimental") {
                1
            } else {
                2
            }
        };
        match rank(&as_).cmp(&rank(&bs_)) {
            std::cmp::Ordering::Equal => bs_.cmp(&as_), // tie-break: newest first
            other => other,
        }
    });

    candidates.first().map(|e| e.path().join("proton"))
}

// ── Public entry point ───────────────────────────────────────────────────────

// Writes a Steam non-Steam shortcut pointing at the launcher binary itself.
// The launcher runs natively; Proton is invoked by the launcher at play time.
pub fn add_to_steam(steam_root: &Path, steam_id: &str, launcher_exe: &Path) -> Result<(), String> {
    let exe_str = launcher_exe.to_string_lossy().into_owned();
    let start_str = launcher_exe
        .parent()
        .unwrap_or(launcher_exe)
        .to_string_lossy()
        .into_owned();

    let shortcuts_path = steam_root
        .join("userdata")
        .join(steam_id)
        .join("config/shortcuts.vdf");

    if let Some(parent) = shortcuts_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    write_shortcut(&shortcuts_path, &exe_str, &start_str)?;
    Ok(())
}

// ── Donor game detection ─────────────────────────────────────────────────────

fn donor_acf_name(app_id: u32) -> String {
    format!("appmanifest_{app_id}.acf")
}

/// Find the install directory of the donor game by scanning Steam's appmanifest ACF files.
/// Returns None if Steam root doesn't exist or the donor game is not installed.
pub fn find_donor_game_dir(steam_root: &Path, app_id: u32) -> Option<PathBuf> {
    let acf_name = donor_acf_name(app_id);
    for steamapps in &["steamapps", "steam/steamapps"] {
        let acf_path = steam_root.join(steamapps).join(&acf_name);
        if !acf_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&acf_path).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("\"installdir\"") {
                let parts: Vec<&str> = line.splitn(2, '\t').collect();
                if let Some(dir_part) = parts.last() {
                    let dir_name = dir_part.trim().trim_matches('"');
                    let install_dir = steam_root.join(steamapps).join("common").join(dir_name);
                    if install_dir.exists() {
                        return Some(install_dir);
                    }
                }
            }
        }
    }
    None
}

/// Copy steam_api64.dll from the donor game's directory into the Evolve bin dir,
/// renaming it to steam_api64_real.dll so our shim can load it.
pub fn copy_steam_api_dll(donor_dir: &Path, game_bin_dir: &Path) -> Result<(), String> {
    let candidates = [
        donor_dir.join("steam_api64.dll"),
        donor_dir.join("bin").join("steam_api64.dll"),
        donor_dir.join("bin64").join("steam_api64.dll"),
    ];
    let src = candidates.iter().find(|p| p.exists()).ok_or_else(|| {
        format!(
            "steam_api64.dll not found in donor game directory: {}",
            donor_dir.display()
        )
    })?;

    let dest = game_bin_dir.join(crate::donor::REAL_STEAM_API_DLL);
    std::fs::copy(src, &dest)
        .map(|_| ())
        .map_err(|e| format!("Failed to copy steam_api64.dll: {e}"))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn app_id_is_stable() {
        let id = non_steam_app_id("/games/evolve/bin64_SteamRetail/Evolve.exe", APP_NAME);
        assert_eq!(
            id,
            non_steam_app_id("/games/evolve/bin64_SteamRetail/Evolve.exe", APP_NAME)
        );
        assert!(id & 0x80000000 != 0, "top bit must be set");
    }

    #[test]
    fn shortcut_roundtrip_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("shortcuts.vdf");

        let id = write_shortcut(&path, "/games/evolve/Evolve.exe", "/games/evolve").unwrap();
        assert!(id & 0x80000000 != 0);

        let bytes = fs::read(&path).unwrap();
        // Should contain APP_NAME as a UTF-8 string somewhere in the blob.
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains(APP_NAME));
        assert!(text.contains("/games/evolve/Evolve.exe"));
    }

    #[test]
    fn shortcut_upsert_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("shortcuts.vdf");

        write_shortcut(&path, "/games/evolve/Evolve.exe", "/games/evolve").unwrap();
        let len1 = fs::read(&path).unwrap().len();

        write_shortcut(&path, "/games/evolve/Evolve.exe", "/games/evolve").unwrap();
        let len2 = fs::read(&path).unwrap().len();

        assert_eq!(len1, len2, "re-writing same entry must not grow the file");
    }

    #[test]
    fn parse_kv_handles_tabs() {
        let (k, v) = parse_kv("\"PersonaName\"\t\t\"CoolGamer\"").unwrap();
        assert_eq!(k, "PersonaName");
        assert_eq!(v, "CoolGamer");
    }

    #[test]
    fn extract_sole_quoted_rejects_kv() {
        // A KV line is NOT a sole quoted token.
        assert!(extract_sole_quoted("\"PersonaName\"\t\"value\"").is_none());
        // A bare ID line is.
        assert_eq!(
            extract_sole_quoted("\"76561198012345678\""),
            Some("76561198012345678".into())
        );
    }

    #[test]
    fn donor_acf_filename_is_correct() {
        assert_eq!(donor_acf_name(480), "appmanifest_480.acf");
        assert_eq!(donor_acf_name(730), "appmanifest_730.acf");
    }

    #[test]
    fn find_donor_game_dir_returns_none_for_missing_acf() {
        let fake_root = std::path::PathBuf::from("/tmp/no_such_steam_root_xyz");
        assert!(find_donor_game_dir(&fake_root, 480).is_none());
    }
}
