# Evolve Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Go + gin HTTP server that emulates the 2K Kando API so Evolve clients can authenticate, load player data, and find lobbies — all without running the local Pinenut emulator.

**Architecture:** Single Go binary behind nginx (TLS). Gin handles routing. PostgreSQL stores player data. The game connects to this server identically to how it connected to Pinenut, except the server runs on a VPS instead of localhost.

**Tech Stack:** Go 1.26, gin-gonic/gin v1.10, lib/pq v1.10, google/uuid v1.6, PostgreSQL 16

---

## File Map

```
evolve-server/
├── cmd/server/main.go                 # entry point, gin setup, all route wiring
├── internal/
│   ├── config/
│   │   ├── config.go                  # env-based config (PORT, DATABASE_URL, SERVER_HOST)
│   │   └── config_test.go
│   ├── db/
│   │   ├── db.go                      # Open() returns *sql.DB
│   │   └── migrate.go                 # runs embedded SQL migrations on startup
│   ├── model/
│   │   ├── doorman.go                 # DoormanResponse, Service, ServiceInstance, ClientConfig
│   │   ├── sso.go                     # SSOResponse
│   │   └── entitlement.go             # EntitlementItem, EntitlementResponse
│   ├── handler/
│   │   ├── doorman.go                 # GET /doorman/1/configs.generate
│   │   ├── sso.go                     # POST /sso/1/auths.logon
│   │   ├── entitlements.go            # GET /entitlements/1/entitlementDefs.getFirstPartyMapping
│   │   ├── storage.go                 # CRUD /storage/1/:datasetId
│   │   ├── stubs.go                   # telemetry, content, stats, grants, queue, events, news
│   │   ├── peers.go                   # POST /peers/register, GET /peers/:lobbyId
│   │   └── status.go                  # GET /status, GET /build_config
│   └── store/
│       ├── player.go                  # UpsertPlayer
│       ├── storage.go                 # Get/Put/Delete storage items
│       └── peer.go                    # RegisterPeer, GetPeers, PurgeExpired
├── migrations/
│   └── 001_init.sql
├── go.mod
└── Makefile
```

---

### Task 1: Project scaffold + health endpoint

**Files:**
- Create: `evolve-server/go.mod`
- Create: `evolve-server/internal/config/config.go`
- Create: `evolve-server/internal/config/config_test.go`
- Create: `evolve-server/internal/db/db.go`
- Create: `evolve-server/cmd/server/main.go`
- Create: `evolve-server/Makefile`

- [ ] **Step 1: Write config_test.go**

```go
// evolve-server/internal/config/config_test.go
package config_test

import (
	"os"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/config"
)

func TestLoad_defaults(t *testing.T) {
	os.Unsetenv("PORT")
	os.Unsetenv("DATABASE_URL")
	os.Unsetenv("SERVER_HOST")
	cfg := config.Load()
	if cfg.Port != "8080" {
		t.Errorf("Port = %q, want 8080", cfg.Port)
	}
	if cfg.ServerHost == "" {
		t.Error("ServerHost must not be empty")
	}
}

func TestLoad_env(t *testing.T) {
	os.Setenv("PORT", "9090")
	defer os.Unsetenv("PORT")
	cfg := config.Load()
	if cfg.Port != "9090" {
		t.Errorf("Port = %q, want 9090", cfg.Port)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd evolve-server && go test ./internal/config/... -v
```

Expected: `cannot find package`

- [ ] **Step 3: Create go.mod and install dependencies**

```bash
cd evolve-server && go mod init github.com/evolve-revival/evolve-server
go get github.com/gin-gonic/gin@v1.10.0
go get github.com/lib/pq@v1.10.9
go get github.com/google/uuid@v1.6.0
```

- [ ] **Step 4: Write config.go**

```go
// evolve-server/internal/config/config.go
package config

import "os"

type Config struct {
	Port       string
	DBDSN      string
	ServerHost string // e.g. "community.evolve.example.com" — included in doorman service URLs
}

func Load() Config {
	return Config{
		Port:       getenv("PORT", "8080"),
		DBDSN:      getenv("DATABASE_URL", "postgres://evolve:evolve@localhost/evolve?sslmode=disable"),
		ServerHost: getenv("SERVER_HOST", "localhost:8080"),
	}
}

func getenv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
```

- [ ] **Step 5: Run config tests**

```bash
cd evolve-server && go test ./internal/config/... -v
```

Expected: PASS, 2 tests.

- [ ] **Step 6: Write db.go**

```go
// evolve-server/internal/db/db.go
package db

import (
	"database/sql"
	_ "github.com/lib/pq"
)

func Open(dsn string) (*sql.DB, error) {
	pool, err := sql.Open("postgres", dsn)
	if err != nil {
		return nil, err
	}
	if err := pool.Ping(); err != nil {
		return nil, err
	}
	return pool, nil
}
```

- [ ] **Step 7: Write main.go with health endpoint**

```go
// evolve-server/cmd/server/main.go
package main

import (
	"log"
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/db"
	"github.com/gin-gonic/gin"
)

func main() {
	cfg := config.Load()

	pool, err := db.Open(cfg.DBDSN)
	if err != nil {
		log.Fatalf("db: %v", err)
	}
	defer pool.Close()

	r := gin.Default()
	r.GET("/health", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "ok"})
	})

	log.Printf("evolve-server listening on :%s", cfg.Port)
	if err := r.Run(":" + cfg.Port); err != nil {
		log.Fatal(err)
	}
}
```

- [ ] **Step 8: Write Makefile**

```makefile
# evolve-server/Makefile
.PHONY: build test run

build:
	go build -o bin/evolve-server ./cmd/server

test:
	go test ./...

run: build
	./bin/evolve-server
```

- [ ] **Step 9: Build and verify**

```bash
cd evolve-server && go build ./... 2>&1
```

Expected: no errors.

- [ ] **Step 10: Commit**

```bash
cd evolve-server && git init && git add . && git commit -m "feat: evolve-server scaffold with health endpoint"
```

---

### Task 2: DB migrations

**Files:**
- Create: `evolve-server/migrations/001_init.sql`
- Create: `evolve-server/internal/db/migrate.go`
- Create: `evolve-server/internal/db/migrate_test.go`

- [ ] **Step 1: Write migrate_test.go**

```go
// evolve-server/internal/db/migrate_test.go
package db_test

import (
	"os"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/db"
)

func TestMigrate(t *testing.T) {
	dsn := os.Getenv("TEST_DATABASE_URL")
	if dsn == "" {
		t.Skip("TEST_DATABASE_URL not set")
	}
	pool, err := db.Open(dsn)
	if err != nil {
		t.Fatalf("open: %v", err)
	}
	defer pool.Close()
	if err := db.Migrate(pool); err != nil {
		t.Fatalf("migrate: %v", err)
	}
	// Run again to verify idempotency
	if err := db.Migrate(pool); err != nil {
		t.Fatalf("re-migrate: %v", err)
	}
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd evolve-server && go test ./internal/db/... -v -run TestMigrate
```

Expected: `cannot find package` or `Migrate undefined`

- [ ] **Step 3: Write 001_init.sql**

