# Steam SDR Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Goldberg's emulated networking with a real Steamworks proxy shim so players get Steam SDR (IP privacy, geo-optimal relay), Steam overlay, and friend invites — without requiring Evolve ownership — by authenticating under a free donor game's App ID.

**Architecture:** A C++ proxy DLL (`evolve_shim.dll`) loads alongside the real `steam_api64.dll` (renamed to `steam_api64_real.dll`). The game loads the shim via `EvolveLogging.ini`'s `dll_path` hook; the shim forwards all Steamworks calls to the real DLL except three (DLC unlock + AppID). The launcher verifies donor game ownership, copies the real DLL into the game directory, and writes the correct config before each launch.

**Tech Stack:** C++ / MinGW-w64 cross-compile (shim DLL), Rust/Tauri (launcher), Svelte 5 (frontend), Go (server cleanup)

---

## File Map

**New:**
- `evolve-shim/CMakeLists.txt` — cross-compile build for Windows x64
- `evolve-shim/shim.cpp` — proxy DLL: 3 intercepts + DLL forwarding
- `evolve-shim/shim.def` — export table (forwarded + intercepted)

**Modified — Launcher (Rust):**
- `evolve-launcher/src-tauri/src/patcher.rs` — update `generate_logging_ini` (shim path, emu_steam false), add `write_steam_appid`, remove `custom_broadcasts.txt`
- `evolve-launcher/src-tauri/src/steam.rs` — add `find_donor_game_dir`, `copy_steam_api_dll`
- `evolve-launcher/src-tauri/src/commands.rs` — remove `ProxyState`, `get_nat_type`; update `launch_game` pre-flight; add `check_donor_game`
- `evolve-launcher/src-tauri/src/lib.rs` — remove `ProxyState` + `get_nat_type` from handler

**Modified — Frontend (Svelte):**
- `evolve-launcher/src/lib/SteamSetupView.svelte` — rewrite: donor game flow replaces account picker
- `evolve-launcher/src/lib/Main.svelte` — remove NAT indicator
- `evolve-launcher/src/types.ts` — remove `NatInfo`, add `DonorStatus`

**Modified — Server (Go):**
- `evolve-server/internal/handler/punch.go` — delete
- `evolve-server/cmd/server/router.go` — remove `/peers/register` route + relay param
- `evolve-server/cmd/server/main.go` — remove relay startup

---

## Task 1: Donor App ID constant + research note

**Files:**
- Create: `evolve-launcher/src-tauri/src/donor.rs`
- Modify: `evolve-launcher/src-tauri/src/lib.rs` (add `mod donor;`)

**Background:** The donor game must be free on Steam and have SDR enabled. Two recommended candidates to verify during this task:
- **Spacewar (480)** — Valve's official Steamworks test app, free, definitely has full SDR. Check: `steam://install/480`
- **CS2 (730)** — free, confirmed SDR, very large player base (more traffic cover)

Verify SDR is active by checking whether the game uses `ISteamNetworkingSockets` (look in its Steamworks partner page or test with a connection probe). Pick one and set `DONOR_APP_ID` below.

- [ ] **Step 1: Create `donor.rs`**

```rust
// evolve-launcher/src-tauri/src/donor.rs

/// Steam App ID of the free donor game used for SDR authentication.
/// Must be free-to-own on Steam and have ISteamNetworkingSockets SDR enabled.
/// Currently: Spacewar (Valve's Steamworks test app) — free, SDR confirmed.
pub const DONOR_APP_ID: u32 = 480;

/// The filename of the real Steamworks DLL after we rename it.
pub const REAL_STEAM_API_DLL: &str = "steam_api64_real.dll";

/// The filename of our proxy shim.
pub const SHIM_DLL: &str = "evolve_shim.dll";

/// Evolve's actual App ID — returned by shim's GetAppID() intercept.
pub const EVOLVE_APP_ID: u32 = 273350;
```

- [ ] **Step 2: Add `mod donor;` to `lib.rs`**

In `evolve-launcher/src-tauri/src/lib.rs`, add after `mod config;`:
```rust
mod donor;
```

- [ ] **Step 3: Commit**

```bash
git add evolve-launcher/src-tauri/src/donor.rs evolve-launcher/src-tauri/src/lib.rs
git commit -m "feat(donor): donor game constants (Spacewar App ID 480)"
```

---

## Task 2: Build `evolve_shim.dll`

**Files:**
- Create: `evolve-shim/CMakeLists.txt`
- Create: `evolve-shim/shim.cpp`
- Create: `evolve-shim/shim.def`

**Context:** The game loads `evolve_shim.dll` via `EvolveLogging.ini` `dll_path`. The shim sits next to `steam_api64_real.dll` (the real Steamworks DLL we copy from the donor game). The DEF file forwards all exports to `steam_api64_real`; three functions are intercepted in C++ instead.

**Prerequisites:** Install MinGW-w64 cross-compiler:
```bash
sudo pacman -S mingw-w64-gcc   # Arch/Zen
```

- [ ] **Step 1: Extract export list from the real `steam_api64.dll`**

Copy `steam_api64.dll` from the donor game's Steam install directory. On Linux, the donor game's files are at `~/.steam/steam/steamapps/common/<DonorGameName>/`. Extract exports:

