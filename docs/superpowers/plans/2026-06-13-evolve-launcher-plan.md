# Evolve Launcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a one-click Windows launcher that patches the game to use the community server, shows server health and player count, and auto-updates the patched Goldberg DLL.

**Architecture:** Tauri 2 app with a React + TypeScript frontend. Rust backend handles file I/O (patching `EvolveLogging.ini`, copying DLLs, launching the game). The frontend polls the evolve-server's `/status` endpoint and renders server status + a Play button.

**Tech Stack:** Tauri 2, Rust, React 18 + TypeScript, Vite, Windows target (x86_64-pc-windows-msvc)

**Prerequisites:**
- `rustup target add x86_64-pc-windows-msvc`
- `npm` or `pnpm`
- Tauri CLI: `cargo install tauri-cli`

---

## File Map

```
evolve-launcher/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs              # Tauri builder, registers all commands
│       ├── setup.rs             # detect_game_path, run_first_setup
│       ├── updater.rs           # check_for_updates, download_dll
│       └── launcher.rs          # launch_game
├── src/
│   ├── main.tsx                 # React entry
│   ├── App.tsx                  # root component, first-run gate
│   ├── components/
│   │   ├── StatusBar.tsx        # server online status + player count
│   │   ├── PlayButton.tsx       # launch game button
│   │   └── VersionInfo.tsx      # version + last updated display
│   └── hooks/
│       └── useServerStatus.ts   # polls /status every 30s
├── index.html
├── package.json
├── vite.config.ts
└── tsconfig.json
```

---

### Task 1: Tauri + React scaffold

- [ ] **Step 1: Create the Tauri app**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher
cargo tauri init
```

When prompted:
- App name: `Evolve Launcher`
- Window title: `Evolve Community Launcher`
- Web assets: `../dist`
- Dev URL: `http://localhost:5173`
- Dev command: `npm run dev`
- Build command: `npm run build`

- [ ] **Step 2: Initialize React frontend**

```bash
cd /home/navitank/Desktop/evolve-revival/evolve-launcher
npm create vite@latest . -- --template react-ts --yes
npm install
```

- [ ] **Step 3: Update package.json scripts to match Tauri dev URL**

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri"
  }
}
```

- [ ] **Step 4: Update tauri.conf.json**

In `src-tauri/tauri.conf.json`, set:
```json
{
  "productName": "evolve-launcher",
  "version": "1.0.0",
  "identifier": "com.evolve-revival.launcher",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Evolve Community Launcher",
        "width": 480,
        "height": 320,
        "resizable": false
      }
    ]
  }
}
```

- [ ] **Step 5: Write a minimal App.tsx to verify the scaffold works**

```tsx
// evolve-launcher/src/App.tsx
export default function App() {
  return <div style={{ padding: 24, fontFamily: 'sans-serif' }}>
    <h2>Evolve Community Launcher</h2>
    <p>Loading...</p>
  </div>
}
```

- [ ] **Step 6: Verify dev server starts**

```bash
cd evolve-launcher && npm run dev
```

Expected: Vite dev server starts at http://localhost:5173, page shows "Evolve Community Launcher".

- [ ] **Step 7: Commit**

```bash
cd evolve-launcher && git add . && git commit -m "feat: Tauri + React scaffold for evolve-launcher"
```

---

### Task 2: Game path detection (Rust command)

The game install path is stored in the Windows registry at:
`HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 273350\InstallLocation`

The launcher tries the registry first, then falls back to asking the user.

**Files:**
- Create: `evolve-launcher/src-tauri/src/setup.rs`
- Modify: `evolve-launcher/src-tauri/src/main.rs`

- [ ] **Step 1: Add winreg dependency to Cargo.toml**

```toml
# src-tauri/Cargo.toml [dependencies] section
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
winreg = "0.52"
reqwest = { version = "0.12", features = ["json", "blocking"] }
tokio = { version = "1", features = ["full"] }
```

- [ ] **Step 2: Write setup.rs with detect_game_path**

```rust
// evolve-launcher/src-tauri/src/setup.rs
use std::path::PathBuf;

#[cfg(target_os = "windows")]
fn registry_game_path() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 273350",
        )
        .ok()?;
    let path: String = key.get_value("InstallLocation").ok()?;
    Some(PathBuf::from(path))
}

#[cfg(not(target_os = "windows"))]
fn registry_game_path() -> Option<PathBuf> {
    None
}

