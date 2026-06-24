# TLS + Headless Test Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make evolve-server answer on HTTPS 127.0.0.1:443 so Evolve can authenticate, and provide a single `make test-game` command that drives the game via Proton and passes when the main menu loads.

**Architecture:** Three independent layers — route gaps filled via gin's NoRoute handler, TLS added to the gin engine with a pre-generated cert signed by the PaiEnNate fake CA, and a bash test script that starts the server, launches Evolve via Proton, and watches the server log for the main-menu queue-poll signal.

**Tech Stack:** Go 1.26.3, gin v1.10.0, openssl (cert generation), bash (test script), Proton 8.0 (game launch)

## Global Constraints

- All Go code goes in `github.com/evolve-revival/evolve-server` module
- Repo root: `/run/media/navitank/Untitled/Projects/evolve-revival/evolve-server`
- Game dir: `/run/media/navitank/Untitled/Projects/EvolveFilesLegacy`
- Proton binary: `~/.steam/steam/steamapps/common/Proton 8.0/proton`
- `make test` must continue to pass with zero env vars set
- `certs/ca.key` and `certs/server.{crt,key}` are never committed (gitignored); `certs/ca.crt` is committed
- Port 443 requires `cap_net_bind_service` via `sudo setcap` — do not run server as root

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `cmd/server/router.go` | Modify | Add `/evolve/config/*path` route + `r.NoRoute` catch-all |
| `cmd/server/routes_test.go` | Create | Unit tests for new routes (no DB, no auth token needed) |
| `internal/config/config.go` | Modify | Add `CertFile`, `KeyFile` fields; change `PORT` default to `443` |
| `internal/config/config_test.go` | Modify | Add tests for new fields and default values |
| `cmd/server/main.go` | Modify | Branch on `CertFile`: `RunTLS` vs `Run` |
| `certs/ca.crt` | Create | PaiEnNate CA cert — copied from game dir, committed |
| `certs/.gitignore` | Create | Ignores `ca.key`, `server.crt`, `server.key`, `*.srl` |
| `Makefile` | Modify | Add `cert`, `cap`, `test-game` targets; update `run` |
| `scripts/test-game.sh` | Create | Full integration test: server → game → main-menu detection |

---

## Task 1: Fill Route Gaps

**Files:**
- Modify: `cmd/server/router.go`
- Create: `cmd/server/routes_test.go`

**Interfaces:**
- Consumes: `handler.NewStubsHandler()` → `*StubsHandler` with `Stub200(c *gin.Context)` method (already exists in `internal/handler/stubs.go`)
- Produces: gin router now returns 200 for any unregistered path

- [ ] **Step 1: Create the failing test**

Create `cmd/server/routes_test.go`:

```go
package main

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/gin-gonic/gin"
)

func TestCatchAllRoutes(t *testing.T) {
	gin.SetMode(gin.TestMode)
	// nil pool is safe: auth middleware is permissive, stub routes never touch DB
	r := buildRouterWithDeps(config.Config{ServerHost: "localhost"}, nil)

	cases := []struct {
		method string
		path   string
	}{
		{"GET", "/8112690398592087182/auth/two_k"},
		{"GET", "/8112690398592087182/profile/get_by_platform_account_id"},
		{"GET", "/evolve/config/2357d522a223d8d57b05071505274b6b"},
		{"POST", "/telemetry/1"},
		{"GET", "/completely/unknown/route"},
	}

	for _, tc := range cases {
		t.Run(tc.method+" "+tc.path, func(t *testing.T) {
			w := httptest.NewRecorder()
			req := httptest.NewRequest(tc.method, tc.path, nil)
			r.ServeHTTP(w, req)
			if w.Code != http.StatusOK {
				t.Fatalf("want 200, got %d for %s %s", w.Code, tc.method, tc.path)
			}
		})
	}
}
```

- [ ] **Step 2: Run test — confirm it fails**

```bash
cd /run/media/navitank/Untitled/Projects/evolve-revival/evolve-server
go test ./cmd/server/ -run TestCatchAllRoutes -v
```

Expected: `FAIL` — some cases return 404.

- [ ] **Step 3: Add routes to router.go**

In `cmd/server/router.go`, add two lines immediately before `return r`:

```go
	r.Any("/evolve/config/*path", stubs.Stub200)
	r.NoRoute(stubs.Stub200)

	return r
}
```

Full context around the insertion point (end of `buildRouterWithDeps`):

```go
	// Wildcard stubs
	r.Any("/apps/1/*path", stubs.Stub200)
	r.Any("/content/1/*path", stubs.Stub200)
	r.Any("/storefront/1/*path", stubs.Stub200)
	r.Any("/sessions/1/*path", stubs.Stub200)
	r.Any("/news/1/*path", stubs.Stub200)

	r.Any("/evolve/config/*path", stubs.Stub200)
	r.NoRoute(stubs.Stub200)

	return r
}
```

- [ ] **Step 4: Run test — confirm it passes**

```bash
go test ./cmd/server/ -run TestCatchAllRoutes -v
```

Expected: all 5 sub-tests `PASS`.

- [ ] **Step 5: Run full test suite — confirm nothing broke**

```bash
go test ./...
```

Expected: all packages `ok`.

- [ ] **Step 6: Commit**

```bash
git add cmd/server/router.go cmd/server/routes_test.go
git commit -m "feat: add /evolve/config/* route and NoRoute catch-all (stub 200)"
```

---

## Task 2: TLS — Config, Certs, Main, Makefile

**Files:**
- Modify: `internal/config/config.go`
- Modify: `internal/config/config_test.go`
- Modify: `cmd/server/main.go`
- Create: `certs/ca.crt`
- Create: `certs/.gitignore`
- Modify: `Makefile`

**Interfaces:**
- Produces: `config.Config.CertFile string`, `config.Config.KeyFile string` — consumed by `cmd/server/main.go`
- Produces: `make cert` generates `certs/server.crt` + `certs/server.key`
- Produces: `make cap` grants port-443 capability on the built binary
- Produces: `make run` starts HTTPS on 443

### 2a — Config fields

- [ ] **Step 1: Write failing config tests**

Add to `internal/config/config_test.go` (keep all existing tests):

```go
func TestLoadCertDefaults(t *testing.T) {
	os.Unsetenv("CERT_FILE")
	os.Unsetenv("KEY_FILE")
	os.Unsetenv("PORT")
	cfg := Load()
	if cfg.CertFile != "certs/server.crt" {
		t.Errorf("CertFile default: want certs/server.crt, got %q", cfg.CertFile)
	}
	if cfg.KeyFile != "certs/server.key" {
		t.Errorf("KeyFile default: want certs/server.key, got %q", cfg.KeyFile)
	}
	if cfg.Port != "443" {
		t.Errorf("Port default: want 443, got %q", cfg.Port)
	}
}

func TestLoadCertOverride(t *testing.T) {
	t.Setenv("CERT_FILE", "/tmp/test.crt")
	t.Setenv("KEY_FILE", "/tmp/test.key")
	cfg := Load()
	if cfg.CertFile != "/tmp/test.crt" {
		t.Errorf("CertFile override: want /tmp/test.crt, got %q", cfg.CertFile)
	}
	if cfg.KeyFile != "/tmp/test.key" {
		t.Errorf("KeyFile override: want /tmp/test.key, got %q", cfg.KeyFile)
	}
}
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
go test ./internal/config/ -run "TestLoadCert" -v
```

Expected: `FAIL — cfg.CertFile is empty string`.

- [ ] **Step 3: Update config.go**

Replace the entire `Config` struct and `Load()` function in `internal/config/config.go`:

```go
type Config struct {
	Port       string
	DBDSN      string
	ServerHost string
	RelayPort  string
	CertFile   string
	KeyFile    string
}

func Load() Config {
	return Config{
		Port:       getenv("PORT", "443"),
		DBDSN:      getenv("DATABASE_URL", "postgres://evolve:evolve@localhost/evolve?sslmode=disable"),
		ServerHost: getenv("SERVER_HOST", "localhost:443"),
		RelayPort:  getenv("RELAY_PORT", "47584"),
		CertFile:   getenv("CERT_FILE", "certs/server.crt"),
		KeyFile:    getenv("KEY_FILE", "certs/server.key"),
	}
}
```

- [ ] **Step 4: Run config tests — confirm they pass**

```bash
go test ./internal/config/ -v
```