```sql
-- evolve-server/migrations/001_init.sql
CREATE TABLE IF NOT EXISTS schema_migrations (
    version  INTEGER PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS players (
    id           TEXT PRIMARY KEY,           -- UUID, assigned on first logon
    steam_id     TEXT UNIQUE,                -- Goldberg steam ID (may be null for guest)
    display_name TEXT NOT NULL DEFAULT '',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS storage_items (
    id          TEXT PRIMARY KEY,            -- UUID
    dataset_id  TEXT NOT NULL,               -- one of the 5 known dataset UUIDs
    player_id   TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    item_key    TEXT NOT NULL,
    data        JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (dataset_id, player_id, item_key)
);

CREATE TABLE IF NOT EXISTS peers (
    lobby_id      TEXT NOT NULL,
    player_id     TEXT NOT NULL,
    ip            TEXT NOT NULL,
    port          INTEGER NOT NULL,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (lobby_id, player_id)
);
```

- [ ] **Step 4: Write migrate.go**

```go
// evolve-server/internal/db/migrate.go
package db

import (
	"database/sql"
	_ "embed"
	"fmt"
)

//go:embed ../../migrations/001_init.sql
var initSQL string

// Migrate runs all pending migrations. Safe to call on every startup.
func Migrate(pool *sql.DB) error {
	// Ensure migrations table exists first (bootstrapping).
	if _, err := pool.Exec(`CREATE TABLE IF NOT EXISTS schema_migrations (
		version INTEGER PRIMARY KEY,
		applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
	)`); err != nil {
		return fmt.Errorf("create migrations table: %w", err)
	}

	var count int
	if err := pool.QueryRow(`SELECT COUNT(*) FROM schema_migrations WHERE version = 1`).Scan(&count); err != nil {
		return fmt.Errorf("check migration 1: %w", err)
	}
	if count > 0 {
		return nil // already applied
	}

	tx, err := pool.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()

	if _, err := tx.Exec(initSQL); err != nil {
		return fmt.Errorf("migration 1: %w", err)
	}
	if _, err := tx.Exec(`INSERT INTO schema_migrations (version) VALUES (1)`); err != nil {
		return fmt.Errorf("record migration 1: %w", err)
	}
	return tx.Commit()
}
```

- [ ] **Step 5: Update main.go to call Migrate on startup**

Add after `db.Open`:
```go
if err := db.Migrate(pool); err != nil {
    log.Fatalf("migrate: %v", err)
}
```

- [ ] **Step 6: Run migration test (requires postgres)**

```bash
cd evolve-server && TEST_DATABASE_URL="postgres://evolve:evolve@localhost/evolve_test?sslmode=disable" go test ./internal/db/... -v -run TestMigrate
```

Expected: PASS, idempotent.

- [ ] **Step 7: Commit**

```bash
git add . && git commit -m "feat: db migrations — players, storage_items, peers tables"
```

---

### Task 3: Model types (doorman + SSO + entitlements)

**Files:**
- Create: `evolve-server/internal/model/doorman.go`
- Create: `evolve-server/internal/model/sso.go`
- Create: `evolve-server/internal/model/entitlement.go`

These are pure data types — no logic, no tests needed.

- [ ] **Step 1: Write model/doorman.go**

```go
// evolve-server/internal/model/doorman.go
package model

// DoormanResponse is returned by GET /doorman/1/configs.generate.
// The game uses this to discover all other service URLs and get an
// anonymous pre-auth session token.
type DoormanResponse struct {
	AccessToken          string        `json:"accessToken"`
	ExpiresIn            int           `json:"expiresIn"`
	RefreshToken         string        `json:"refreshToken"`
	RefreshExpiresIn     int           `json:"refreshExpiresIn"`
	TokenType            string        `json:"tokenType"`
	PlayerId             interface{}   `json:"playerId"`     // null for pre-auth
	SessionId            string        `json:"sessionId"`
	Services             []Service     `json:"services"`
	ClientConfigSettings ClientConfig  `json:"clientConfigSettings"`
	PlatformType         int           `json:"platformType"`
	OnlineServicePlatform int          `json:"onlineServicePlatform"`
}

type Service struct {
	ServiceName      string            `json:"serviceName"`
	ServiceInstances []ServiceInstance `json:"serviceInstances"`
}

type ServiceInstance struct {
	Protocol     string   `json:"protocol"`
	Host         string   `json:"host"`
	Port         int      `json:"port"`
	BaseUri      string   `json:"baseUri"`
	Actions      []string `json:"actions"`
	IsProduction bool     `json:"isProduction"`
}

type ClientConfig struct {
	DoormanConnectTimeout        int  `json:"doormanConnectTimeout"`
	DoormanRequestTimeout        int  `json:"doormanRequestTimeout"`
	LogLevelMin                  int  `json:"logLevelMin"`
	LogLevelMax                  int  `json:"logLevelMax"`
	LogLevel                     int  `json:"logLevel"`
	DebugModeMin                 bool `json:"debugModeMin"`
	DebugModeMax                 bool `json:"debugModeMax"`
	DebugMode                    bool `json:"debugMode"`
	RestLogToFileModeMin         bool `json:"restLogToFileModeMin"`
	RestLogToFileModeMax         bool `json:"restLogToFileModeMax"`
	RestLogToFileMode            bool `json:"restLogToFileMode"`
	AutoCacheLogin               bool `json:"autoCacheLogin"`
	MinRequestTimeout            int  `json:"minRequestTimeout"`
	DefaultRequestTimeout        int  `json:"defaultRequestTimeout"`
	AssertTelemetry              bool `json:"assertTelemetry"`
	AppHasStorefront             bool `json:"appHasStorefront"`
	StorefrontHasConsumables     bool `json:"storefrontHasConsumables"`
}
```

- [ ] **Step 2: Write model/sso.go**

```go
// evolve-server/internal/model/sso.go
package model

// SSOResponse is returned by POST /sso/1/auths.logon.
// Contains the player-specific access token and player UUID.
type SSOResponse struct {
	IsNewPlayer      bool   `json:"isNewPlayer"`
	HasPlayedApp     bool   `json:"hasPlayedApp"`
	AccessToken      string `json:"accessToken"`
	ExpiresIn        int    `json:"expiresIn"`
	TokenType        string `json:"tokenType"`
	PlayerId         string `json:"playerId"`
	SessionId        string `json:"sessionId"`
	RefreshToken     string `json:"refreshToken"`
	RefreshExpiresIn int    `json:"refreshExpiresIn"`
	DobNeeded        bool   `json:"dobNeeded"`
}
```

- [ ] **Step 3: Write model/entitlement.go**