/// Returns the detected game install path, or None if not found.
#[tauri::command]
pub fn detect_game_path() -> Option<String> {
    if let Some(p) = registry_game_path() {
        if p.join("bin64_SteamRetail").join("Evolve.exe").exists() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

/// Returns true if the game exists at the given path.
#[tauri::command]
pub fn validate_game_path(path: String) -> bool {
    let p = PathBuf::from(&path);
    p.join("bin64_SteamRetail").join("Evolve.exe").exists()
}
```

- [ ] **Step 3: Register commands in main.rs**

```rust
// evolve-launcher/src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod setup;
mod updater;
mod launcher;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            setup::detect_game_path,
            setup::validate_game_path,
            setup::run_first_setup,
            updater::check_for_updates,
            launcher::launch_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Add placeholder stubs for missing commands (so it compiles)**

```rust
// evolve-launcher/src-tauri/src/updater.rs
#[tauri::command]
pub async fn check_for_updates(_server_url: String) -> Result<bool, String> {
    Ok(false) // TODO: Task 6
}
```

```rust
// evolve-launcher/src-tauri/src/launcher.rs
#[tauri::command]
pub fn launch_game(_game_path: String) -> Result<(), String> {
    Ok(()) // TODO: Task 5
}
```

- [ ] **Step 5: Build to verify it compiles**

```bash
cd evolve-launcher/src-tauri && cargo build 2>&1 | tail -10
```

Expected: `Compiling evolve-launcher` ... `Finished` with no errors.

- [ ] **Step 6: Add GamePath setup UI component**

```tsx
// evolve-launcher/src/components/GamePath.tsx
import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

export default function GamePath({ onConfirm }: { onConfirm: (path: string) => void }) {
  const [path, setPath] = useState('')
  const [error, setError] = useState('')

  const confirm = async () => {
    const valid: boolean = await invoke('validate_game_path', { path })
    if (valid) {
      onConfirm(path)
    } else {
      setError('Evolve.exe not found at this path. Navigate to the folder containing bin64_SteamRetail.')
    }
  }

  return (
    <div style={{ padding: 24 }}>
      <p>Enter your Evolve install directory:</p>
      <input
        style={{ width: '100%', marginBottom: 8 }}
        value={path}
        onChange={e => setPath(e.target.value)}
        placeholder="e.g. C:\Program Files (x86)\Steam\steamapps\common\Evolve"
      />
      {error && <p style={{ color: 'red', fontSize: 12 }}>{error}</p>}
      <button onClick={confirm}>Confirm</button>
    </div>
  )
}
```

- [ ] **Step 7: Commit**

```bash
cd evolve-launcher && git add . && git commit -m "feat: game path detection via registry + validation command"
```

---

### Task 3: First-run setup (Rust command)

On first run, the launcher patches `EvolveLogging.ini` to point to the community server and drops the bundled `GoldbergNewEvolveEmu.dll`. A marker file `evolve-launcher-setup.done` in the game directory prevents re-running.

**Files:**
- Modify: `evolve-launcher/src-tauri/src/setup.rs`

The community server domain and the bundled DLL bytes are embedded at compile time.

The DLL to bundle: copy `GoldbergNewEvolveEmu.dll` (the patched version built from the evolve-goldberg fork) into `src-tauri/resources/GoldbergNewEvolveEmu.dll`. For now, bundle the existing unpatched DLL as a placeholder.

- [ ] **Step 1: Create resources directory and copy placeholder DLL**

```bash
mkdir -p evolve-launcher/src-tauri/resources
cp /home/navitank/Desktop/EvolveFilesLegacy/bin64_SteamRetail/GoldbergNewEvolveEmu.dll \
   evolve-launcher/src-tauri/resources/GoldbergNewEvolveEmu.dll
```

- [ ] **Step 2: Add resource bundle to tauri.conf.json**

In `tauri.conf.json`, add inside `"bundle"`:
```json
"bundle": {
  "active": true,
  "resources": ["resources/*"]
}
```

- [ ] **Step 3: Add run_first_setup to setup.rs**

```rust
// Add to evolve-launcher/src-tauri/src/setup.rs

use std::fs;
use tauri::Manager;

const SERVER_DOMAIN: &str = "community.evolve-revival.net";
const SERVER_PORT: &str = "443";
const MARKER_FILE: &str = "evolve-launcher-setup.done";
const INI_FILENAME: &str = "EvolveLogging.ini";
const DLL_FILENAME: &str = "GoldbergNewEvolveEmu.dll";

/// Patches EvolveLogging.ini and drops the bundled DLL into bin64_SteamRetail.
/// Returns Ok(true) if setup ran, Ok(false) if already done.
#[tauri::command]
pub fn run_first_setup(app: tauri::AppHandle, game_path: String) -> Result<bool, String> {
    let bin64 = PathBuf::from(&game_path).join("bin64_SteamRetail");
    let marker = bin64.join(MARKER_FILE);

    if marker.exists() {
        return Ok(false);
    }

    // Patch EvolveLogging.ini
    let ini_path = bin64.join(INI_FILENAME);
    let ini_content = format!(
        "[server]\nserver_domain = {}\nserver_port = {}\nuse_internal_server = false\ninternal_server_dll = EvolveLegacyRebornServer.dll\nexternal_ip_list = \n\n[steam]\nemu_steam = true\ndll_path = {}\n",
        SERVER_DOMAIN, SERVER_PORT, DLL_FILENAME
    );
    fs::write(&ini_path, ini_content)
        .map_err(|e| format!("write ini: {}", e))?;

    // Copy bundled DLL
    let dll_src = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("resources")
        .join(DLL_FILENAME);
    let dll_dst = bin64.join(DLL_FILENAME);
    fs::copy(&dll_src, &dll_dst)
        .map_err(|e| format!("copy dll: {}", e))?;

    // Write marker
    fs::write(&marker, "1").map_err(|e| format!("write marker: {}", e))?;

    Ok(true)
}
```

- [ ] **Step 4: Update main.rs handler list** (already included `setup::run_first_setup` in Task 2 Step 3 — nothing to change)

- [ ] **Step 5: Build to verify it compiles**

```bash
cd evolve-launcher/src-tauri && cargo build 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 6: Commit**

```bash
cd evolve-launcher && git add . && git commit -m "feat: first-run setup — patches EvolveLogging.ini + drops DLL"
```

---

### Task 4: Server status hook + StatusBar component

The launcher polls `GET https://<server>/status` every 30 seconds and displays online/offline + player count.

**Files:**
- Create: `evolve-launcher/src/hooks/useServerStatus.ts`
- Create: `evolve-launcher/src/components/StatusBar.tsx`

- [ ] **Step 1: Write useServerStatus.ts**

```ts
// evolve-launcher/src/hooks/useServerStatus.ts
import { useState, useEffect } from 'react'

export interface ServerStatus {
  online: boolean
  onlinePlayers: number
  version: string
  error?: string
}

const SERVER_URL = 'https://community.evolve-revival.net'

export function useServerStatus(): ServerStatus {
  const [status, setStatus] = useState<ServerStatus>({
    online: false,
    onlinePlayers: 0,
    version: '',
  })

  useEffect(() => {
    const poll = async () => {
      try {
        const res = await fetch(`${SERVER_URL}/status`, { signal: AbortSignal.timeout(5000) })
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        const data = await res.json()
        setStatus({
          online: true,
          onlinePlayers: data.onlinePlayers ?? 0,
          version: data.version ?? '',
        })
      } catch (e: unknown) {
        setStatus(prev => ({ ...prev, online: false, error: String(e) }))
      }
    }

    poll()
    const interval = setInterval(poll, 30_000)
    return () => clearInterval(interval)
  }, [])

  return status
}
```

- [ ] **Step 2: Write StatusBar.tsx**

```tsx
// evolve-launcher/src/components/StatusBar.tsx
import { useServerStatus } from '../hooks/useServerStatus'

export default function StatusBar() {
  const status = useServerStatus()
  const dot: React.CSSProperties = {
    display: 'inline-block',
    width: 10,
    height: 10,
    borderRadius: '50%',
    backgroundColor: status.online ? '#4caf50' : '#f44336',
    marginRight: 8,
  }
  return (
    <div style={{ display: 'flex', alignItems: 'center', fontSize: 13, color: '#ccc' }}>
      <span style={dot} />
      {status.online
        ? `Online — ${status.onlinePlayers} player${status.onlinePlayers !== 1 ? 's' : ''}`
        : 'Server offline'}
    </div>
  )
}
```

- [ ] **Step 3: Add StatusBar to App.tsx to verify it renders**

```tsx
// evolve-launcher/src/App.tsx
import StatusBar from './components/StatusBar'

export default function App() {
  return (
    <div style={{ padding: 24 }}>
      <h2>Evolve Community Launcher</h2>
      <StatusBar />
    </div>
  )
}
```

- [ ] **Step 4: Run dev server and verify the status bar renders**

```bash
cd evolve-launcher && npm run dev
```

Open http://localhost:5173. Expected: status bar shows "Server offline" (since the server isn't running locally) or "Online — N players" if the server is running.

- [ ] **Step 5: Commit**

```bash
cd evolve-launcher && git add . && git commit -m "feat: server status hook + StatusBar component"
```

---

### Task 5: Play button

**Files:**
- Modify: `evolve-launcher/src-tauri/src/launcher.rs`
- Create: `evolve-launcher/src/components/PlayButton.tsx`

- [ ] **Step 1: Write launcher.rs**

```rust
// evolve-launcher/src-tauri/src/launcher.rs
use std::path::PathBuf;
use std::process::Command;

/// Launches Evolve.exe from bin64_SteamRetail.
/// game_path is the root Evolve directory (parent of bin64_SteamRetail).
#[tauri::command]
pub fn launch_game(game_path: String) -> Result<(), String> {
    let exe = PathBuf::from(&game_path)
        .join("bin64_SteamRetail")
        .join("Evolve.exe");

    if !exe.exists() {
        return Err(format!("Evolve.exe not found at {:?}", exe));
    }

    Command::new(&exe)
        .current_dir(exe.parent().unwrap())
        .spawn()
        .map_err(|e| format!("launch: {}", e))?;

    Ok(())
}
```

- [ ] **Step 2: Write PlayButton.tsx**

```tsx
// evolve-launcher/src/components/PlayButton.tsx
import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface Props {
  gamePath: string
  disabled?: boolean
}

export default function PlayButton({ gamePath, disabled }: Props) {
  const [launching, setLaunching] = useState(false)
  const [error, setError] = useState('')

  const play = async () => {
    setLaunching(true)
    setError('')
    try {
      await invoke('launch_game', { gamePath })
    } catch (e: unknown) {
      setError(String(e))
    } finally {
      setLaunching(false)
    }
  }

  return (
    <div>
      <button
        onClick={play}
        disabled={disabled || launching}
        style={{
          fontSize: 18,
          padding: '10px 40px',
          background: disabled ? '#555' : '#1976d2',
          color: '#fff',
          border: 'none',
          borderRadius: 4,
          cursor: disabled ? 'not-allowed' : 'pointer',
        }}
      >
        {launching ? 'Launching…' : 'Play'}
      </button>
      {error && <p style={{ color: 'red', fontSize: 12 }}>{error}</p>}
    </div>
  )
}
```

- [ ] **Step 3: Commit**

```bash
cd evolve-launcher && git add . && git commit -m "feat: Play button + launch_game Tauri command"
```

---

### Task 6: Auto-updater

On every launch, the launcher checks `/build_config` on the server. If `dllVersion` is newer than the bundled DLL version, it downloads and replaces it silently. Launcher updates require user confirmation.

**Files:**
- Modify: `evolve-launcher/src-tauri/src/updater.rs`

The DLL version is tracked by writing a `evolve-launcher-dll-version.txt` file alongside the DLL.

- [ ] **Step 1: Write updater.rs**

```rust
// evolve-launcher/src-tauri/src/updater.rs
use reqwest::blocking::Client;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const SERVER_URL: &str = "https://community.evolve-revival.net";
const VERSION_FILE: &str = "evolve-launcher-dll-version.txt";
const DLL_FILENAME: &str = "GoldbergNewEvolveEmu.dll";

#[derive(Deserialize)]
struct BuildConfig {
    #[serde(rename = "dllVersion")]
    dll_version: String,
    #[serde(rename = "dllUrl")]
    dll_url: Option<String>,
    #[serde(rename = "launcherVersion")]
    launcher_version: String,
}

/// Checks for a newer DLL version. Downloads and replaces it if found.
/// Returns Ok(true) if a DLL update was applied, Ok(false) if up to date.
#[tauri::command]
pub fn check_for_updates(game_path: String) -> Result<bool, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let cfg: BuildConfig = client
        .get(format!("{}/build_config", SERVER_URL))
        .send()
        .map_err(|e| format!("fetch build_config: {}", e))?
        .json()
        .map_err(|e| format!("parse build_config: {}", e))?;

    let bin64 = PathBuf::from(&game_path).join("bin64_SteamRetail");
    let version_file = bin64.join(VERSION_FILE);
    let current_version = fs::read_to_string(&version_file).unwrap_or_default();

    if current_version.trim() == cfg.dll_version.as_str() {
        return Ok(false); // already up to date
    }

    // Download new DLL if URL is provided
    if let Some(url) = &cfg.dll_url {
        let dll_bytes = client
            .get(url)
            .send()
            .map_err(|e| format!("download dll: {}", e))?
            .bytes()
            .map_err(|e| format!("read dll bytes: {}", e))?;

        let dll_dst = bin64.join(DLL_FILENAME);
        fs::write(&dll_dst, &dll_bytes)
            .map_err(|e| format!("write dll: {}", e))?;
    }

    fs::write(&version_file, &cfg.dll_version)
        .map_err(|e| format!("write version file: {}", e))?;

    Ok(true)
}
```

- [ ] **Step 2: Build to verify it compiles**

```bash
cd evolve-launcher/src-tauri && cargo build 2>&1 | tail -5
```

Expected: `Finished`.

- [ ] **Step 3: Call check_for_updates from App.tsx on startup**

```tsx
// In App.tsx, add to the useEffect that runs on mount:
import { invoke } from '@tauri-apps/api/core'

useEffect(() => {
  if (gamePath) {
    invoke('check_for_updates', { gamePath }).catch(console.error)
  }
}, [gamePath])
```

- [ ] **Step 4: Commit**

```bash
cd evolve-launcher && git add . && git commit -m "feat: auto-updater checks build_config and downloads new DLL"
```

---

### Task 7: VersionInfo + wire everything into App.tsx

**Files:**
- Create: `evolve-launcher/src/components/VersionInfo.tsx`
- Modify: `evolve-launcher/src/App.tsx`

- [ ] **Step 1: Write VersionInfo.tsx**

```tsx
// evolve-launcher/src/components/VersionInfo.tsx
import packageJson from '../../package.json'
import { useServerStatus } from '../hooks/useServerStatus'

export default function VersionInfo() {
  const status = useServerStatus()
  return (
    <div style={{ fontSize: 11, color: '#888', marginTop: 8 }}>
      Launcher v{packageJson.version}
      {status.version ? ` · Server v${status.version}` : ''}
    </div>
  )
}
```

- [ ] **Step 2: Write the final App.tsx**

This wires together first-run detection, game path, and all UI components:

```tsx
// evolve-launcher/src/App.tsx
import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import StatusBar from './components/StatusBar'
import PlayButton from './components/PlayButton'
import VersionInfo from './components/VersionInfo'
import GamePath from './components/GamePath'

type AppState = 'detecting' | 'need-path' | 'ready'

export default function App() {
  const [appState, setAppState] = useState<AppState>('detecting')
  const [gamePath, setGamePath] = useState('')

  useEffect(() => {
    // Try to auto-detect game path
    invoke<string | null>('detect_game_path').then(path => {
      if (path) {
        setGamePath(path)
        // Run first-time setup (idempotent — skips if already done)
        invoke('run_first_setup', { gamePath: path })
          .then(() => invoke('check_for_updates', { gamePath: path }))
          .catch(console.error)
          .finally(() => setAppState('ready'))
      } else {
        setAppState('need-path')
      }
    })
  }, [])

  const handlePathConfirmed = (path: string) => {
    setGamePath(path)
    invoke('run_first_setup', { gamePath: path })
      .then(() => invoke('check_for_updates', { gamePath: path }))
      .catch(console.error)
      .finally(() => setAppState('ready'))
  }

  if (appState === 'detecting') {
    return (
      <div style={{ padding: 24, fontFamily: 'sans-serif' }}>
        <p>Detecting Evolve installation…</p>
      </div>
    )
  }

  if (appState === 'need-path') {
    return <GamePath onConfirm={handlePathConfirmed} />
  }

  return (
    <div style={{
      padding: 24,
      fontFamily: 'sans-serif',
      background: '#1a1a2e',
      color: '#eee',
      height: '100vh',
      display: 'flex',
      flexDirection: 'column',
      gap: 16,
    }}>
      <h2 style={{ margin: 0 }}>Evolve Community Launcher</h2>
      <StatusBar />
      <div style={{ flex: 1, display: 'flex', alignItems: 'center' }}>
        <PlayButton gamePath={gamePath} />
      </div>
      <VersionInfo />
    </div>
  )
}
```

- [ ] **Step 3: Run the dev server and verify the full UI**

```bash
cd evolve-launcher && npm run dev
```

Open http://localhost:5173. Verify:
- Shows "Detecting Evolve installation…" briefly
- Then shows the main UI with StatusBar, Play button, and VersionInfo
- (If game not detected, shows GamePath input)

- [ ] **Step 4: Build the Tauri app (Windows cross-compile or native Windows build)**

```bash
cd evolve-launcher && cargo tauri build --target x86_64-pc-windows-msvc
```

Expected: produces `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/evolve-launcher_1.0.0_x64_en-US.msi`

- [ ] **Step 5: Commit**

```bash
cd evolve-launcher && git add . && git commit -m "feat: wire App.tsx — first-run gate, status bar, play button, auto-update"
```
