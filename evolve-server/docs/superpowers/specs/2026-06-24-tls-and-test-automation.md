# evolve-server: TLS + Headless Test Automation
**Date:** 2026-06-24
**Status:** Approved

---

## 1. Goals

- Make evolve-server answer on HTTPS `127.0.0.1:443` so the game's cURL can connect
- Fill the two route gaps that block the main menu from loading
- Give Claude (and CI) a single `make test-game` command that launches Evolve via Proton, waits for the main menu, and exits 0/1 — no human interaction required

---

## 2. Context

`EvolveLogging.ini` points the game at `server_domain = 127.0.0.1`, `server_port = 443`, `use_internal_server = false`. The game's cURL validates TLS against `ca-bundle.crt` (PaiEnNate Fake Root CA, valid until 2123). The evolve-server currently runs HTTP on 8080 → `cURL error 7` on every boot.

All tests pass. Routes are substantially complete. Three things are missing:

| Gap | Symptom |
|---|---|
| No TLS on 443 | cURL error 7, game never reaches doorman |
| `/:platformId/*` not routed | 404 on `/8112690398592087182/auth/two_k` and `/profile/...` after SSO |
| `/evolve/config/:id` not routed | 404 right at main menu load |

**Test pass signal:** `/queue/waittime?platformId=2&regionId=IAD` appearing in the server request log. This fires from the main menu's match-queue polling loop with a real region ID — it never fires during boot or auth. It is the unambiguous indicator that the game has fully loaded the main menu.

---

## 3. TLS Setup

### 3.1 Cert generation (`make cert`)

One-time setup. Requires `openssl` on PATH.

**Inputs:**
- `certs/ca.key` — PaiEnNate RSA-2048 CA private key (extracted from `EvolveLegacyRebornServer.dll` in RiceFix; stored locally, never committed — gitignored)
- `certs/ca.crt` — PaiEnNate CA certificate (copied once from `ca-bundle.crt` in the game dir; safe to commit, already public in RiceFix)

**Outputs:** `certs/server.crt` + `certs/server.key` — a leaf cert for `127.0.0.1`, signed by the PaiEnNate CA, valid 3650 days.

```makefile
cert:
    openssl genrsa -out certs/server.key 2048
    openssl req -new -key certs/server.key \
        -subj "/CN=127.0.0.1/O=EvolveRevival" \
        -out certs/server.csr
    openssl x509 -req -in certs/server.csr \
        -CA certs/ca.crt -CAkey certs/ca.key -CAcreateserial \
        -out certs/server.crt -days 3650 \
        -extfile <(printf "subjectAltName=IP:127.0.0.1")
    rm certs/server.csr
```

`certs/` is gitignored. `certs/ca.crt` is the CA cert (safe to commit; public). `certs/ca.key` is the CA private key (gitignored; extracted once and stored locally).

### 3.2 Server changes

`config.go` adds two fields:
```go
CertFile string  // CERT_FILE env, default "certs/server.crt"
KeyFile  string  // KEY_FILE env, default "certs/server.key"
```

`main.go` switches from `r.Run` to:
```go
if cfg.CertFile != "" {
    r.RunTLS(":"+cfg.Port, cfg.CertFile, cfg.KeyFile)
} else {
    r.Run(":"+cfg.Port)
}
```

`PORT` defaults to `443` (changed from `8080`). Unit tests use `httptest.NewServer` and never bind a real port, so this default doesn't affect them.

### 3.3 Port 443 without root (`make cap`)

```makefile
cap: build
    sudo setcap cap_net_bind_service=+ep ./bin/evolve-server
```

Required once per build. `make run` calls `make cap` first.

---

## 4. Missing Routes

### 4.1 Platform ID namespace

The game calls `/:platformId/auth/two_k` and `/:platformId/profile/get_by_platform_account_id` where `platformId` is a large integer (`8112690398592087182`). These are post-SSO identity calls. RiceFix returns 200 for all of them ("Something I forgot to emulate but seems okay").

Add to router:
```go
r.Any("/:platformId/auth/*subpath", stubs.Stub200)
r.Any("/:platformId/profile/*subpath", stubs.Stub200)
```

These must be registered before the catch-all (see 4.2) to avoid gin routing conflicts.

### 4.2 Evolve config + catch-all

`/evolve/config/:id` fires right at main menu load alongside `/evolve/event` (already stubbed). Add:
```go
r.Any("/evolve/config/*path", stubs.Stub200)
```

And add gin's no-route handler as a final safety net matching RiceFix's fallthrough behavior:
```go
r.NoRoute(stubs.Stub200)
```

---

## 5. Test Automation

### 5.1 Script: `scripts/test-game.sh`

```
GAME_DIR   path to EvolveFilesLegacy (default: /run/media/navitank/Untitled/Projects/EvolveFilesLegacy)
PROTON     path to proton binary     (default: ~/.steam/steam/steamapps/common/Proton 8.0/proton)
TIMEOUT    seconds to wait           (default: 90)
```

**Steps:**
1. `make build` — rebuild binary
2. `make cap` — setcap for port 443
3. Truncate `$GAME_DIR/kandoC.log` and `$GAME_DIR/EVOLVE_LOG.txt`
4. Start server: `PORT=443 CERT_FILE=certs/server.crt KEY_FILE=certs/server.key ./bin/evolve-server > /tmp/es.log 2>&1 &` — save PID
5. Health-wait: `until curl -sk https://127.0.0.1/status | grep -q ok; do sleep 1; done` — abort if >10s
6. Launch game: `STEAM_COMPAT_DATA_PATH=... STEAM_COMPAT_CLIENT_INSTALL_PATH=~/.steam/steam "$PROTON" run "$GAME_DIR/bin64_SteamRetail/Evolve.exe"` — save PID
7. Poll `/tmp/es.log` every 2s for `queue/waittime` with non-empty `regionId` param
8. On match: print `✓ main menu reached (${elapsed}s)`, kill both PIDs, exit 0
9. On timeout: print FAIL header, tail last 20 lines of `/tmp/es.log` and `kandoC.log`, kill both PIDs, exit 1

**Cleanup:** `trap` on EXIT kills both PIDs to prevent orphaned Evolve processes.

### 5.2 Makefile targets

```makefile
test-game:
    @bash scripts/test-game.sh

run: cap
    PORT=443 CERT_FILE=certs/server.crt KEY_FILE=certs/server.key ./bin/evolve-server

cap: build
    sudo setcap cap_net_bind_service=+ep ./bin/evolve-server

cert:
    # ... openssl commands above

build:
    go build -o bin/evolve-server ./cmd/server

test:
    go test ./...
```

---

## 6. What Is Not Changing

- No changes to database schema, storage handlers, SSO logic, or relay
- No changes to `EvolveLogging.ini` — `use_internal_server = false`, port 443 already set
- No changes to Goldberg config or steam_appid.txt (already set to 480)
- `make test` (unit tests) continues to work without TLS or Proton

---

## 7. Open Questions

None blocking this implementation. The `sessions-heartbeat` route (mentioned in earlier design spec) was superseded by `/queue/waittime` as the pass signal — BYPINENUT confirms heartbeat is not visible in RiceFix logs but queue polling is.