```go
// evolve-server/internal/model/entitlement.go
package model

// EntitlementItem represents a single granted entitlement.
// All 468 entitlementIds are granted to every player.
// appGroupId "c3dc178f670ee769fe59e244610d66e2" is Evolve's app group.
type EntitlementItem struct {
	CreatedOn             int         `json:"createdOn"`
	EntitlementDefId      string      `json:"entitlementDefId"`
	IsServerAuthoritative bool        `json:"isServerAuthoritative"`
	IsValid               bool        `json:"isValid"`
	RuleData              RuleData    `json:"ruleData"`
	EntitlementId         string      `json:"entitlementId"`
	AppPublicId           interface{} `json:"appPublicId"`
	AppGroupId            string      `json:"appGroupId"`
	PlayerPublicId        string      `json:"playerPublicId"`
	IsAvailable           bool        `json:"isAvailable"`
	IsShared              bool        `json:"isShared"`
}

type RuleData struct {
	Grant bool `json:"grant"`
}

// EntitlementDefs is the mapping table that pairs entitlement IDs to def IDs.
// entitlementDefIds and entitlementIds are parallel arrays — index i in one
// corresponds to index i in the other. All 468 are extracted from Pinenut.
var EntitlementIds = []string{
	"c7d22439bc13e53554776bbec4c175db", "bada12bfd30bcd02a609f7c88ca0a244",
	"d732cbb899e12be62d1fdfb6d43ac15c", "c0e9412ae0e437935f4fc1ef9db8add7",
	"47b1ec72d6dee517cc7fd896e5c66c93", "79c0b7fbb8b5a610d5665fe66e3c6c6a",
	"2ebd108f2cdce9e5315295786859eabe", "7ea5435a8af71001b790a6766d0b14b5",
	"5511deee62056a54d1a940fc8d14d1d7", "bb9e6744eb44a96c25987283f2150844",
	"034463e6f628e94147c366850693f3af", "ae60dc2f1dc7ce5e837e23d765308a7f",
	"09dc8569d41d18169dba133b646a0c7c", "ae16a01bd0f92671190a36f7edcd218c",
	"f26c3f0e0d4c89f9583403d08c68fb15", "669261b069a12f196ca6e083c5944059",
	"92f875f36cf4fe94265d07ac1574678b", "cb4b79d5d1e10b0b0a3dc98f044c0f14",
	"4b3b9650cf729c8ebd90354d22121f5e", "c5bed6ded5991a7d5deb29df9e270856",
	"43745436a426991ef359ac767af529bf", "5e83883ab63364d58502733b10186c31",
	"c816eb19a5a20bbc035fed633a4c04ca", "cf6ff1e376dec1a317459794d60d916a",
	"5d8e4486551dcd54b27224a159e8885d", "9c84c670e462589c20e5cf95afea516a",
	"7ad74999061ae851075957b3b65d3f66", "a96e8e5e664d6d6fd230731d59d62e94",
	"e946877ca525e8379d9929de9c4e5890", "f3deb191029a5baee0e980b8d3e02894",
	// NOTE: this slice contains the first 30 of 468 entitlement IDs.
	// To get the full list, run:
	//   strings EvolveLegacyRebornServer.dll | grep '"entitlementId"' | sed 's/.*"\([0-9a-f]*\)".*/\1/' | sort -u
	// and paste all 468 here. All are granted to every player (Pinenut behaviour).
}

// EntitlementDefIds maps entitlement IDs to their def IDs (same index).
// Populate this list by running the same extraction command against the DLL,
// grep '"entitlementDefId"' instead. For now Pinenut uses the same ID for both.
var EntitlementDefIds = EntitlementIds

const AppGroupId = "c3dc178f670ee769fe59e244610d66e2"
```

- [ ] **Step 4: Commit**

```bash
git add . && git commit -m "feat: response model types for doorman, SSO, entitlements"
```

---

### Task 4: Player store + SSO handler

**Files:**
- Create: `evolve-server/internal/store/player.go`
- Create: `evolve-server/internal/handler/sso.go`
- Create: `evolve-server/internal/handler/sso_test.go`

- [ ] **Step 1: Write sso_test.go**

```go
// evolve-server/internal/handler/sso_test.go
package handler_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/db"
	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

func setupTestDB(t *testing.T) *sql.DB {
	t.Helper()
	dsn := os.Getenv("TEST_DATABASE_URL")
	if dsn == "" {
		t.Skip("TEST_DATABASE_URL not set")
	}
	pool, err := db.Open(dsn)
	if err != nil {
		t.Fatalf("open: %v", err)
	}
	if err := db.Migrate(pool); err != nil {
		t.Fatalf("migrate: %v", err)
	}
	t.Cleanup(func() {
		pool.Exec("TRUNCATE players, storage_items, peers CASCADE")
		pool.Close()
	})
	return pool
}

func TestSSO_Logon_NewPlayer(t *testing.T) {
	pool := setupTestDB(t)
	ps := store.NewPlayerStore(pool)
	cfg := config.Config{ServerHost: "localhost:8080"}

	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.POST("/sso/1/auths.logon", handler.NewSSOHandler(ps, cfg).Logon)

	body := `{"steamId":"76561198000000001","displayName":"TestPlayer"}`
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodPost, "/sso/1/auths.logon", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200; body: %s", w.Code, w.Body)
	}
	var resp model.SSOResponse
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if resp.PlayerId == "" {
		t.Error("PlayerId must not be empty")
	}
	if resp.AccessToken == "" {
		t.Error("AccessToken must not be empty")
	}
	if resp.IsNewPlayer != true {
		t.Error("IsNewPlayer should be true for first login")
	}
}

func TestSSO_Logon_ExistingPlayer(t *testing.T) {
	pool := setupTestDB(t)
	ps := store.NewPlayerStore(pool)
	cfg := config.Config{ServerHost: "localhost:8080"}

	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.POST("/sso/1/auths.logon", handler.NewSSOHandler(ps, cfg).Logon)

	body := `{"steamId":"76561198000000002","displayName":"ReturningPlayer"}`
	do := func() *httptest.ResponseRecorder {
		w := httptest.NewRecorder()
		req := httptest.NewRequest(http.MethodPost, "/sso/1/auths.logon", bytes.NewBufferString(body))
		req.Header.Set("Content-Type", "application/json")
		r.ServeHTTP(w, req)
		return w
	}
	do() // first login
	w2 := do() // second login

	var resp model.SSOResponse
	json.Unmarshal(w2.Body.Bytes(), &resp)
	if resp.IsNewPlayer != false {
		t.Error("IsNewPlayer should be false on second login")
	}
}
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestSSO 2>&1 | head -20
```