Expected: all tests `PASS`.

### 2b — main.go TLS branch

- [ ] **Step 5: Update main.go**

Replace the entire `cmd/server/main.go` with:

```go
package main

import (
	"log"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/db"
)

func main() {
	cfg := config.Load()

	pool, err := db.Open(cfg.DBDSN)
	if err != nil {
		log.Fatalf("db: %v", err)
	}
	defer pool.Close()

	if err := db.Migrate(pool); err != nil {
		log.Fatalf("migrate: %v", err)
	}

	r := buildRouterWithDeps(cfg, pool)

	log.Printf("evolve-server listening on :%s (tls=%v)", cfg.Port, cfg.CertFile != "")
	if cfg.CertFile != "" {
		if err := r.RunTLS(":"+cfg.Port, cfg.CertFile, cfg.KeyFile); err != nil {
			log.Fatalf("server: %v", err)
		}
	} else {
		if err := r.Run(":" + cfg.Port); err != nil {
			log.Fatalf("server: %v", err)
		}
	}
}
```

- [ ] **Step 6: Confirm build passes**

```bash
go build ./cmd/server/
```

Expected: exits 0, no output.

- [ ] **Step 7: Run full test suite**

```bash
go test ./...
```

Expected: all packages `ok`.

### 2c — Cert files and Makefile

- [ ] **Step 8: Commit code changes before cert setup**

```bash
git add internal/config/config.go internal/config/config_test.go cmd/server/main.go
git commit -m "feat: add TLS config fields and RunTLS branch in main"
```

- [ ] **Step 9: Create certs/.gitignore**

Create `certs/.gitignore`:

```
ca.key
server.crt
server.key
*.srl
```

- [ ] **Step 10: Copy CA cert into repo**

```bash
cp /run/media/navitank/Untitled/Projects/EvolveFilesLegacy/ca-bundle.crt \
   /run/media/navitank/Untitled/Projects/evolve-revival/evolve-server/certs/ca.crt
```

Verify:
```bash
openssl x509 -in certs/ca.crt -noout -subject
```
Expected output: `subject=C=US, O=PaiEnNate, OU=B140, CN=PaiEnNate Fake Root CA`

- [ ] **Step 11: Extract CA private key**

```bash
strings /run/media/navitank/Untitled/RiceFix.zip | unzip -p \
  /run/media/navitank/Untitled/RiceFix.zip \
  RiceFix/Bin64_SteamRetail/EvolveLegacyRebornServer.dll | \
  strings | \
  awk '/-----BEGIN RSA PRIVATE KEY-----/{f=1} f{print} /-----END RSA PRIVATE KEY-----/{f=0}' \
  > certs/ca.key
```

Verify the file was created and has content:
```bash
head -2 certs/ca.key
```
Expected:
```
-----BEGIN RSA PRIVATE KEY-----
MIIEpQIBAAKCAQEA5hgjjleCJV0MKQJ+ks51pGQ8wfvA1toJzREr5pHoPD0iKmKB
```

If the pipe approach doesn't work cleanly (the DLL is inside a zip), use the already-extracted copy:
```bash
strings /tmp/claude-1000/-run-media-navitank-Untitled-Projects-EvolveFilesLegacy/e346298f-ffa2-4fc6-9df9-fc7931e8fc3c/scratchpad/rf/RiceFix/Bin64_SteamRetail/EvolveLegacyRebornServer.dll \
  | awk '/-----BEGIN RSA PRIVATE KEY-----/{f=1} f{print} /-----END RSA PRIVATE KEY-----/{f=0}' \
  > certs/ca.key
```

- [ ] **Step 12: Update Makefile**

Replace `Makefile` entirely:

