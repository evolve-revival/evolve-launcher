package main

import (
	"log"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/db"
	"github.com/evolve-revival/evolve-server/internal/relay"
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

	// Start UDP relay for Goldberg peer discovery alongside the HTTP server.
	go func() {
		if err := relay.New().Run(":" + cfg.RelayPort); err != nil {
			log.Fatalf("relay: %v", err)
		}
	}()

	r := buildRouterWithDeps(cfg, pool)

	log.Printf("evolve-server listening on :%s", cfg.Port)
	if err := r.Run(":" + cfg.Port); err != nil {
		log.Fatalf("server: %v", err)
	}
}