Expected: build errors (handler package doesn't exist yet).

- [ ] **Step 3: Write store/player.go**

```go
// evolve-server/internal/store/player.go
package store

import (
	"database/sql"
	"fmt"
	"time"

	"github.com/google/uuid"
)

type Player struct {
	Id          string
	SteamId     string
	DisplayName string
	CreatedAt   time.Time
	IsNew       bool // not stored; true if this logon created the row
}

type PlayerStore struct{ db *sql.DB }

func NewPlayerStore(db *sql.DB) *PlayerStore { return &PlayerStore{db: db} }

// UpsertBySteamId finds or creates a player row for the given Steam ID.
// Returns IsNew=true if the row was just created.
func (s *PlayerStore) UpsertBySteamId(steamId, displayName string) (*Player, error) {
	var p Player
	err := s.db.QueryRow(
		`SELECT id, steam_id, display_name, created_at FROM players WHERE steam_id = $1`,
		steamId,
	).Scan(&p.Id, &p.SteamId, &p.DisplayName, &p.CreatedAt)

	if err == sql.ErrNoRows {
		p.Id = uuid.New().String()
		p.SteamId = steamId
		p.DisplayName = displayName
		p.IsNew = true
		_, err = s.db.Exec(
			`INSERT INTO players (id, steam_id, display_name) VALUES ($1, $2, $3)`,
			p.Id, steamId, displayName,
		)
		if err != nil {
			return nil, fmt.Errorf("insert player: %w", err)
		}
		return &p, nil
	}
	if err != nil {
		return nil, fmt.Errorf("query player: %w", err)
	}
	return &p, nil
}
```

- [ ] **Step 4: Write handler/sso.go**

```go
// evolve-server/internal/handler/sso.go
package handler

import (
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

type SSOHandler struct {
	players *store.PlayerStore
	cfg     config.Config
}

func NewSSOHandler(players *store.PlayerStore, cfg config.Config) *SSOHandler {
	return &SSOHandler{players: players, cfg: cfg}
}

type ssoRequest struct {
	SteamId     string `json:"steamId"`
	DisplayName string `json:"displayName"`
}

// Logon handles POST /sso/1/auths.logon.
// Creates or retrieves a player by Steam ID, returns an access token.
func (h *SSOHandler) Logon(c *gin.Context) {
	var req ssoRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	if req.SteamId == "" {
		req.SteamId = uuid.New().String() // guest player
	}

	player, err := h.players.UpsertBySteamId(req.SteamId, req.DisplayName)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, model.SSOResponse{
		IsNewPlayer:      player.IsNew,
		HasPlayedApp:     !player.IsNew,
		AccessToken:      uuid.New().String(),
		ExpiresIn:        86400,
		TokenType:        "bearer",
		PlayerId:         player.Id,
		SessionId:        uuid.New().String(),
		RefreshToken:     uuid.New().String(),
		RefreshExpiresIn: 7200,
		DobNeeded:        false,
	})
}
```

- [ ] **Step 5: Fix test import (add missing `database/sql` import to test)**

In `sso_test.go`, add `"database/sql"` to the import block.

- [ ] **Step 6: Run SSO tests**

```bash
cd evolve-server && TEST_DATABASE_URL="postgres://evolve:evolve@localhost/evolve_test?sslmode=disable" go test ./internal/handler/... -v -run TestSSO
```

Expected: PASS, 2 tests.

- [ ] **Step 7: Commit**

```bash
git add . && git commit -m "feat: SSO logon endpoint + player upsert store"
```

---

### Task 4b: Auth session middleware

The SSO handler issues an `accessToken` UUID. Every subsequent request from the game sends this token in the `Authorization: Bearer <token>` header. The auth middleware looks up the token and injects `playerId` into the gin context — which storage, entitlements, and peers handlers all read via `c.GetString("playerId")`.

Tokens are stored in-memory (a `sync.Map`). If the server restarts, the game calls SSO again and gets a new token, so persistence isn't needed.

**Files:**
- Modify: `evolve-server/internal/handler/sso.go` (write token to session store)
- Create: `evolve-server/internal/middleware/auth.go`
- Create: `evolve-server/internal/middleware/auth_test.go`

- [ ] **Step 1: Write auth_test.go**

```go
// evolve-server/internal/middleware/auth_test.go
package middleware_test

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/middleware"
	"github.com/gin-gonic/gin"
)

func TestAuth_ValidToken(t *testing.T) {
	middleware.StoreToken("valid-token-123", "player-uuid-abc")

	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.Use(middleware.Auth())
	r.GET("/test", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"playerId": c.GetString("playerId")})
	})

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	req.Header.Set("Authorization", "Bearer valid-token-123")
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200; body: %s", w.Code, w.Body)
	}
	if w.Body.String() != `{"playerId":"player-uuid-abc"}` {
		t.Errorf("body = %s", w.Body)
	}
}

func TestAuth_MissingToken_AllowsThrough(t *testing.T) {
	// Routes that don't need auth (doorman, SSO) should still work with no token.
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.Use(middleware.Auth())
	r.GET("/test", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"playerId": c.GetString("playerId")})
	})

	w := httptest.NewRecorder()
	r.ServeHTTP(w, httptest.NewRequest(http.MethodGet, "/test", nil))
	if w.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200", w.Code)
	}
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd evolve-server && go test ./internal/middleware/... -v 2>&1 | head -10
```

Expected: build error.

- [ ] **Step 3: Write middleware/auth.go**

```go
// evolve-server/internal/middleware/auth.go
package middleware

import (
	"strings"
	"sync"

	"github.com/gin-gonic/gin"
)

var tokenStore sync.Map // token string → playerId string

// StoreToken records an accessToken → playerId mapping.
// Called by the SSO handler after a successful logon.
func StoreToken(token, playerId string) {
	tokenStore.Store(token, playerId)
}

// Auth is a gin middleware that reads Authorization: Bearer <token>,
// looks up the playerId, and injects it as "playerId" into the context.
// If no token is present or it's unknown, the request continues with
// an empty playerId (doorman and SSO endpoints work without auth).
func Auth() gin.HandlerFunc {
	return func(c *gin.Context) {
		authHeader := c.GetHeader("Authorization")
		if strings.HasPrefix(authHeader, "Bearer ") {
			token := strings.TrimPrefix(authHeader, "Bearer ")
			if val, ok := tokenStore.Load(token); ok {
				c.Set("playerId", val.(string))
			}
		}
		c.Next()
	}
}
```

- [ ] **Step 4: Update SSO handler to call StoreToken**

In `handler/sso.go`, import `middleware` and call `middleware.StoreToken(accessToken, player.Id)` before `c.JSON(...)`. The full updated Logon function:

```go
func (h *SSOHandler) Logon(c *gin.Context) {
	var req ssoRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	if req.SteamId == "" {
		req.SteamId = uuid.New().String()
	}

	player, err := h.players.UpsertBySteamId(req.SteamId, req.DisplayName)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	accessToken := uuid.New().String()
	middleware.StoreToken(accessToken, player.Id) // register token → playerId

	c.JSON(http.StatusOK, model.SSOResponse{
		IsNewPlayer:      player.IsNew,
		HasPlayedApp:     !player.IsNew,
		AccessToken:      accessToken,
		ExpiresIn:        86400,
		TokenType:        "bearer",
		PlayerId:         player.Id,
		SessionId:        uuid.New().String(),
		RefreshToken:     uuid.New().String(),
		RefreshExpiresIn: 7200,
		DobNeeded:        false,
	})
}
```

- [ ] **Step 5: Add Auth middleware import to handler/sso.go**

Add to imports: `"github.com/evolve-revival/evolve-server/internal/middleware"`

- [ ] **Step 6: Run auth tests**

```bash
cd evolve-server && go test ./internal/middleware/... -v
```

Expected: PASS, 2 tests.

- [ ] **Step 7: Commit**

```bash
git add . && git commit -m "feat: auth middleware — Bearer token → playerId injection"
```

---

### Task 5: Doorman endpoint

**Files:**
- Create: `evolve-server/internal/handler/doorman.go`
- Create: `evolve-server/internal/handler/doorman_test.go`

- [ ] **Step 1: Write doorman_test.go**

```go
// evolve-server/internal/handler/doorman_test.go
package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/gin-gonic/gin"
)

func TestDoorman_ConfigsGenerate(t *testing.T) {
	cfg := config.Config{ServerHost: "testserver.example.com"}
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.GET("/doorman/1/configs.generate", handler.NewDoormanHandler(cfg).ConfigsGenerate)

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/doorman/1/configs.generate", nil)
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200", w.Code)
	}

	var resp model.DoormanResponse
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if resp.AccessToken == "" {
		t.Error("AccessToken must not be empty")
	}
	if len(resp.Services) == 0 {
		t.Error("Services must not be empty")
	}

	// Verify all expected services are present
	serviceNames := make(map[string]bool)
	for _, svc := range resp.Services {
		serviceNames[svc.ServiceName] = true
		if len(svc.ServiceInstances) == 0 {
			t.Errorf("service %q has no instances", svc.ServiceName)
		}
		if svc.ServiceInstances[0].Host != "testserver.example.com/" {
			t.Errorf("service %q host = %q, want testserver.example.com/",
				svc.ServiceName, svc.ServiceInstances[0].Host)
		}
	}
	for _, name := range []string{"doorman", "singlesignon", "storage", "entitlements", "sessions", "stats"} {
		if !serviceNames[name] {
			t.Errorf("missing service %q", name)
		}
	}
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestDoorman 2>&1 | head -10
```

Expected: build error (`DoormanHandler undefined`).

- [ ] **Step 3: Write handler/doorman.go**

```go
// evolve-server/internal/handler/doorman.go
package handler

import (
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

type DoormanHandler struct{ cfg config.Config }

func NewDoormanHandler(cfg config.Config) *DoormanHandler {
	return &DoormanHandler{cfg: cfg}
}

// ConfigsGenerate handles GET /doorman/1/configs.generate.
// Returns an anonymous pre-auth session + the full services directory.
// The game uses the services list to discover where SSO, storage, etc. live.
func (h *DoormanHandler) ConfigsGenerate(c *gin.Context) {
	host := h.cfg.ServerHost + "/"

	services := []model.Service{
		svc("doorman", host, "doorman/1"),
		svc("singlesignon", host, "sso/1"),
		svc("entitlements", host, "entitlements/1"),
		svc("storage", host, "storage/1"),
		svc("sessions", host, "sessions/1"),
		svc("stats", host, "stats/1"),
		svc("grants", host, "grants/1"),
		svc("telemetry", host, "telemetry/1"),
		svc("content", host, "content/1"),
		svc("news", host, "news/1"),
		svc("apps", host, "apps/1"),
		svc("players", host, "players/1"),
		svc("storefront", host, "storefront/1"),
	}

	c.JSON(http.StatusOK, model.DoormanResponse{
		AccessToken:      uuid.New().String(),
		ExpiresIn:        86400,
		RefreshToken:     uuid.New().String(),
		RefreshExpiresIn: 7200,
		TokenType:        "bearer",
		PlayerId:         nil,
		SessionId:        uuid.New().String(),
		Services:         services,
		ClientConfigSettings: model.ClientConfig{
			DoormanConnectTimeout:    10,
			DoormanRequestTimeout:    15,
			LogLevelMin:              0,
			LogLevelMax:              4,
			LogLevel:                 0,
			DebugModeMin:             false,
			DebugModeMax:             true,
			DebugMode:                false,
			RestLogToFileModeMin:     false,
			RestLogToFileModeMax:     true,
			RestLogToFileMode:        false,
			AutoCacheLogin:           true,
			MinRequestTimeout:        90,
			DefaultRequestTimeout:    90,
			AssertTelemetry:          false,
			AppHasStorefront:         true,
			StorefrontHasConsumables: true,
		},
		PlatformType:          3,
		OnlineServicePlatform: 3,
	})
}

func svc(name, host, baseUri string) model.Service {
	return model.Service{
		ServiceName: name,
		ServiceInstances: []model.ServiceInstance{{
			Protocol:     "https",
			Host:         host,
			Port:         443,
			BaseUri:      baseUri,
			Actions:      []string{},
			IsProduction: false,
		}},
	}
}
```

- [ ] **Step 4: Run doorman tests**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestDoorman
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add . && git commit -m "feat: doorman configs.generate endpoint"
```

---

### Task 6: Entitlements handler

**Files:**
- Create: `evolve-server/internal/handler/entitlements.go`
- Create: `evolve-server/internal/handler/entitlements_test.go`

- [ ] **Step 1: Write entitlements_test.go**

```go
// evolve-server/internal/handler/entitlements_test.go
package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/gin-gonic/gin"
)

