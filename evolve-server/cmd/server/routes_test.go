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
