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

	log.Printf("evolve-server listening on :%s", cfg.Port)
	if err := r.Run(":" + cfg.Port); err != nil {
		log.Fatalf("server: %v", err)
	}
}
