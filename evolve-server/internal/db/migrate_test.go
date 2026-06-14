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
	// Idempotency
	if err := db.Migrate(pool); err != nil {
		t.Fatalf("re-migrate: %v", err)
	}
}