```makefile
.PHONY: build test run migrate cert cap test-game

build:
	go build -o bin/evolve-server ./cmd/server

test:
	go test ./...

test-integration:
	TEST_DATABASE_URL="$(TEST_DATABASE_URL)" go test ./...

test-game:
	@bash scripts/test-game.sh

run: cap
	PORT=443 CERT_FILE=certs/server.crt KEY_FILE=certs/server.key ./bin/evolve-server

cap: build
	sudo setcap cap_net_bind_service=+ep ./bin/evolve-server

cert:
	mkdir -p certs
	printf 'subjectAltName=IP:127.0.0.1\n' > certs/san.ext
	openssl genrsa -out certs/server.key 2048
	openssl req -new -key certs/server.key \
		-subj "/CN=127.0.0.1/O=EvolveRevival" \
		-out certs/server.csr
	openssl x509 -req -in certs/server.csr \
		-CA certs/ca.crt -CAkey certs/ca.key -CAcreateserial \
		-out certs/server.crt -days 3650 \
		-extfile certs/san.ext
	rm -f certs/server.csr certs/san.ext

migrate:
	psql "$(DATABASE_URL)" -f internal/db/migrations/001_init.sql
```

Note: Makefile indentation must be tabs, not spaces.

- [ ] **Step 13: Run `make cert`**

```bash
make cert
```

Expected output ends with something like:
```
Certificate request self-signature ok
subject=CN=127.0.0.1, O=EvolveRevival
```

Verify SAN is present:
```bash
openssl x509 -in certs/server.crt -noout -ext subjectAltName
```
Expected: `IP Address:127.0.0.1`

- [ ] **Step 14: Run `make cap`**

```bash
make cap
```

Expected: prompts for sudo password, then exits 0. Verify:
```bash
getcap bin/evolve-server
```
Expected: `bin/evolve-server cap_net_bind_service=ep`

- [ ] **Step 15: Smoke-test HTTPS**

Start the server (requires PostgreSQL running per DATABASE_URL default, or set a test DSN):

```bash
# In one terminal — if you have postgres running:
make run

# Or without DB (server will fail on migrate, but that's fine to test TLS separately):
PORT=443 CERT_FILE=certs/server.crt KEY_FILE=certs/server.key ./bin/evolve-server &
SERVER_PID=$!
sleep 2

# Test TLS with the CA cert:
curl -s --cacert certs/ca.crt https://127.0.0.1/status
```

Expected: `{"status":"ok","version":"1.0.0"}`

```bash
kill $SERVER_PID
```

- [ ] **Step 16: Commit cert infrastructure**

```bash
git add certs/ca.crt certs/.gitignore Makefile
git commit -m "feat: cert infrastructure — ca.crt, make cert/cap/run targets"
```

---

## Task 3: Test Automation Script

**Files:**
- Create: `scripts/test-game.sh`
- (Makefile `test-game` target already added in Task 2)

**Interfaces:**
- Consumes: `make cap` (binary with setcap), `certs/server.crt` + `certs/server.key` (from `make cert`)
- Consumes: game at `$GAME_DIR/bin64_SteamRetail/Evolve.exe`, prefix at `$GAME_DIR/proton_prefix`
- Produces: exit 0 (main menu reached) or exit 1 (timeout/failure) with diagnostic output

- [ ] **Step 1: Create scripts/test-game.sh**

```bash
mkdir -p scripts
```

