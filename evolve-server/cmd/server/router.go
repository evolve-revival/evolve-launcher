package main

import (
	"database/sql"

	"github.com/evolve-revival/evolve-server/internal/config"
	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/middleware"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

func buildRouterWithDeps(cfg config.Config, pool *sql.DB) *gin.Engine {
	players := store.NewPlayerStore(pool)
	storage := store.NewStorageStore(pool)

	sso := handler.NewSSOHandler(players)
	doorman := handler.NewDoormanHandler(cfg.ServerHost)
	entitlements := handler.NewEntitlementsHandler()
	stor := handler.NewStorageHandler(storage)
	playersH := handler.NewPlayersHandler(players)
	stubs := handler.NewStubsHandler()
	status := handler.NewStatusHandler("1.0.0")

	r := gin.Default()
	r.Use(middleware.Auth())

	// Health
	r.GET("/status", status.Status)
	r.GET("/build_config", status.BuildConfig)

	// Doorman
	r.GET("/doorman/1/configs/generate", doorman.ConfigsGenerate)

	// SSO
	r.POST("/sso/1/logon/:game", sso.Logon)

	// Entitlements
	r.GET("/entitlements/1/firstPartyMapping/:platform/:platformId", entitlements.GetFirstPartyMapping)
	r.GET("/entitlements/1/mapping/:appGroupId", entitlements.GetMapping)
	r.GET("/entitlements/1/appOwnership/:appGroupId", entitlements.CheckAppOwnership)

	// Storage
	r.GET("/storage/1/data/:datasetId", stor.List)
	r.PUT("/storage/1/data/:datasetId/:key", stor.Put)
	r.DELETE("/storage/1/data/:datasetId/:key", stor.Delete)

	// Players
	r.GET("/players/1/:playerId", playersH.Get)
	r.Any("/players/1/:playerId/*subpath", stubs.Stub200)

	// Stubs
	r.POST("/telemetry/1/event", stubs.Stub200)
	r.GET("/stats/1/configs", stubs.StatsConfigs)
	r.POST("/grants/1/find", stubs.GrantsFind)
	r.GET("/queue/waittime", stubs.QueueWaittime)
	r.POST("/heartbeat", stubs.Heartbeat)

	// Wildcard stubs
	r.Any("/apps/1/*path", stubs.Stub200)
	r.Any("/content/1/*path", stubs.Stub200)
	r.Any("/storefront/1/*path", stubs.Stub200)
	r.Any("/sessions/1/*path", stubs.Stub200)
	r.Any("/news/1/*path", stubs.Stub200)

	return r
}