```bash
python3 - <<'EOF'
import struct, sys

data = open("steam_api64.dll", "rb").read()
pe_off = struct.unpack_from("<I", data, 0x3C)[0]
opt_off = pe_off + 24
opt_size = struct.unpack_from("<H", data, pe_off + 20)[0]
exp_rva = struct.unpack_from("<I", data, opt_off + 112)[0]
num_sections = struct.unpack_from("<H", data, pe_off + 6)[0]
sect_off = opt_off + opt_size

def rva2off(rva):
    for i in range(num_sections):
        s = sect_off + i * 40
        vaddr = struct.unpack_from("<I", data, s + 12)[0]
        vsz   = struct.unpack_from("<I", data, s + 16)[0]
        raw   = struct.unpack_from("<I", data, s + 20)[0]
        if vaddr <= rva < vaddr + vsz:
            return raw + (rva - vaddr)
    return None

exp_off = rva2off(exp_rva)
num_names = struct.unpack_from("<I", data, exp_off + 24)[0]
names_rva = struct.unpack_from("<I", data, exp_off + 32)[0]
names_off = rva2off(names_rva)
for i in range(num_names):
    n_rva = struct.unpack_from("<I", data, names_off + i * 4)[0]
    n_off = rva2off(n_rva)
    print(data[n_off:n_off+256].split(b"\x00")[0].decode())
EOF
```

Save the output — you'll use it to populate `shim.def`.

- [ ] **Step 2: Create `evolve-shim/shim.def`**

