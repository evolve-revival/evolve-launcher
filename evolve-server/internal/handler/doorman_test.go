package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/gin-gonic/gin"
)

func TestDoorman_ConfigsGenerate(t *testing.T) {
	gin.SetMode(gin.TestMode)
	h := handler.NewDoormanHandler("localhost:8080")
	r := gin.New()
	r.GET("/doorman/1/configs/generate", h.ConfigsGenerate)

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/doorman/1/configs/generate", nil)
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d; body = %s", w.Code, w.Body)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatal(err)
	}

	services, ok := resp["services"].([]interface{})
	if !ok || len(services) == 0 {
		t.Fatal("expected non-empty services array")
	}

	names := make(map[string]bool)
	for _, svc := range services {
		m := svc.(map[string]interface{})
		names[m["serviceName"].(string)] = true
	}

	for _, required := range []string{"Doorman", "Sso", "Entitlements", "Storage", "Peers"} {
		if !names[required] {
			t.Errorf("missing service %q in response", required)
		}
	}
}