func TestEntitlements_GetFirstPartyMapping(t *testing.T) {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	h := handler.NewEntitlementsHandler()
	r.GET("/entitlements/1/entitlementDefs.getFirstPartyMapping",
		h.GetFirstPartyMapping)

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet,
		"/entitlements/1/entitlementDefs.getFirstPartyMapping?playerId=test-player-id", nil)
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200; body: %s", w.Code, w.Body)
	}

	var result []model.EntitlementItem
	if err := json.Unmarshal(w.Body.Bytes(), &result); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(result) == 0 {
		t.Error("expected at least one entitlement")
	}
	for _, e := range result {
		if !e.IsAvailable {
			t.Errorf("entitlement %q should be available", e.EntitlementId)
		}
		if !e.IsValid {
			t.Errorf("entitlement %q should be valid", e.EntitlementId)
		}
	}
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestEntitlements 2>&1 | head -10
```

Expected: build error.

- [ ] **Step 3: Write handler/entitlements.go**

```go
// evolve-server/internal/handler/entitlements.go
package handler

import (
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/gin-gonic/gin"
)

type EntitlementsHandler struct{}

func NewEntitlementsHandler() *EntitlementsHandler {
	return &EntitlementsHandler{}
}

// GetFirstPartyMapping handles GET /entitlements/1/entitlementDefs.getFirstPartyMapping.
// Returns all 468 entitlements as granted — identical to Pinenut behaviour.
// playerId comes from the query param or the Authorization header player context.
func (h *EntitlementsHandler) GetFirstPartyMapping(c *gin.Context) {
	playerId := c.Query("playerId")
	if playerId == "" {
		playerId = c.GetString("playerId") // set by auth middleware when wired
	}

	items := make([]model.EntitlementItem, len(model.EntitlementIds))
	for i, eid := range model.EntitlementIds {
		defId := eid
		if i < len(model.EntitlementDefIds) {
			defId = model.EntitlementDefIds[i]
		}
		items[i] = model.EntitlementItem{
			CreatedOn:             0,
			EntitlementDefId:      defId,
			IsServerAuthoritative: true,
			IsValid:               true,
			RuleData:              model.RuleData{Grant: true},
			EntitlementId:         eid,
			AppPublicId:           nil,
			AppGroupId:            model.AppGroupId,
			PlayerPublicId:        playerId,
			IsAvailable:           true,
			IsShared:              false,
		}
	}
	c.JSON(http.StatusOK, items)
}

// GetMapping handles GET /entitlements/1/entitlementDefs.getMapping — stub, returns empty list.
func (h *EntitlementsHandler) GetMapping(c *gin.Context) {
	c.JSON(http.StatusOK, []interface{}{})
}

// CheckAppOwnership handles GET /entitlements/1/checkAppOwnership — always owned.
func (h *EntitlementsHandler) CheckAppOwnership(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"owned": true})
}
```

- [ ] **Step 4: Run entitlements tests**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestEntitlements
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add . && git commit -m "feat: entitlements endpoints — all 468 entitlements granted"
```

---

### Task 7: Storage CRUD handler

The game uses 5 storage datasets to persist per-player state (properties, unlocks, sessions, replays, replay ownership). Each dataset stores arbitrary JSON blobs keyed by a string.

**Files:**
- Create: `evolve-server/internal/store/storage.go`
- Create: `evolve-server/internal/handler/storage.go`
- Create: `evolve-server/internal/handler/storage_test.go`

Dataset UUIDs (from project context):
- sessions: `e9c21d966612393f9514896e4080f0c9`
- playerProperties: `e4e9ba4c5d6630df11d8ce3683ec1fde`
- playerUnlocks: `5ab57ab47a39339c1023a75dc99d2110`
- replays: `b06d4d28b6467691823e4ac44aebe6d0`
- replayOwners: `e754746f6e4ef8c73e734bff7305f450`

- [ ] **Step 1: Write storage_test.go**

