package handler_test

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/gin-gonic/gin"
)

func TestStubs_Return200(t *testing.T) {
	gin.SetMode(gin.TestMode)
	h := handler.NewStubsHandler()
	r := gin.New()
	r.POST("/telemetry/1/event", h.Stub200)
	r.GET("/stats/1/configs", h.StatsConfigs)
	r.POST("/grants/1/find", h.GrantsFind)
	r.GET("/queue/1/waittime", h.QueueWaittime)
	r.POST("/heartbeat", h.Heartbeat)

	for _, path := range []string{"/stats/1/configs", "/queue/1/waittime"} {
		w := httptest.NewRecorder()
		req := httptest.NewRequest(http.MethodGet, path, nil)
		r.ServeHTTP(w, req)
		if w.Code != http.StatusOK {
			t.Errorf("GET %s: status = %d", path, w.Code)
		}
	}

	for _, path := range []string{"/telemetry/1/event", "/grants/1/find", "/heartbeat"} {
		w := httptest.NewRecorder()
		req := httptest.NewRequest(http.MethodPost, path, nil)
		r.ServeHTTP(w, req)
		if w.Code != http.StatusOK {
			t.Errorf("POST %s: status = %d", path, w.Code)
		}
	}
}
