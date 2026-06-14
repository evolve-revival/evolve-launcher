package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/gin-gonic/gin"
)

func TestStatus(t *testing.T) {
	gin.SetMode(gin.TestMode)
	h := handler.NewStatusHandler("dev")
	r := gin.New()
	r.GET("/status", h.Status)
	r.GET("/build_config", h.BuildConfig)

	for _, path := range []string{"/status", "/build_config"} {
		w := httptest.NewRecorder()
		r.ServeHTTP(w, httptest.NewRequest(http.MethodGet, path, nil))
		if w.Code != http.StatusOK {
			t.Errorf("GET %s: status = %d", path, w.Code)
		}
		var resp map[string]interface{}
		if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
			t.Errorf("GET %s: invalid JSON: %v", path, err)
		}
	}
}
