package db

import (
	"database/sql"
	_ "embed"
	"fmt"
)

//go:embed migrations/001_init.sql
var initSQL string

// Migrate runs all pending migrations. Safe to call on every startup.
func Migrate(pool *sql.DB) error {
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
		return nil
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