```
LIBRARY evolve_shim
EXPORTS
; All exports forwarded to the real DLL — add every name from the extract above
; EXCEPT the three we intercept (listed at the bottom without = forwarding)
SteamAPI_Init = steam_api64_real.SteamAPI_Init
SteamAPI_InitAnonymousUser = steam_api64_real.SteamAPI_InitAnonymousUser
SteamAPI_InitFlat = steam_api64_real.SteamAPI_InitFlat
SteamAPI_ManualDispatch_FreeLastCallback = steam_api64_real.SteamAPI_ManualDispatch_FreeLastCallback
SteamAPI_ManualDispatch_GetAPICallResult = steam_api64_real.SteamAPI_ManualDispatch_GetAPICallResult
SteamAPI_ManualDispatch_GetNextCallback = steam_api64_real.SteamAPI_ManualDispatch_GetNextCallback
SteamAPI_ManualDispatch_Init = steam_api64_real.SteamAPI_ManualDispatch_Init
SteamAPI_ManualDispatch_RunFrame = steam_api64_real.SteamAPI_ManualDispatch_RunFrame
SteamAPI_RegisterCallback = steam_api64_real.SteamAPI_RegisterCallback
SteamAPI_RegisterCallResult = steam_api64_real.SteamAPI_RegisterCallResult
SteamAPI_ReleaseCurrentThreadMemory = steam_api64_real.SteamAPI_ReleaseCurrentThreadMemory
SteamAPI_RestartAppIfNecessary = steam_api64_real.SteamAPI_RestartAppIfNecessary
SteamAPI_RunCallbacks = steam_api64_real.SteamAPI_RunCallbacks
SteamAPI_SetMiniDumpComment = steam_api64_real.SteamAPI_SetMiniDumpComment
SteamAPI_Shutdown = steam_api64_real.SteamAPI_Shutdown
SteamAPI_SteamApps = steam_api64_real.SteamAPI_SteamApps
SteamAPI_SteamFriends = steam_api64_real.SteamAPI_SteamFriends
SteamAPI_SteamGameServer = steam_api64_real.SteamAPI_SteamGameServer
SteamAPI_SteamInput = steam_api64_real.SteamAPI_SteamInput
SteamAPI_SteamInventory = steam_api64_real.SteamAPI_SteamInventory
SteamAPI_SteamMatchmaking = steam_api64_real.SteamAPI_SteamMatchmaking
SteamAPI_SteamMatchmakingServers = steam_api64_real.SteamAPI_SteamMatchmakingServers
SteamAPI_SteamNetworking = steam_api64_real.SteamAPI_SteamNetworking
SteamAPI_SteamNetworkingMessages = steam_api64_real.SteamAPI_SteamNetworkingMessages
SteamAPI_SteamNetworkingSockets = steam_api64_real.SteamAPI_SteamNetworkingSockets
SteamAPI_SteamNetworkingUtils = steam_api64_real.SteamAPI_SteamNetworkingUtils
SteamAPI_SteamParties = steam_api64_real.SteamAPI_SteamParties
SteamAPI_SteamRemoteStorage = steam_api64_real.SteamAPI_SteamRemoteStorage
SteamAPI_SteamScreenshots = steam_api64_real.SteamAPI_SteamScreenshots
SteamAPI_SteamUGC = steam_api64_real.SteamAPI_SteamUGC
SteamAPI_SteamUser = steam_api64_real.SteamAPI_SteamUser
SteamAPI_SteamUserStats = steam_api64_real.SteamAPI_SteamUserStats
SteamAPI_SteamUtils = steam_api64_real.SteamAPI_SteamUtils
SteamAPI_UnregisterCallback = steam_api64_real.SteamAPI_UnregisterCallback
SteamAPI_UnregisterCallResult = steam_api64_real.SteamAPI_UnregisterCallResult
SteamAPI_WriteMiniDump = steam_api64_real.SteamAPI_WriteMiniDump
SteamAPI_GetHSteamPipe = steam_api64_real.SteamAPI_GetHSteamPipe
SteamAPI_GetHSteamUser = steam_api64_real.SteamAPI_GetHSteamUser
SteamAPI_IsSteamRunning = steam_api64_real.SteamAPI_IsSteamRunning
SteamAPI_GetSteamInstallPath = steam_api64_real.SteamAPI_GetSteamInstallPath
SteamGameServer_Init = steam_api64_real.SteamGameServer_Init
SteamGameServer_RunCallbacks = steam_api64_real.SteamGameServer_RunCallbacks
SteamGameServer_Shutdown = steam_api64_real.SteamGameServer_Shutdown
SteamAPI_ISteamClient_BReleaseSteamPipe = steam_api64_real.SteamAPI_ISteamClient_BReleaseSteamPipe
SteamAPI_ISteamClient_ConnectToGlobalUser = steam_api64_real.SteamAPI_ISteamClient_ConnectToGlobalUser
SteamAPI_ISteamClient_CreateLocalUser = steam_api64_real.SteamAPI_ISteamClient_CreateLocalUser
SteamAPI_ISteamClient_CreateSteamPipe = steam_api64_real.SteamAPI_ISteamClient_CreateSteamPipe
SteamAPI_ISteamClient_GetISteamApps = steam_api64_real.SteamAPI_ISteamClient_GetISteamApps
SteamAPI_ISteamClient_GetISteamFriends = steam_api64_real.SteamAPI_ISteamClient_GetISteamFriends
SteamAPI_ISteamClient_GetISteamMatchmaking = steam_api64_real.SteamAPI_ISteamClient_GetISteamMatchmaking
SteamAPI_ISteamClient_GetISteamNetworking = steam_api64_real.SteamAPI_ISteamClient_GetISteamNetworking
SteamAPI_ISteamClient_GetISteamRemoteStorage = steam_api64_real.SteamAPI_ISteamClient_GetISteamRemoteStorage
SteamAPI_ISteamClient_GetISteamUser = steam_api64_real.SteamAPI_ISteamClient_GetISteamUser
SteamAPI_ISteamClient_GetISteamUserStats = steam_api64_real.SteamAPI_ISteamClient_GetISteamUserStats
SteamAPI_ISteamClient_GetISteamUtils = steam_api64_real.SteamAPI_ISteamClient_GetISteamUtils
SteamAPI_ISteamFriends_InviteUserToGame = steam_api64_real.SteamAPI_ISteamFriends_InviteUserToGame
SteamAPI_ISteamFriends_SetRichPresence = steam_api64_real.SteamAPI_ISteamFriends_SetRichPresence
SteamAPI_ISteamFriends_GetFriendCount = steam_api64_real.SteamAPI_ISteamFriends_GetFriendCount
SteamAPI_ISteamMatchmaking_CreateLobby = steam_api64_real.SteamAPI_ISteamMatchmaking_CreateLobby
SteamAPI_ISteamMatchmaking_JoinLobby = steam_api64_real.SteamAPI_ISteamMatchmaking_JoinLobby
SteamAPI_ISteamMatchmaking_LeaveLobby = steam_api64_real.SteamAPI_ISteamMatchmaking_LeaveLobby
SteamAPI_ISteamMatchmaking_RequestLobbyList = steam_api64_real.SteamAPI_ISteamMatchmaking_RequestLobbyList
SteamAPI_ISteamNetworkingSockets_ConnectP2P = steam_api64_real.SteamAPI_ISteamNetworkingSockets_ConnectP2P
SteamAPI_ISteamNetworkingSockets_CreateListenSocketP2P = steam_api64_real.SteamAPI_ISteamNetworkingSockets_CreateListenSocketP2P
SteamAPI_ISteamUser_GetSteamID = steam_api64_real.SteamAPI_ISteamUser_GetSteamID
SteamAPI_ISteamUserStats_GetAchievement = steam_api64_real.SteamAPI_ISteamUserStats_GetAchievement
SteamAPI_ISteamUserStats_SetAchievement = steam_api64_real.SteamAPI_ISteamUserStats_SetAchievement
SteamAPI_ISteamUserStats_StoreStats = steam_api64_real.SteamAPI_ISteamUserStats_StoreStats
SteamAPI_ISteamUtils_GetConnectedUniverse = steam_api64_real.SteamAPI_ISteamUtils_GetConnectedUniverse
SteamAPI_ISteamUtils_GetServerRealTime = steam_api64_real.SteamAPI_ISteamUtils_GetServerRealTime
; Add remaining exports from the extract script above, each as:
; FuncName = steam_api64_real.FuncName

; --- Intercepted (implemented in shim.cpp — do NOT forward) ---
SteamAPI_ISteamApps_BIsDlcInstalled
SteamAPI_ISteamApps_BIsSubscribedApp
SteamAPI_ISteamUtils_GetAppID
```

- [ ] **Step 3: Create `evolve-shim/shim.cpp`**

