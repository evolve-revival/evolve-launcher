package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/middleware"
	"github.com/gin-gonic/gin"
)

func setupEntitlements(t *testing.T) *gin.Engine {
	t.Helper()
	gin.SetMode(gin.TestMode)
	h := handler.NewEntitlementsHandler()
	r := gin.New()
	r.Use(middleware.Auth())
	r.GET("/entitlements/1/firstPartyMapping/:platform/:platformId", h.GetFirstPartyMapping)
	r.GET("/entitlements/1/mapping/:appGroupId", h.GetMapping)
	r.GET("/entitlements/1/appOwnership/:appGroupId", h.CheckAppOwnership)
	return r
}

func TestEntitlements_GetMapping(t *testing.T) {
	r := setupEntitlements(t)

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/entitlements/1/mapping/c3dc178f670ee769fe59e244610d66e2", nil)
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d; body = %s", w.Code, w.Body)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatal(err)
	}
	items := resp["entitlements"].([]interface{})
	if len(items) == 0 {
		t.Fatal("expected non-empty entitlements")
	}
}

func TestEntitlements_CheckAppOwnership(t *testing.T) {
	r := setupEntitlements(t)

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/entitlements/1/appOwnership/c3dc178f670ee769fe59e244610d66e2", nil)
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d", w.Code)
	}
	var resp map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &resp)
	if resp["ownsApp"] != true {
		t.Errorf("ownsApp = %v, want true", resp["ownsApp"])
	}
}
