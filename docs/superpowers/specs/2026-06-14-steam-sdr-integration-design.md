# Steam SDR Integration Design

## Goal

Replace Goldberg's emulated networking with real Steamworks to gain Steam Datagram Relay (IP privacy, geo-optimal global ping), Steam overlay, and friend invite support — without requiring players to own Evolve on Steam, by authenticating under a free donor game's App ID.

## Architecture

A thin proxy DLL (`evolve_shim.dll`) sits between `Evolve.exe` and the real `steam_api64.dll`. The game loads it via the existing `EvolveLogging.ini` `dll_path` hook. The shim forwards all Steamworks calls to the real SDK except three (DLC unlock and AppID reporting). Everything Steam provides — SDR relay routing, overlay injection, friend invites, lobby discovery — flows through the real SDK automatically once the shim is in place.

The donor game is any free Steam game with SDR enabled. All players own it (free). Steam authenticates them under the donor App ID, which is what SDR uses for relay session negotiation. Game-internal logic still sees App ID 273350 via the shim intercept.

`EvolveLegacyRebornServer.dll` (Pinenut local server) is removed — replaced by our VPS. `GameOverlayRenderer64.dll` (Goldberg overlay) is removed — real Steam handles overlay injection. `GoldbergNewEvolveEmu.dll` is replaced by `evolve_shim.dll`. The STUN proxy (`nat.rs`) and UDP relay (`relay.go`) are retired for game traffic — SDR handles peer transport.

**Tech Stack:** C++ (shim DLL), Rust/Tauri (launcher), existing Go VPS backend

---

## Component 1: Donor Game

### Selection criteria

- Free on Steam (zero cost, anyone can add to library)
- SDR enabled by Valve (`ISteamNetworkingSockets` relay infrastructure active for that App ID)
- Uses P2P sessions (`ISteamNetworkingSockets`, not just `ISteamMatchmaking` server browser)
- Stable — not likely to be removed or have SDR revoked

The specific App ID is stored as a server-side config value (`donor_app_id`) fetched from our VPS at launcher startup. If the donor ever needs to change, a server-side config push is sufficient — no client update required.

### Ownership verification

The launcher checks donor ownership by scanning Steam's `appmanifest_<donorAppId>.acf` files under the Steam `steamapps/` directory. No API call needed; works while Steam is loading.

If not owned: launcher opens `steam://store/<donorAppId>` — one click, free, instant.

### `steam_api64.dll` sourcing

Once the donor is confirmed installed, the launcher copies `steam_api64.dll` from the donor game's install directory into `Bin64_SteamRetail/`. This is the real Steamworks DLL Valve ships with the donor game. If the donor is later uninstalled and the DLL goes missing, the launcher detects this at pre-flight and re-copies or warns the user.

---

## Component 2: `evolve_shim.dll`

A C++ proxy DLL (~200 lines) that exports the same flat `SteamAPI_*` symbol table as `steam_api64.dll`. At load time it calls `LoadLibrary("steam_api64.dll")` and resolves each export via `GetProcAddress`. All calls forward through except:

| Intercepted call | Shim behavior |
|---|---|
| `ISteamApps::BIsDlcInstalled(dlcId)` | Always returns `true` |
| `ISteamUtils::GetAppID()` | Returns `273350` |
| `ISteamApps::GetAppID()` | Returns `273350` |

The AppID intercept preserves game-internal logic (server URL construction, save paths, hardcoded ID checks) while leaving Steam session auth and SDR negotiation unaffected — those happen inside `steam_api64.dll` beneath the intercept layer using the donor App ID.

Immediately after `SteamAPI_Init()` succeeds, the shim calls:
```cpp
SteamFriends()->SetRichPresence("status", "Playing Evolve Stage 2");
```
so friends see the correct game name in their activity feed regardless of the donor game title.

**Distribution:** `evolve_shim.dll` is a versioned CDN file in the game manifest. Launcher updates it via the standard remote patch flow like any other game file.

**Linux/Proton:** Proton intercepts `steam_api64.dll` loading and bridges to the Linux Steam client at a layer below the flat API surface. The shim sits above that bridge — intercepts still fire, SDR still routes through Valve relays, overlay still injects.

---

## Component 3: Launcher Changes

### Installation flow

1. Download game files from CDN (manifest includes `evolve_shim.dll`; excludes `GoldbergNewEvolveEmu.dll`, `EvolveLegacyRebornServer.dll`, `GameOverlayRenderer64.dll`)
2. Apply patches (unchanged)
3. **Steam setup screen** (replaces current `SteamSetupView`):
   - Detect Steam installation (Windows registry / `~/.steam` on Linux)
   - If absent: show download prompt, block until installed
   - Scan for donor ACF file; if missing, open donor store page and poll until owned
   - Copy `steam_api64.dll` from donor install dir → `Bin64_SteamRetail/`