```cpp
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <cstdint>

static HMODULE g_real = nullptr;

BOOL WINAPI DllMain(HINSTANCE, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH)
        g_real = LoadLibraryA("steam_api64_real.dll");
    return TRUE;
}

// Intercept 1: always report all DLCs as installed
extern "C" __declspec(dllexport)
bool __cdecl SteamAPI_ISteamApps_BIsDlcInstalled(void* /*self*/, uint32_t /*dlcId*/) {
    return true;
}

// Intercept 2: always report the game as owned (handles ownership check)
extern "C" __declspec(dllexport)
bool __cdecl SteamAPI_ISteamApps_BIsSubscribedApp(void* /*self*/, uint32_t /*appId*/) {
    return true;
}

// Intercept 3: report Evolve's real App ID so game-internal logic is unaffected
extern "C" __declspec(dllexport)
uint32_t __cdecl SteamAPI_ISteamUtils_GetAppID(void* /*self*/) {
    return 273350u;
}
```

- [ ] **Step 4: Create `evolve-shim/CMakeLists.txt`**

```cmake
cmake_minimum_required(VERSION 3.16)
project(evolve_shim CXX)

set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_C_COMPILER   x86_64-w64-mingw32-gcc)
set(CMAKE_CXX_COMPILER x86_64-w64-mingw32-g++)
set(CMAKE_RC_COMPILER  x86_64-w64-mingw32-windres)

add_library(evolve_shim SHARED shim.cpp shim.def)
set_target_properties(evolve_shim PROPERTIES PREFIX "" SUFFIX ".dll")
target_compile_options(evolve_shim PRIVATE -O2 -Wall)
```

- [ ] **Step 5: Build**

```bash
cd evolve-shim
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
ls -lh build/evolve_shim.dll
```

Expected: `evolve_shim.dll` produced, size ~20–80 KB.

- [ ] **Step 6: Verify exports**

```bash
python3 - <<'EOF'
import struct
data = open("build/evolve_shim.dll","rb").read()
pe_off = struct.unpack_from("<I", data, 0x3C)[0]
opt_off = pe_off + 24
opt_size = struct.unpack_from("<H", data, pe_off + 20)[0]
exp_rva = struct.unpack_from("<I", data, opt_off + 112)[0]
num_sec = struct.unpack_from("<H", data, pe_off + 6)[0]
sec_off = opt_off + opt_size
def r2o(rva):
    for i in range(num_sec):
        s = sec_off + i*40
        va,vsz,raw = struct.unpack_from("<III", data, s+12)
        if va <= rva < va+vsz: return raw+(rva-va)
eo = r2o(exp_rva)
nn = struct.unpack_from("<I", data, eo+24)[0]
no = r2o(struct.unpack_from("<I", data, eo+32)[0])
names = []
for i in range(nn):
    nr = struct.unpack_from("<I", data, no+i*4)[0]
    names.append(data[r2o(nr):r2o(nr)+200].split(b"\x00")[0].decode())
for n in sorted(names): print(n)
EOF
```

Expected: `SteamAPI_ISteamApps_BIsDlcInstalled`, `SteamAPI_ISteamApps_BIsSubscribedApp`, `SteamAPI_ISteamUtils_GetAppID` appear in the list alongside all forwarded exports.

- [ ] **Step 7: Commit**

```bash
git add evolve-shim/
git commit -m "feat(shim): evolve_shim.dll proxy — DLC unlock + AppID intercept"
```

---

## Task 3: Patcher — write real Steamworks config

**Files:**
- Modify: `evolve-launcher/src-tauri/src/patcher.rs`

**Context:** `generate_logging_ini` currently outputs `emu_steam = true` and `dll_path = GoldbergNewEvolveEmu.dll`. It must now output `emu_steam = false` and `dll_path = evolve_shim.dll`. `apply_patches` currently writes `custom_broadcasts.txt` — that write must be removed. A new `write_steam_appid` function writes the donor App ID to `steam_appid.txt` in the bin dir.

- [ ] **Step 1: Write failing tests**

In `patcher.rs`, replace the two affected tests and add one new test:

```rust
#[test]
fn generates_correct_ini_content() {
    let ini = generate_logging_ini("https://revival.example.com:8443");
    assert!(ini.contains("[server]"));
    assert!(ini.contains("server_domain = revival.example.com"));
    assert!(ini.contains("server_port = 8443"));
    assert!(ini.contains("use_internal_server = false"));
    assert!(ini.contains("[steam]"));
    assert!(ini.contains("emu_steam = false"));
    assert!(ini.contains("dll_path = evolve_shim.dll"));
    assert!(!ini.contains("GoldbergNewEvolveEmu.dll"));
}

#[test]
fn steam_appid_content_is_donor_id() {
    assert_eq!(steam_appid_content(), "480\n");
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd evolve-launcher/src-tauri && cargo test patcher 2>&1 | grep -E "FAILED|error"
```

