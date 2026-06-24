package main

import (
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/db"
)

func TestIntegration_StatusEndpoint(t *testing.T) {
	dsn := os.Getenv("TEST_DATABASE_URL")
	if dsn == "" {
		t.Skip("TEST_DATABASE_URL not set")
	}

	pool, err := db.Open(dsn)
	if err != nil {
		t.Fatalf("db: %v", err)
	}
	defer pool.Close()

	if err := db.Migrate(pool); err != nil {
		t.Fatalf("migrate: %v", err)
	}

	cfg := config.Config{Port: "8080", ServerHost: "localhost:8080"}
	r := buildRouterWithDeps(cfg, pool)

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/status", nil)
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d; body = %s", w.Code, w.Body)
	}
}

func TestIntegration_DoormanConfigsGenerate(t *testing.T) {
	dsn := os.Getenv("TEST_DATABASE_URL")
	if dsn == "" {
		t.Skip("TEST_DATABASE_URL not set")
	}

	pool, err := db.Open(dsn)
	if err != nil {
		t.Fatalf("db: %v", err)
	}
	defer pool.Close()

	if err := db.Migrate(pool); err != nil {
		t.Fatalf("migrate: %v", err)
	}

	cfg := config.Config{Port: "8080", ServerHost: "localhost:8080"}
	r := buildRouterWithDeps(cfg, pool)

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/doorman/1/configs/generate", nil)
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d; body = %s", w.Code, w.Body)
	}
}