4. Write `EvolveLogging.ini`:
   ```ini
   [server]
   server_domain = <vps-host>
   server_port = 443
   use_internal_server = false

   [steam]
   emu_steam = false
   dll_path = evolve_shim.dll
   ```
5. Write donor App ID to `steam_settings/steam_appid.txt`
6. Install complete

### Pre-flight checks (every launch)

Before spawning `Evolve.exe`:

1. Verify Steam process is running — if not, launch Steam and wait up to 10 seconds
2. Verify `Bin64_SteamRetail/steam_api64.dll` exists — if missing, re-copy from donor install dir
3. Verify `EvolveLogging.ini` server address matches current VPS config — rewrite if stale
4. No STUN proxy started (SDR replaces it)

### Remote patching

Unchanged from current implementation. The CDN manifest versions `evolve_shim.dll` as a game file. On next launcher start after a shim update, the build mismatch triggers download and in-place replacement. The VPS server address in `EvolveLogging.ini` is also manifest-versioned, allowing VPS migration without a client update.

### `get_nat_type` command

Removed from the Tauri invoke handler and the UI. NAT type is no longer meaningful once SDR handles transport. The `nat.rs` module stays in the codebase but `start_proxy` is no longer called at launch. The UI NAT indicator (`Main.svelte`) is replaced with a Steam connection status indicator showing relay vs. direct, sourced from `ISteamNetworkingUtils::GetRelayNetworkStatus()` via a new `get_steam_connection_status` Tauri command.

---

## Component 4: Steam Integration

### SDR

All `ISteamNetworkingSockets` calls route through Valve's global relay mesh automatically. Players never exchange real IPs — Valve relay addresses only. Relay selection is geo-optimal per player pair. No VPS relay nodes needed for game traffic.

### Overlay

Steam injects its overlay into any process that calls `SteamAPI_Init()` with a valid session. Shift+Tab, friend list, and notifications work in-game with no additional configuration.

### Invites

**In-game invite:** Game calls `ISteamFriends::InviteUserToGame(steamId, connectString)`. Friend receives a Steam notification with "Join Game". Clicking it launches Evolve via the launcher with the connect string passed as a command-line argument. Launcher forwards it to the game process on startup.

**Profile join:** Friend sees activity, clicks "Join Game" on Steam profile — same flow.

Connect string format is Evolve's existing multiplayer join format, handled internally by the game.

### Lobby discovery

`ISteamMatchmaking` lobby creation and search operate under the donor App ID. Two players using the same donor App ID find each other's lobbies through Steam's matchmaking without VPS involvement. Our VPS handles progression, stats, and server-authoritative data only.

---

## What Is Retired

| Component | Retired because |
|---|---|
| `GoldbergNewEvolveEmu.dll` | Replaced by `evolve_shim.dll` + real Steamworks |
| `EvolveLegacyRebornServer.dll` | Replaced by VPS (already done) |
| `GameOverlayRenderer64.dll` | Real Steam overlay replaces it |
| `nat.rs` STUN proxy | SDR handles peer transport |
| `relay.go` UDP relay (game traffic path) | SDR handles peer transport |
| NAT indicator UI | Replaced by Steam connection status |
| `/peers/register` HTTP endpoint | Goldberg peer discovery no longer used |

The relay server's named peer registry and punch handler can be removed. The Go relay package itself may be kept if future use cases emerge, but it carries no active load.

---

## Risk and Mitigations

**Donor game removed or SDR disabled:** Launcher fetches donor App ID from server config. Switching donors requires a server-side config change only — no client update. Launcher shows a clear error if donor ownership check fails, with a link to the new donor's store page.

**Valve detects spoofing:** No Evolve-specific mitigation possible. The risk is the same as any game using the donor App ID spoof pattern. Using a less prominent donor (not a Valve first-party title) reduces visibility.

**`steam_api64.dll` version mismatch:** The shim uses `GetProcAddress` at runtime, so it works with any version of `steam_api64.dll` as long as the flat API symbols exist (they have been stable since 2013). No version pinning needed.

**Linux/Proton compatibility:** Proton's Steam bridge operates below the shim layer. Tested assumption: shim intercepts fire before Proton's translation layer, preserving DLC and AppID behavior. Validate during implementation with a Proton test run.
