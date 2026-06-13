# Evolve Revival: Server + Launcher Design

## Goal

Replace the local RiceFix/Pinenut emulator with a centralized community server that all players connect to, paired with a one-click launcher that removes all manual setup steps.

## Architecture

Three components ship together:

1. **evolve-server** — Go + gin HTTP server running on a VPS, emulates the 2K Kando API the game already calls. PostgreSQL for persistence. Adds a `/peers` API for VPN-free player discovery.
2. **evolve-launcher** — Tauri + React desktop app. Patches the game on first run, shows server health, has a Play button. Auto-updates from the server.
3. **evolve-goldberg** — Fork of open-source Goldberg Steam emulator with LAN broadcast replaced by HTTP peer lookup against evolve-server. Ships as `GoldbergNewEvolveEmu.dll` bundled in the launcher.

---

## evolve-server

### Stack
- Go 1.26+, `gin-gonic/gin`, `lib/pq` (PostgreSQL driver)
- PostgreSQL 16 (hosted on same VPS)
- Deployed as a single binary behind nginx (TLS termination)

### API Surface

All endpoints are under the prefix the game expects (`*.my.2k.com` equivalent, configured via `EvolveLogging.ini`).

| Endpoint | Method | Purpose |
|---|---|---|
| `/doorman` | GET | Session config / auth gateway |
| `/singlesignon/auths.logon` | POST | SSO login, returns session token |
| `/entitlements/checkAppOwnership` | GET | Returns owned = true |
| `/entitlements/getFirstPartyMapping` | GET | Stub |
| `/entitlements/getMapping` | GET | Stub |
| `/storage/1/:dataset` | GET/POST/PUT/DELETE | 5 player datasets |
| `/stats/configs.generate` | GET | Returns empty stat groups |
| `/grants/find` | GET | Returns entitlement grants |
| `/sessions-heartbeat` | POST | Keepalive for active sessions |
| `/queue/waittime` | GET | Returns fake queue time (0) |
| `/evolve/event` | POST | Analytics stub (200 OK) |
| `/build_config` | GET | Launcher/DLL version manifest |
| `/status` | GET | Health check: online players, server uptime |
| `/peers/register` | POST | Goldberg registers player IP + lobby ID |
| `/peers/:lobbyId` | GET | Returns peer list for a lobby |

### Storage Datasets (PostgreSQL tables)

| Dataset | UUID | Table |
|---|---|---|
| sessions | e9c21d966612393f9514896e4080f0c9 | `sessions` |
| playerProperties | e4e9ba4c5d6630df11d8ce3683ec1fde | `player_properties` |
| playerUnlocks | 5ab57ab47a39339c1023a75dc99d2110 | `player_unlocks` |
| replays | b06d4d28b6467691823e4ac44aebe6d0 | `replays` |
| replayOwners | e754746f6e4ef8c73e734bff7305f450 | `replay_owners` |

### Peer Registration

- `POST /peers/register` body: `{ lobbyId, playerId, ip, port }`
- `GET /peers/:lobbyId` returns: `[{ playerId, ip, port }]`
- Peers auto-expire after 5 minutes (cron or TTL column)
- This is what patched Goldberg calls instead of UDP LAN broadcast

### Database Schema (summary)

```sql
players (id TEXT PK, display_name TEXT, created_at TIMESTAMP)
player_properties (player_id TEXT, key TEXT, value JSONB, updated_at TIMESTAMP)
player_unlocks (player_id TEXT, data JSONB, updated_at TIMESTAMP)
sessions (id TEXT PK, player_id TEXT, data JSONB, created_at TIMESTAMP, expires_at TIMESTAMP)
replays (id TEXT PK, player_id TEXT, data JSONB, created_at TIMESTAMP)
replay_owners (replay_id TEXT, player_id TEXT)
peers (lobby_id TEXT, player_id TEXT, ip TEXT, port INT, registered_at TIMESTAMP)
```

---

## evolve-launcher

### Stack
- Tauri 2 (Rust backend, WebView2 frontend)
- React + TypeScript frontend (Vite)
- Ships as a single Windows `.exe` installer

### First-Run Flow

1. Detect Evolve install path (check Steam registry key, fallback to manual selection)
2. Patch `EvolveLogging.ini`: set `server_domain`, `server_port`, `use_internal_server = false`
3. Drop patched `GoldbergNewEvolveEmu.dll` into the game directory
4. Write a marker file so first-run only runs once

### Main UI

- **Status bar**: green/red dot, "X players online" (polls `/status` every 30s)
- **Play button**: launches `Evolve.exe` via Steam or direct exe
- **Version info**: launcher version, DLL version, last updated

### Auto-Updater

- On launch, calls `/build_config` — returns `{ launcherVersion, dllVersion, dllUrl, launcherUrl }`
- If DLL version is newer: downloads + replaces `GoldbergNewEvolveEmu.dll` silently
- If launcher version is newer: prompts user, downloads installer, relaunches

---

## evolve-goldberg

### Approach

Fork `https://gitlab.com/Mr_Goldberg/goldberg_emulator` (open source, MIT-ish).

### Change

Replace `Networking::send_lobby_broadcast()` / peer discovery code with:
```
GET http://<server>/peers/<lobbyId>
→ parse JSON peer list
→ connect directly to each IP:port via existing Steam P2P socket code
```

Register on lobby join:
```
POST http://<server>/peers/register
body: { lobbyId, playerId, ip, port }
```

Compiled as a Windows DLL (`GoldbergNewEvolveEmu.dll`), bundled with launcher.

---

## Deployment

- Single VPS (2 vCPU / 4 GB RAM is sufficient for community scale)
- `evolve-server` binary + PostgreSQL + nginx
- DNS: point `server.evolve-community.net` (or similar) at VPS IP
- Players set `server_domain = server.evolve-community.net` in `EvolveLogging.ini` (done automatically by launcher)
