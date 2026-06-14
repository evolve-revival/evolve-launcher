package handler_test

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/relay"
	"github.com/gin-gonic/gin"
)

func newPunchRouter(t *testing.T) (*gin.Engine, *relay.Relay) {
	t.Helper()
	gin.SetMode(gin.TestMode)
	rel := relay.New()
	h := handler.NewPunchHandler(rel)
	r := gin.New()
	r.POST("/peers/register", h.Register)
	return r, rel
}

func TestPunchRegister_ok(t *testing.T) {
	router, rel := newPunchRouter(t)
	body := `{"id":"player1","ip":"1.2.3.4","port":12345}`
	w := httptest.NewRecorder()
	req, _ := http.NewRequest(http.MethodPost, "/peers/register", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	router.ServeHTTP(w, req)

	if w.Code != http.StatusNoContent {
		t.Fatalf("got %d want 204", w.Code)
	}
	addr := rel.LookupNamed("player1")
	if addr == nil {
		t.Fatal("peer not stored in registry")
	}
	if addr.Port != 12345 {
		t.Errorf("port: got %d want 12345", addr.Port)
	}
}

func TestPunchRegister_invalidIP(t *testing.T) {
	router, _ := newPunchRouter(t)
	body := `{"id":"x","ip":"not-an-ip","port":1}`
	w := httptest.NewRecorder()
	req, _ := http.NewRequest(http.MethodPost, "/peers/register", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Fatalf("got %d want 400", w.Code)
	}
	if !strings.Contains(w.Body.String(), "invalid IP") {
		t.Errorf("body: %s", w.Body.String())
	}
}