Create `scripts/test-game.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

GAME_DIR="${GAME_DIR:-/run/media/navitank/Untitled/Projects/EvolveFilesLegacy}"
PROTON="${PROTON:-$HOME/.steam/steam/steamapps/common/Proton 8.0/proton}"
TIMEOUT="${TIMEOUT:-90}"
SERVER_LOG="/tmp/evolve-server-test.log"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

SERVER_PID=""
GAME_PID=""

cleanup() {
    [[ -n "$GAME_PID" ]] && kill "$GAME_PID" 2>/dev/null || true
    [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> Building server..."
cd "$REPO_DIR"
make build

echo "==> Setting port-443 capability..."
sudo setcap cap_net_bind_service=+ep ./bin/evolve-server

echo "==> Clearing game logs..."
: > "$GAME_DIR/kandoC.log"
: > "$GAME_DIR/EVOLVE_LOG.txt"

echo "==> Starting evolve-server on :443..."
PORT=443 CERT_FILE=certs/server.crt KEY_FILE=certs/server.key \
    ./bin/evolve-server > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!

echo "==> Waiting for server health (up to 10s)..."
for i in $(seq 1 10); do
    if curl -sk --cacert certs/ca.crt https://127.0.0.1/status 2>/dev/null | grep -q '"ok"'; then
        echo "    server ready (${i}s)"
        break
    fi
    if [[ $i -eq 10 ]]; then
        echo "FAIL: server did not become healthy in 10s"
        echo "--- server log ---"
        cat "$SERVER_LOG"
        exit 1
    fi
    sleep 1
done

echo "==> Launching Evolve via Proton..."
STEAM_COMPAT_DATA_PATH="$GAME_DIR/proton_prefix" \
STEAM_COMPAT_CLIENT_INSTALL_PATH="$HOME/.steam/steam" \
"$PROTON" run "$GAME_DIR/bin64_SteamRetail/Evolve.exe" &
GAME_PID=$!

echo "==> Watching for main menu (up to ${TIMEOUT}s)..."
echo "    Pass signal: /queue/waittime with non-empty regionId in server log"
START=$(date +%s)
while true; do
    NOW=$(date +%s)
    ELAPSED=$((NOW - START))

    if [[ $ELAPSED -ge $TIMEOUT ]]; then
        echo "FAIL: main menu not reached within ${TIMEOUT}s"
        echo "--- server log (last 30 lines) ---"
        tail -30 "$SERVER_LOG"
        echo "--- kandoC.log (last 20 lines) ---"
        tail -20 "$GAME_DIR/kandoC.log"
        exit 1
    fi

    # Pass: queue/waittime hit with a real regionId (e.g. IAD, EU, etc.)
    # The game sends regionId= (empty) on first boot; non-empty means main menu loaded.
    if grep -qE 'queue/waittime.*regionId=[A-Z]{2,}' "$SERVER_LOG" 2>/dev/null; then
        echo "PASS: main menu reached (${ELAPSED}s)"
        exit 0
    fi

    sleep 2
done
```

- [ ] **Step 2: Make executable**

```bash
chmod +x scripts/test-game.sh
```

- [ ] **Step 3: Verify the script is syntactically valid**

```bash
bash -n scripts/test-game.sh
```

Expected: exits 0, no output.

- [ ] **Step 4: Confirm `make test-game` invokes the script**

```bash
# Dry run — just verify make finds the target (will fail fast since DB isn't expected here)
make -n test-game
```

Expected output: `bash scripts/test-game.sh`

- [ ] **Step 5: Run the full integration test**

Prerequisites: PostgreSQL running (for evolve-server's DB migrations), `certs/server.crt` and `certs/server.key` exist (from `make cert`).

```bash
make test-game
```

Watch the output. Expected flow:
```
==> Building server...
==> Setting port-443 capability...
==> Clearing game logs...
==> Starting evolve-server on :443...
==> Waiting for server health (up to 10s)...
    server ready (1s)
==> Launching Evolve via Proton...
==> Watching for main menu (up to 90s)...
    Pass signal: /queue/waittime with non-empty regionId in server log
PASS: main menu reached (47s)
```

If it FAILs, the script prints the last 30 lines of `/tmp/evolve-server-test.log` and `kandoC.log`. Common issues:
- `cURL error 7` in kandoC.log → server not answering on 443 (check `make cap` ran, check certs exist)
- Server log shows 404 on a route → check router catch-all is wired (Task 1)
- Timeout with only empty-regionId queue polls → game stuck at auth screen; check kandoC.log for the specific failing route

- [ ] **Step 6: Commit**

```bash
git add scripts/test-game.sh
git commit -m "feat: add test-game script — headless Proton integration test"
```

---

## Self-Review

**Spec coverage:**

| Spec section | Task covering it |
|---|---|
| TLS on 443 with RunTLS | Task 2 (main.go) |
| PORT default → 443 | Task 2 (config.go) |
| `make cert` with SAN for 127.0.0.1 | Task 2 (Makefile) |
| `make cap` for port 443 | Task 2 (Makefile) |
| `/:platformId/*` routes | Task 1 (NoRoute catch-all) |
| `/evolve/config/*path` route | Task 1 (explicit + NoRoute) |
| `r.NoRoute(stubs.Stub200)` catch-all | Task 1 |
| `certs/ca.crt` committed, `ca.key` gitignored | Task 2 |
| `make test-game` script | Task 3 |
| Pass signal: `queue/waittime` with regionId | Task 3 |
| Failure output: server log + kandoC.log tail | Task 3 |
| `make test` unaffected | Unit tests use `httptest`, no port binding |

**No gaps found.**