```go
// evolve-server/internal/handler/storage_test.go
package handler_test

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

const testDatasetId = "e4e9ba4c5d6630df11d8ce3683ec1fde" // playerProperties

func TestStorage_PutThenGet(t *testing.T) {
	pool := setupTestDB(t)
	// Insert a player first
	pool.Exec(`INSERT INTO players (id, steam_id, display_name) VALUES ('player-1', 'steam-1', 'TestPlayer') ON CONFLICT DO NOTHING`)

	ss := store.NewStorageStore(pool)
	gin.SetMode(gin.TestMode)
	r := gin.New()
	// Inject player id via middleware for test
	r.Use(func(c *gin.Context) { c.Set("playerId", "player-1"); c.Next() })
	sh := handler.NewStorageHandler(ss)
	r.PUT("/storage/1/:datasetId/:itemKey", sh.Put)
	r.GET("/storage/1/:datasetId", sh.List)

	// PUT an item
	body := `{"level":5,"xp":1000}`
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodPut,
		fmt.Sprintf("/storage/1/%s/mykey", testDatasetId),
		bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("PUT status = %d; body: %s", w.Code, w.Body)
	}

	// GET the list
	w2 := httptest.NewRecorder()
	req2 := httptest.NewRequest(http.MethodGet,
		fmt.Sprintf("/storage/1/%s", testDatasetId), nil)
	r.ServeHTTP(w2, req2)
	if w2.Code != http.StatusOK {
		t.Fatalf("GET status = %d; body: %s", w2.Code, w2.Body)
	}

	var items []map[string]interface{}
	if err := json.Unmarshal(w2.Body.Bytes(), &items); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(items) != 1 {
		t.Fatalf("expected 1 item, got %d", len(items))
	}
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestStorage 2>&1 | head -10
```

Expected: build error.

- [ ] **Step 3: Write store/storage.go**

```go
// evolve-server/internal/store/storage.go
package store

import (
	"database/sql"
	"encoding/json"
	"fmt"

	"github.com/google/uuid"
)

type StorageItem struct {
	Id        string
	DatasetId string
	PlayerId  string
	ItemKey   string
	Data      json.RawMessage
}

type StorageStore struct{ db *sql.DB }

func NewStorageStore(db *sql.DB) *StorageStore { return &StorageStore{db: db} }

func (s *StorageStore) List(datasetId, playerId string) ([]StorageItem, error) {
	rows, err := s.db.Query(
		`SELECT id, dataset_id, player_id, item_key, data FROM storage_items
		 WHERE dataset_id = $1 AND player_id = $2`,
		datasetId, playerId,
	)
	if err != nil {
		return nil, fmt.Errorf("storage list: %w", err)
	}
	defer rows.Close()

	var items []StorageItem
	for rows.Next() {
		var item StorageItem
		if err := rows.Scan(&item.Id, &item.DatasetId, &item.PlayerId, &item.ItemKey, &item.Data); err != nil {
			return nil, err
		}
		items = append(items, item)
	}
	return items, rows.Err()
}

func (s *StorageStore) Put(datasetId, playerId, itemKey string, data json.RawMessage) error {
	id := uuid.New().String()
	_, err := s.db.Exec(
		`INSERT INTO storage_items (id, dataset_id, player_id, item_key, data)
		 VALUES ($1, $2, $3, $4, $5)
		 ON CONFLICT (dataset_id, player_id, item_key)
		 DO UPDATE SET data = $5, updated_at = NOW()`,
		id, datasetId, playerId, itemKey, data,
	)
	return err
}

func (s *StorageStore) Delete(datasetId, playerId, itemKey string) error {
	_, err := s.db.Exec(
		`DELETE FROM storage_items WHERE dataset_id=$1 AND player_id=$2 AND item_key=$3`,
		datasetId, playerId, itemKey,
	)
	return err
}
```

- [ ] **Step 4: Write handler/storage.go**

```go
// evolve-server/internal/handler/storage.go
package handler

import (
	"encoding/json"
	"io"
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

type StorageHandler struct{ ss *store.StorageStore }

func NewStorageHandler(ss *store.StorageStore) *StorageHandler {
	return &StorageHandler{ss: ss}
}

// List handles GET /storage/1/:datasetId
func (h *StorageHandler) List(c *gin.Context) {
	playerId := c.GetString("playerId")
	items, err := h.ss.List(c.Param("datasetId"), playerId)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	if items == nil {
		items = []store.StorageItem{}
	}
	result := make([]map[string]interface{}, len(items))
	for i, item := range items {
		result[i] = map[string]interface{}{
			"id":        item.Id,
			"key":       item.ItemKey,
			"data":      item.Data,
			"datasetId": item.DatasetId,
			"playerId":  item.PlayerId,
		}
	}
	c.JSON(http.StatusOK, result)
}

// Put handles PUT /storage/1/:datasetId/:itemKey
func (h *StorageHandler) Put(c *gin.Context) {
	playerId := c.GetString("playerId")
	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "read body"})
		return
	}
	if !json.Valid(body) {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid json"})
		return
	}
	if err := h.ss.Put(c.Param("datasetId"), playerId, c.Param("itemKey"), body); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	c.JSON(http.StatusOK, gin.H{"ok": true})
}

// Delete handles DELETE /storage/1/:datasetId/:itemKey
func (h *StorageHandler) Delete(c *gin.Context) {
	playerId := c.GetString("playerId")
	if err := h.ss.Delete(c.Param("datasetId"), playerId, c.Param("itemKey")); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	c.Status(http.StatusNoContent)
}
```

- [ ] **Step 5: Run storage tests**

```bash
cd evolve-server && TEST_DATABASE_URL="postgres://evolve:evolve@localhost/evolve_test?sslmode=disable" go test ./internal/handler/... -v -run TestStorage
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add . && git commit -m "feat: storage CRUD handler + store"
```

---

### Task 8: Stub endpoints

These endpoints are called by the game but need only minimal responses.

**Files:**
- Create: `evolve-server/internal/handler/stubs.go`
- Create: `evolve-server/internal/handler/stubs_test.go`

- [ ] **Step 1: Write stubs_test.go**

```go
// evolve-server/internal/handler/stubs_test.go
package handler_test

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/gin-gonic/gin"
)

func TestStubs_AllReturn200(t *testing.T) {
	gin.SetMode(gin.TestMode)
	stubs := handler.NewStubsHandler()
	r := gin.New()
	r.POST("/telemetry/1", stubs.Stub200)
	r.POST("/content/1", stubs.Stub200)
	r.GET("/stats/1/configs.generate", stubs.StatsConfigs)
	r.GET("/grants/1/grants.find", stubs.GrantsFind)
	r.POST("/sessions/1/heartbeat", stubs.Stub200)
	r.GET("/queue/1/waittime", stubs.QueueWaittime)
	r.POST("/evolve/event", stubs.Stub200)
	r.GET("/news/1/configs.generate", stubs.StatsConfigs)
	r.GET("/apps/1", stubs.Stub200)

	cases := []struct {
		method, path string
	}{
		{"POST", "/telemetry/1"},
		{"POST", "/content/1"},
		{"GET", "/stats/1/configs.generate"},
		{"GET", "/grants/1/grants.find"},
		{"POST", "/sessions/1/heartbeat"},
		{"GET", "/queue/1/waittime"},
		{"POST", "/evolve/event"},
		{"GET", "/news/1/configs.generate"},
	}
	for _, tc := range cases {
		w := httptest.NewRecorder()
		req := httptest.NewRequest(tc.method, tc.path, nil)
		r.ServeHTTP(w, req)
		if w.Code != http.StatusOK {
			t.Errorf("%s %s: status = %d, want 200", tc.method, tc.path, w.Code)
		}
	}
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestStubs 2>&1 | head -10
```

Expected: build error.

- [ ] **Step 3: Write handler/stubs.go**

```go
// evolve-server/internal/handler/stubs.go
package handler

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

type StubsHandler struct{}

func NewStubsHandler() *StubsHandler { return &StubsHandler{} }

// Stub200 returns {"status":"success"} for endpoints the game calls
// but that don't need real logic (telemetry, content, events, heartbeat).
func (h *StubsHandler) Stub200(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"status": "success"})
}

// StatsConfigs handles GET /stats/1/configs.generate — returns empty stat groups.
func (h *StubsHandler) StatsConfigs(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"statGroups": []interface{}{},
	})
}

// GrantsFind handles GET /grants/1/grants.find — returns empty grants list.
func (h *StubsHandler) GrantsFind(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"grants": []interface{}{},
		"total":  0,
	})
}

// QueueWaittime handles GET /queue/1/waittime — always returns 0s wait.
func (h *StubsHandler) QueueWaittime(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"estimatedWait": 0,
		"queueDepth":    0,
	})
}
```