Expected: `generates_correct_ini_content` FAILED (still outputs old values), `steam_appid_content` FAILED (function doesn't exist).

- [ ] **Step 3: Update `patcher.rs`**

Replace `generate_logging_ini` and add `steam_appid_content`. Remove `PROXY_LOCAL_HOST` constant and the `custom_broadcasts.txt` write from `apply_patches`:

```rust
use crate::donor;
use crate::downloader::download_with_retry;
use crate::install::Manifest;
use reqwest::Client;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub fn extract_host(url: &str) -> String {
    let without_scheme = url.find("://").map(|i| &url[i + 3..]).unwrap_or(url);
    let host_and_port = without_scheme
        .find('/')
        .map(|i| &without_scheme[..i])
        .unwrap_or(without_scheme);
    host_and_port
        .rfind(':')
        .map(|i| &host_and_port[..i])
        .unwrap_or(host_and_port)
        .to_string()
}

pub fn extract_port(url: &str) -> u16 {
    let without_scheme = url.find("://").map(|i| &url[i + 3..]).unwrap_or(url);
    let host_and_port = without_scheme
        .find('/')
        .map(|i| &without_scheme[..i])
        .unwrap_or(without_scheme);
    if let Some(i) = host_and_port.rfind(':') {
        if let Ok(p) = host_and_port[i + 1..].parse::<u16>() {
            return p;
        }
    }
    if url.starts_with("https://") { 443 } else { 80 }
}

pub fn generate_logging_ini(server_url: &str) -> String {
    let host = extract_host(server_url);
    let port = extract_port(server_url);
    format!(
        "[server]\n\
         server_domain = {host}\n\
         server_port = {port}\n\
         use_internal_server = false\n\
         \n\
         [steam]\n\
         emu_steam = false\n\
         dll_path = {}\n",
        donor::SHIM_DLL
    )
}

/// Content for steam_appid.txt — real Steamworks reads this to determine App ID at startup.
pub fn steam_appid_content() -> String {
    format!("{}\n", donor::DONOR_APP_ID)
}

pub async fn apply_patches(
    client: &Client,
    manifest: &Manifest,
    install_dir: &Path,
    server_url: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(), String> {
    for patch in &manifest.patches {
        if std::path::Path::new(&patch.path).is_absolute() {
            return Err(format!("Manifest contains absolute patch path: {}", patch.path));
        }
        let dest = install_dir.join(&patch.path);
        if !dest.starts_with(install_dir) {
            return Err(format!("Manifest patch path escapes install dir: {}", patch.path));
        }
        let url = format!("{}{}", manifest.base_url, patch.path);
        download_with_retry(client, &url, &dest, &patch.sha256, cancelled.clone()).await?;
    }

    let bin_dir = install_dir.join("bin64_SteamRetail");
    std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

    std::fs::write(bin_dir.join("EvolveLogging.ini"), generate_logging_ini(server_url))
        .map_err(|e| format!("Failed to write EvolveLogging.ini: {e}"))?;

    std::fs::write(bin_dir.join("steam_appid.txt"), steam_appid_content())
        .map_err(|e| format!("Failed to write steam_appid.txt: {e}"))
}
```

- [ ] **Step 4: Update remaining tests in `patcher.rs`**

Remove the `custom_broadcasts_uses_localhost` test (no longer applicable). Keep all host/port extraction tests unchanged. Update `generates_ini_with_default_https_port` to also check `emu_steam = false`:

```rust
#[test]
fn generates_ini_with_default_https_port() {
    let ini = generate_logging_ini("https://play.evolve-community.net");
    assert!(ini.contains("server_port = 443"));
    assert!(ini.contains("emu_steam = false"));
}
```

- [ ] **Step 5: Run tests — all pass**

```bash
cd evolve-launcher/src-tauri && cargo test patcher 2>&1 | tail -5
```

Expected: all patcher tests PASS.

- [ ] **Step 6: Commit**

```bash
git add evolve-launcher/src-tauri/src/patcher.rs
git commit -m "feat(patcher): write real Steamworks config — shim dll_path, donor steam_appid.txt"
```

---

## Task 4: Donor verification in `steam.rs`

**Files:**
- Modify: `evolve-launcher/src-tauri/src/steam.rs`

**Context:** Before launch, the launcher must verify the donor game is installed (ACF scan) and copy `steam_api64.dll` from the donor's install dir into the game's `bin64_SteamRetail/` as `steam_api64_real.dll`. Two new public functions handle this.

- [ ] **Step 1: Write failing tests**

At the bottom of `steam.rs` tests block, add:

```rust
#[test]
fn donor_acf_filename_is_correct() {
    assert_eq!(donor_acf_name(480), "appmanifest_480.acf");
    assert_eq!(donor_acf_name(730), "appmanifest_730.acf");
}

#[test]
fn find_donor_game_dir_returns_none_for_missing_acf() {
    use std::path::PathBuf;
    // A path that definitely doesn't have any ACF files
    let fake_root = PathBuf::from("/tmp/no_such_steam_root_xyz");
    assert!(find_donor_game_dir(&fake_root, 480).is_none());
}
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd evolve-launcher/src-tauri && cargo test steam::tests::donor 2>&1 | grep -E "FAILED|error"
```

Expected: both tests FAIL (functions don't exist yet).

- [ ] **Step 3: Add donor functions to `steam.rs`**

Add these functions before the `#[cfg(test)]` block:

```rust
use crate::donor;

fn donor_acf_name(app_id: u32) -> String {
    format!("appmanifest_{app_id}.acf")
}

/// Find the install directory of the donor game by scanning Steam's appmanifest ACF files.
/// Returns None if Steam root doesn't exist or the donor game is not installed.
pub fn find_donor_game_dir(steam_root: &std::path::Path, app_id: u32) -> Option<std::path::PathBuf> {
    let acf_name = donor_acf_name(app_id);
    // Steam stores appmanifests in steamapps/ and steamapps/common/
    for steamapps in &["steamapps", "steam/steamapps"] {
        let acf_path = steam_root.join(steamapps).join(&acf_name);
        if !acf_path.exists() {
            continue;
        }
        // Parse "installdir" field from the ACF (VDF text format)
        let content = std::fs::read_to_string(&acf_path).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("\"installdir\"") {
                // Format: "installdir"		"GameDirectoryName"
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
/// Returns Err if the source DLL is not found or the copy fails.
pub fn copy_steam_api_dll(
    donor_dir: &std::path::Path,
    game_bin_dir: &std::path::Path,
) -> Result<(), String> {
    // The DLL may live at the game root or in a subdirectory — check common locations
    let candidates = [
        donor_dir.join("steam_api64.dll"),
        donor_dir.join("bin").join("steam_api64.dll"),
        donor_dir.join("bin64").join("steam_api64.dll"),
    ];
    let src = candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| format!(
            "steam_api64.dll not found in donor game directory: {}",
            donor_dir.display()
        ))?;

    let dest = game_bin_dir.join(donor::REAL_STEAM_API_DLL);
    std::fs::copy(src, &dest)
        .map(|_| ())
        .map_err(|e| format!("Failed to copy steam_api64.dll: {e}"))
}
```

- [ ] **Step 4: Run tests — all pass**

```bash
cd evolve-launcher/src-tauri && cargo test steam 2>&1 | tail -8
```

Expected: all steam tests PASS including the two new donor tests.

- [ ] **Step 5: Commit**

```bash
git add evolve-launcher/src-tauri/src/steam.rs
git commit -m "feat(steam): donor game ACF scan + steam_api64.dll copy"
```

---

## Task 5: `launch_game` pre-flight + remove proxy, `check_donor_game` command

**Files:**
- Modify: `evolve-launcher/src-tauri/src/commands.rs`

**Context:** `launch_game` currently starts the STUN proxy and registers with the relay. Both are removed. New pre-flight checks: Steam running, `steam_api64_real.dll` present, `EvolveLogging.ini` fresh. A new `check_donor_game` command is added for the setup view to call.

- [ ] **Step 1: Remove `ProxyState` and `get_nat_type`**

In `commands.rs`, delete the entire `ProxyState` struct and the `get_nat_type` function.

Remove from the imports:
```rust
// Remove these lines:
// (anything importing from crate::nat that's only used by ProxyState/get_nat_type)
```

- [ ] **Step 2: Add `check_donor_game` and `open_steam_store` commands**

Add after the Steam integration section:

```rust
// ── Donor / SDR setup ────────────────────────────────────────────────────

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
    let donor_name = format!("App ID {}", crate::donor::DONOR_APP_ID);

    let steam_root = match crate::steam::find_steam_root() {
        Some(r) => r,
        None => return DonorStatus { installed: false, dll_ready: false, donor_name },
    };

    let donor_dir = crate::steam::find_donor_game_dir(&steam_root, crate::donor::DONOR_APP_ID);
    let installed = donor_dir.is_some();

    let dll_ready = if cfg.install_dir.is_empty() {
        false
    } else {
        PathBuf::from(&cfg.install_dir)
            .join("bin64_SteamRetail")
            .join(crate::donor::REAL_STEAM_API_DLL)
            .exists()
    };

    DonorStatus { installed, dll_ready, donor_name, donor_app_id: crate::donor::DONOR_APP_ID }
}

#[tauri::command]
pub fn open_steam_store(app_id: u32) {
    let url = format!("steam://store/{app_id}");
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/c", "start", "", &url]).spawn();
    #[cfg(not(target_os = "windows"))]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
}
```

- [ ] **Step 3: Update `launch_game`**

Replace the existing `launch_game` with:

```rust
#[tauri::command]
pub async fn launch_game(app: AppHandle) -> Result<(), String> {
    let cfg = Config::load(&app);
    if cfg.install_dir.is_empty() {
        return Err("Game is not installed".to_string());
    }

    let bin_dir = PathBuf::from(&cfg.install_dir).join("bin64_SteamRetail");
    let exe = bin_dir.join("Evolve.exe");

    // Pre-flight 1: ensure steam_api64_real.dll is present (copy if missing)
    let real_dll = bin_dir.join(crate::donor::REAL_STEAM_API_DLL);
    if !real_dll.exists() {
        let steam_root = crate::steam::find_steam_root()
            .ok_or_else(|| "Steam not found — install Steam to play".to_string())?;
        let donor_dir = crate::steam::find_donor_game_dir(&steam_root, crate::donor::DONOR_APP_ID)
            .ok_or_else(|| format!(
                "Donor game (App ID {}) not installed — add it to your Steam library first",
                crate::donor::DONOR_APP_ID
            ))?;
        crate::steam::copy_steam_api_dll(&donor_dir, &bin_dir)?;
    }

    // Pre-flight 2: rewrite EvolveLogging.ini in case config drifted
    let ini = crate::patcher::generate_logging_ini(&cfg.server_url);
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
        let steam_running = std::process::Command::new("pgrep")
            .arg("-x")
            .arg("steam")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
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
        let compat_prefix = PathBuf::from(&cfg.install_dir).join("proton_prefix");
        std::fs::create_dir_all(&compat_prefix).map_err(|e| e.to_string())?;
        let cwd = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
        std::process::Command::new(&proton)
            .arg("run")
            .arg(&exe)
            .env("STEAM_COMPAT_DATA_PATH", &compat_prefix)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &steam_root)
            .current_dir(cwd)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to launch via Proton ({proton:?}): {e}"))
    }
}
```

- [ ] **Step 4: Build — no errors**

```bash
cd evolve-launcher/src-tauri && cargo build 2>&1 | grep "^error" | head -10
```

Expected: no errors.

- [ ] **Step 5: Run tests**

```bash
cargo test 2>&1 | tail -5
```

Expected: all tests PASS (nat tests still compile since nat.rs stays, just unused from launch_game).

- [ ] **Step 6: Commit**

```bash
git add evolve-launcher/src-tauri/src/commands.rs
git commit -m "feat(launcher): donor pre-flight in launch_game, check_donor_game command, remove proxy"
```

---

## Task 6: Update `lib.rs` — remove ProxyState and dead commands

**Files:**
- Modify: `evolve-launcher/src-tauri/src/lib.rs`

- [ ] **Step 1: Update `lib.rs`**

Replace the current content with:

```rust
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Build — no errors**

```bash
cd evolve-launcher/src-tauri && cargo build 2>&1 | grep "^error" | head -10
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add evolve-launcher/src-tauri/src/lib.rs
git commit -m "chore(launcher): remove ProxyState and get_nat_type from invoke handler"
```

---

## Task 7: New `SteamSetupView.svelte` — donor game flow

**Files:**
- Modify: `evolve-launcher/src/lib/SteamSetupView.svelte`
- Modify: `evolve-launcher/src/types.ts`

**Context:** The current view picks a Goldberg SteamID. The new view checks donor game ownership and guides the player to install it if missing.

- [ ] **Step 1: Add `DonorStatus` to `types.ts`**

In `evolve-launcher/src/types.ts`, add:

```typescript
export interface DonorStatus {
  installed: boolean;
  dll_ready: boolean;
  donor_name: string;
  donor_app_id: number;
}
```

Remove `NatInfo` interface (no longer used).

- [ ] **Step 2: Rewrite `SteamSetupView.svelte`**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { DonorStatus } from '../types';

  let { onDone }: { onDone: () => void } = $props();

  type Phase = 'checking' | 'need-donor' | 'copying' | 'ready' | 'no-steam' | 'error';

  let phase = $state<Phase>('checking');
  let donorName = $state('');
  let donorAppId = $state(0);
  let errorMsg = $state('');

  async function check() {
    phase = 'checking';
    try {
      const status = await invoke<DonorStatus>('check_donor_game');
      donorName = status.donor_name;
      donorAppId = status.donor_app_id;
      if (!status.installed) {
        phase = 'need-donor';
      } else if (!status.dll_ready) {
        phase = 'copying';
        await invoke('launch_game').catch(() => {}); // triggers DLL copy via pre-flight
        phase = 'ready';
      } else {
        phase = 'ready';
      }
    } catch (e) {
      const msg = String(e);
      if (msg.includes('Steam not found')) {
        phase = 'no-steam';
      } else {
        errorMsg = msg;
        phase = 'error';
      }
    }
  }

  function openDonorStore() {
    invoke('open_steam_store', { appId: donorAppId }).catch(() => {});
    // poll until installed
    const interval = setInterval(async () => {
      const status = await invoke<DonorStatus>('check_donor_game');
      if (status.installed) {
        clearInterval(interval);
        check();
      }
    }, 3000);
  }

  onMount(check);
</script>

<div class="steam-setup">
  <div class="title">Steam Setup</div>

  {#if phase === 'checking' || phase === 'copying'}
    <div class="body">
      <div class="spinner"></div>
      <span class="hint">{phase === 'copying' ? 'Preparing Steam files…' : 'Checking Steam…'}</span>
    </div>

  {:else if phase === 'need-donor'}
    <div class="body">
      <p class="subtitle">
        To enable Steam multiplayer (SDR, overlay, invites), you need one free Steam game installed:
        <strong>{donorName}</strong>.
      </p>
      <button class="primary-btn" onclick={openDonorStore}>
        Add to Steam Library (Free)
      </button>
      <span class="hint">Click above — it's free. Once installed, this step completes automatically.</span>
    </div>

  {:else if phase === 'ready'}
    <div class="body">
      <div class="check-icon">✓</div>
      <p class="subtitle">Steam is ready. You'll get overlay, invites, and relay networking.</p>
    </div>

  {:else if phase === 'no-steam'}
    <div class="body">
      <p class="subtitle">
        Steam was not found. Install Steam and log in to enable multiplayer features.
      </p>
    </div>

  {:else if phase === 'error'}
    <div class="body">
      <p class="error-msg">{errorMsg}</p>
      <button class="primary-btn" onclick={check}>Retry</button>
    </div>
  {/if}

  <button class="skip-btn" onclick={onDone}>
    {phase === 'ready' ? 'Continue' : 'Skip'}
  </button>
</div>

<style>
  .steam-setup {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    padding: 32px;
    gap: 24px;
    color: #fff;
    background: #0f0f12;
  }

  .title {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .body {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    width: 100%;
    max-width: 420px;
  }

  .subtitle {
    text-align: center;
    color: #aaa;
    font-size: 14px;
    line-height: 1.6;
    margin: 0;
  }

  .primary-btn {
    background: #4ade80;
    border: none;
    color: #000;
    padding: 10px 28px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .primary-btn:hover { opacity: 0.85; }

  .check-icon {
    font-size: 40px;
    color: #4ade80;
  }

  .hint {
    font-size: 12px;
    color: #666;
    text-align: center;
  }

  .error-msg {
    color: #f87171;
    font-size: 13px;
    text-align: center;
  }

  .skip-btn {
    background: transparent;
    border: 1px solid #333;
    color: #888;
    padding: 8px 28px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .skip-btn:hover { border-color: #888; color: #ccc; }

  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid #2e2e38;
    border-top-color: #4ade80;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }
</style>
```

- [ ] **Step 3: Type-check**

```bash
cd evolve-launcher && npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add evolve-launcher/src/lib/SteamSetupView.svelte evolve-launcher/src/types.ts
git commit -m "feat(ui): donor game setup flow replaces Goldberg account picker"
```

---

## Task 8: UI cleanup — remove NAT indicator from `Main.svelte`

**Files:**
- Modify: `evolve-launcher/src/lib/Main.svelte`

- [ ] **Step 1: Remove NAT indicator**

In `Main.svelte`:
- Delete `import type { NatInfo } from '../types';` (if present after types.ts update)
- Delete `let natInfo = $state<NatInfo | null>(null);`
- Delete the `invoke<NatInfo>('get_nat_type')...` call in `onMount`
- Delete the `{#if natInfo !== null}` NAT indicator block from the template
- Delete the `.nat-indicator`, `.nat-dot`, `.nat-dot.nat-direct`, `.nat-dot.nat-relay`, `.nat-label` CSS rules

- [ ] **Step 2: Type-check**

```bash
cd evolve-launcher && npx tsc --noEmit 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add evolve-launcher/src/lib/Main.svelte
git commit -m "chore(ui): remove NAT indicator (replaced by Steam SDR)"
```

---

## Task 9: Server cleanup — retire relay and punch handler

**Files:**
- Delete: `evolve-server/internal/handler/punch.go`
- Delete: `evolve-server/internal/handler/punch_test.go`
- Modify: `evolve-server/cmd/server/router.go`
- Modify: `evolve-server/cmd/server/main.go`

**Context:** The relay and `/peers/register` endpoint were for Goldberg peer discovery. With SDR, they are unused. The relay package (`evolve-server/internal/relay/`) stays — do not delete it.

- [ ] **Step 1: Delete punch handler files**

```bash
rm evolve-server/internal/handler/punch.go
rm evolve-server/internal/handler/punch_test.go
```

- [ ] **Step 2: Update `router.go`**

Remove the relay parameter and the `/peers/register` route. Replace `buildRouterWithDeps` signature:

```go
func buildRouterWithDeps(cfg config.Config, pool *sql.DB) *gin.Engine {
    r := gin.New()
    r.Use(gin.Recovery())

    auth := r.Group("/", middleware.Auth(cfg.JWTSecret))
    auth.GET("/ping", func(c *gin.Context) { c.JSON(200, gin.H{"ok": true}) })

    // No /peers/register route — SDR handles peer connectivity
    return r
}
```

- [ ] **Step 3: Update `main.go`**

Remove relay startup. Replace the relevant section:

```go
// Remove: rel := relay.New()
// Remove: go func() { rel.Run(":" + cfg.RelayPort) }()
// Remove: r := buildRouterWithDeps(cfg, pool, rel)
// Add:
r := buildRouterWithDeps(cfg, pool)
```

Also remove the `relay` import from `main.go` if it's no longer referenced.

- [ ] **Step 4: Build and test**

```bash
cd evolve-server
go build ./... 2>&1
go test ./... 2>&1 | tail -10
```

Expected: clean build, all tests pass. (relay package tests still pass — the package itself is untouched.)

- [ ] **Step 5: Commit**

```bash
git add evolve-server/
git commit -m "chore(server): retire relay + punch handler — SDR replaces Goldberg peer discovery"
```

---

## Post-Implementation Checklist

- [ ] Build `evolve_shim.dll` and upload to CDN alongside game files
- [ ] Update CDN manifest: add `evolve_shim.dll`, remove `GoldbergNewEvolveEmu.dll` / `EvolveLegacyRebornServer.dll` / `GameOverlayRenderer64.dll`
- [ ] Test end-to-end on Windows: launch game, verify overlay (Shift+Tab), verify DLC access, verify friend invite sends
- [ ] Test on Linux/Proton: verify shim intercepts still fire through Proton's Wine layer
- [ ] Verify `steam_appid.txt` contains donor App ID (not 273350) in `bin64_SteamRetail/`
- [ ] Confirm relay package compiles but `relay.Run()` is not called from main