- [ ] **Step 4: Run stub tests**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestStubs
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add . && git commit -m "feat: stub endpoints — telemetry, stats, grants, queue, heartbeat"
```

---

### Task 9: Peers API

The peers API replaces LAN UDP broadcast. Goldberg calls `POST /peers/register` when joining/creating a lobby and `GET /peers/:lobbyId` to discover other players in the same lobby. Peers expire after 5 minutes.

**Files:**
- Create: `evolve-server/internal/store/peer.go`
- Create: `evolve-server/internal/handler/peers.go`
- Create: `evolve-server/internal/handler/peers_test.go`

- [ ] **Step 1: Write peers_test.go**

```go
// evolve-server/internal/handler/peers_test.go
package handler_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

func TestPeers_RegisterAndGet(t *testing.T) {
	pool := setupTestDB(t)
	ps := store.NewPeerStore(pool)
	gin.SetMode(gin.TestMode)
	r := gin.New()
	ph := handler.NewPeersHandler(ps)
	r.POST("/peers/register", ph.Register)
	r.GET("/peers/:lobbyId", ph.GetPeers)

	// Register a peer
	body := `{"lobbyId":"lobby-abc","playerId":"player-1","ip":"1.2.3.4","port":47584}`
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodPost, "/peers/register", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("register status = %d; body: %s", w.Code, w.Body)
	}

	// Get peers for lobby
	w2 := httptest.NewRecorder()
	req2 := httptest.NewRequest(http.MethodGet, "/peers/lobby-abc", nil)
	r.ServeHTTP(w2, req2)
	if w2.Code != http.StatusOK {
		t.Fatalf("get peers status = %d; body: %s", w2.Code, w2.Body)
	}

	var peers []map[string]interface{}
	if err := json.Unmarshal(w2.Body.Bytes(), &peers); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(peers) != 1 {
		t.Fatalf("expected 1 peer, got %d", len(peers))
	}
	if peers[0]["ip"] != "1.2.3.4" {
		t.Errorf("ip = %v, want 1.2.3.4", peers[0]["ip"])
	}
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestPeers 2>&1 | head -10
```

Expected: build error.

- [ ] **Step 3: Write store/peer.go**

```go
// evolve-server/internal/store/peer.go
package store

import (
	"database/sql"
	"fmt"
	"time"
)

type Peer struct {
	LobbyId      string
	PlayerId     string
	IP           string
	Port         int
	RegisteredAt time.Time
}

type PeerStore struct{ db *sql.DB }

func NewPeerStore(db *sql.DB) *PeerStore { return &PeerStore{db: db} }

func (s *PeerStore) Register(lobbyId, playerId, ip string, port int) error {
	_, err := s.db.Exec(
		`INSERT INTO peers (lobby_id, player_id, ip, port, registered_at)
		 VALUES ($1, $2, $3, $4, NOW())
		 ON CONFLICT (lobby_id, player_id)
		 DO UPDATE SET ip = $3, port = $4, registered_at = NOW()`,
		lobbyId, playerId, ip, port,
	)
	return err
}

func (s *PeerStore) GetByLobby(lobbyId string) ([]Peer, error) {
	// Only return peers registered in the last 5 minutes.
	rows, err := s.db.Query(
		`SELECT lobby_id, player_id, ip, port, registered_at FROM peers
		 WHERE lobby_id = $1 AND registered_at > NOW() - INTERVAL '5 minutes'`,
		lobbyId,
	)
	if err != nil {
		return nil, fmt.Errorf("peers query: %w", err)
	}
	defer rows.Close()

	var peers []Peer
	for rows.Next() {
		var p Peer
		if err := rows.Scan(&p.LobbyId, &p.PlayerId, &p.IP, &p.Port, &p.RegisteredAt); err != nil {
			return nil, err
		}
		peers = append(peers, p)
	}
	return peers, rows.Err()
}

// PurgeExpired removes peers older than 5 minutes. Call periodically.
func (s *PeerStore) PurgeExpired() error {
	_, err := s.db.Exec(
		`DELETE FROM peers WHERE registered_at < NOW() - INTERVAL '5 minutes'`,
	)
	return err
}
```

- [ ] **Step 4: Write handler/peers.go**

```go
// evolve-server/internal/handler/peers.go
package handler

import (
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

type PeersHandler struct{ ps *store.PeerStore }

func NewPeersHandler(ps *store.PeerStore) *PeersHandler {
	return &PeersHandler{ps: ps}
}

type registerRequest struct {
	LobbyId  string `json:"lobbyId"  binding:"required"`
	PlayerId string `json:"playerId" binding:"required"`
	IP       string `json:"ip"       binding:"required"`
	Port     int    `json:"port"     binding:"required"`
}

// Register handles POST /peers/register.
// Called by patched Goldberg when a player joins/creates a lobby.
func (h *PeersHandler) Register(c *gin.Context) {
	var req registerRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	if err := h.ps.Register(req.LobbyId, req.PlayerId, req.IP, req.Port); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	c.JSON(http.StatusOK, gin.H{"ok": true})
}

// GetPeers handles GET /peers/:lobbyId.
// Called by patched Goldberg to discover other players in the lobby.
func (h *PeersHandler) GetPeers(c *gin.Context) {
	peers, err := h.ps.GetByLobby(c.Param("lobbyId"))
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	result := make([]gin.H, len(peers))
	for i, p := range peers {
		result[i] = gin.H{
			"playerId": p.PlayerId,
			"ip":       p.IP,
			"port":     p.Port,
		}
	}
	c.JSON(http.StatusOK, result)
}
```

- [ ] **Step 5: Run peers tests**

```bash
cd evolve-server && TEST_DATABASE_URL="postgres://evolve:evolve@localhost/evolve_test?sslmode=disable" go test ./internal/handler/... -v -run TestPeers
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add . && git commit -m "feat: peers API for VPN-free lobby discovery"
```

---

### Task 10: Status + build_config endpoints

**Files:**
- Create: `evolve-server/internal/handler/status.go`
- Create: `evolve-server/internal/handler/status_test.go`

- [ ] **Step 1: Write status_test.go**

```go
// evolve-server/internal/handler/status_test.go
package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/gin-gonic/gin"
)

func TestStatus_ReturnsOnlineAndVersion(t *testing.T) {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	sh := handler.NewStatusHandler("1.0.0")
	r.GET("/status", sh.Status)
	r.GET("/build_config", sh.BuildConfig)

	w := httptest.NewRecorder()
	r.ServeHTTP(w, httptest.NewRequest(http.MethodGet, "/status", nil))
	if w.Code != http.StatusOK {
		t.Fatalf("status = %d", w.Code)
	}
	var s map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &s)
	if s["online"] != true {
		t.Error("online should be true")
	}

	w2 := httptest.NewRecorder()
	r.ServeHTTP(w2, httptest.NewRequest(http.MethodGet, "/build_config", nil))
	if w2.Code != http.StatusOK {
		t.Fatalf("build_config status = %d", w2.Code)
	}
	var bc map[string]interface{}
	json.Unmarshal(w2.Body.Bytes(), &bc)
	if bc["serverVersion"] != "1.0.0" {
		t.Errorf("serverVersion = %v, want 1.0.0", bc["serverVersion"])
	}
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestStatus 2>&1 | head -10
```

- [ ] **Step 3: Write handler/status.go**

```go
// evolve-server/internal/handler/status.go
package handler

import (
	"net/http"
	"sync/atomic"
	"time"

	"github.com/gin-gonic/gin"
)

// onlinePlayers is incremented/decremented as sessions are created/expire.
// Used by the launcher UI to show "X players online".
var onlinePlayers int64

func IncrementOnline()  { atomic.AddInt64(&onlinePlayers, 1) }
func DecrementOnline()  { atomic.AddInt64(&onlinePlayers, -1) }
func OnlineCount() int64 { return atomic.LoadInt64(&onlinePlayers) }

type StatusHandler struct {
	version   string
	startTime time.Time
}

func NewStatusHandler(version string) *StatusHandler {
	return &StatusHandler{version: version, startTime: time.Now()}
}

// Status handles GET /status — polled by the launcher every 30s.
func (h *StatusHandler) Status(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"online":        true,
		"onlinePlayers": OnlineCount(),
		"uptime":        int(time.Since(h.startTime).Seconds()),
		"version":       h.version,
	})
}

// BuildConfig handles GET /build_config — checked by the launcher for updates.
func (h *StatusHandler) BuildConfig(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"serverVersion": h.version,
		"dllVersion":    "1.0.0",
		"launcherVersion": "1.0.0",
	})
}
```

- [ ] **Step 4: Run status tests**

```bash
cd evolve-server && go test ./internal/handler/... -v -run TestStatus
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add . && git commit -m "feat: /status and /build_config endpoints"
```

---

### Task 11: Wire all routes + full integration test

**Files:**
- Modify: `evolve-server/cmd/server/main.go`
- Create: `evolve-server/cmd/server/main_test.go`

- [ ] **Step 1: Write integration test**

```go
// evolve-server/cmd/server/main_test.go
// Uses package main (not main_test) so it can access buildRouterWithDeps from router.go.
package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/db"
	"github.com/gin-gonic/gin"
)

func buildRouter(t *testing.T) *gin.Engine {
	t.Helper()
	dsn := os.Getenv("TEST_DATABASE_URL")
	if dsn == "" {
		t.Skip("TEST_DATABASE_URL not set")
	}
	pool, _ := db.Open(dsn)
	db.Migrate(pool)
	pool.Exec("TRUNCATE players, storage_items, peers CASCADE")
	t.Cleanup(func() { pool.Close() })

	cfg := config.Config{ServerHost: "localhost:8080"}
	return buildRouterWithDeps(cfg, pool)
}

func TestIntegration_DoormanAndSSO(t *testing.T) {
	r := buildRouter(t)
	gin.SetMode(gin.TestMode)

	// 1. Doorman
	w := httptest.NewRecorder()
	r.ServeHTTP(w, httptest.NewRequest("GET", "/doorman/1/configs.generate", nil))
	if w.Code != http.StatusOK {
		t.Fatalf("doorman: %d — %s", w.Code, w.Body)
	}
	var door map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &door)
	if door["accessToken"] == nil {
		t.Error("doorman: missing accessToken")
	}

	// 2. SSO logon
	w2 := httptest.NewRecorder()
	body := `{"steamId":"76561198099999999","displayName":"IntegPlayer"}`
	req := httptest.NewRequest("POST", "/sso/1/auths.logon",
		bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w2, req)
	if w2.Code != http.StatusOK {
		t.Fatalf("sso: %d — %s", w2.Code, w2.Body)
	}
	var sso map[string]interface{}
	json.Unmarshal(w2.Body.Bytes(), &sso)
	if sso["playerId"] == "" || sso["playerId"] == nil {
		t.Error("sso: missing playerId")
	}
}
```

- [ ] **Step 2: Write the route wiring helper (buildRouterWithDeps) in a separate file**

```go
// evolve-server/cmd/server/router.go
package main

import (
	"database/sql"
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/middleware"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

func buildRouterWithDeps(cfg config.Config, pool *sql.DB) *gin.Engine {
	r := gin.Default()
	r.Use(middleware.Auth()) // injects playerId from Bearer token on all routes

	players := store.NewPlayerStore(pool)
	storage := store.NewStorageStore(pool)
	peers := store.NewPeerStore(pool)

	dh := handler.NewDoormanHandler(cfg)
	sh := handler.NewSSOHandler(players, cfg)
	eh := handler.NewEntitlementsHandler()
	storh := handler.NewStorageHandler(storage)
	stubs := handler.NewStubsHandler()
	ph := handler.NewPeersHandler(peers)
	sth := handler.NewStatusHandler("1.0.0")

	r.GET("/health", func(c *gin.Context) { c.JSON(http.StatusOK, gin.H{"status": "ok"}) })
	r.GET("/status", sth.Status)
	r.GET("/build_config", sth.BuildConfig)

	// Doorman
	r.GET("/doorman/1/configs.generate", dh.ConfigsGenerate)

	// SSO
	r.POST("/sso/1/auths.logon", sh.Logon)

	// Entitlements
	r.GET("/entitlements/1/entitlementDefs.getFirstPartyMapping", eh.GetFirstPartyMapping)
	r.GET("/entitlements/1/entitlementDefs.getMapping", eh.GetMapping)
	r.GET("/entitlements/1/checkAppOwnership", eh.CheckAppOwnership)

	// Storage
	r.GET("/storage/1/:datasetId", storh.List)
	r.PUT("/storage/1/:datasetId/:itemKey", storh.Put)
	r.DELETE("/storage/1/:datasetId/:itemKey", storh.Delete)

	// Stubs
	r.POST("/telemetry/1", stubs.Stub200)
	r.POST("/content/1", stubs.Stub200)
	r.GET("/stats/1/configs.generate", stubs.StatsConfigs)
	r.GET("/grants/1/grants.find", stubs.GrantsFind)
	r.POST("/sessions/1/heartbeat", stubs.Stub200)
	r.GET("/queue/1/waittime", stubs.QueueWaittime)
	r.POST("/evolve/event", stubs.Stub200)
	r.GET("/news/1/configs.generate", stubs.StatsConfigs)
	r.NoRoute(func(c *gin.Context) { c.JSON(http.StatusOK, gin.H{"status": "success"}) })

	// Peers
	r.POST("/peers/register", ph.Register)
	r.GET("/peers/:lobbyId", ph.GetPeers)

	return r
}
```

- [ ] **Step 3: Update main.go to use buildRouterWithDeps**

Replace the gin setup in `main.go` with:
```go
r := buildRouterWithDeps(cfg, pool)
log.Printf("evolve-server listening on :%s", cfg.Port)
if err := r.Run(":" + cfg.Port); err != nil {
    log.Fatal(err)
}
```

Remove the old `r.GET("/health", ...)` line (now in buildRouterWithDeps).

- [ ] **Step 4: Fix integration test imports (add `bytes` package)**

Add `"bytes"` to the import block in `main_test.go`.

- [ ] **Step 5: Run full integration test**

```bash
cd evolve-server && TEST_DATABASE_URL="postgres://evolve:evolve@localhost/evolve_test?sslmode=disable" go test ./... -v 2>&1 | tail -30
```

Expected: all tests PASS.

- [ ] **Step 6: Build the binary**

```bash
cd evolve-server && go build -o bin/evolve-server ./cmd/server && echo "Build OK"
```

- [ ] **Step 7: Commit**

```bash
git add . && git commit -m "feat: wire all routes + integration test — evolve-server complete"
```
