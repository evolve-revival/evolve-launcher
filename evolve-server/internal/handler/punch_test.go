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
	// IP comes from RemoteAddr, not from the request body.
	body := `{"id":"player1","port":12345}`
	w := httptest.NewRecorder()
	req, _ := http.NewRequest(http.MethodPost, "/peers/register", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	req.RemoteAddr = "1.2.3.4:54321"
	router.ServeHTTP(w, req)

	if w.Code != http.StatusNoContent {
		t.Fatalf("got %d want 204: %s", w.Code, w.Body.String())
	}
	addr := rel.LookupNamed("player1")
	if addr == nil {
		t.Fatal("peer not stored in registry")
	}
	if addr.Port != 12345 {
		t.Errorf("port: got %d want 12345", addr.Port)
	}
	// IP must come from RemoteAddr, not any body field.
	if addr.IP.String() != "1.2.3.4" {
		t.Errorf("ip: got %s want 1.2.3.4 (spoof fix broken)", addr.IP)
	}
}

// TestPunchRegister_spoofIgnored verifies that a body "ip" field is ignored:
// the stored IP must match RemoteAddr, not the spoofed body value.
func TestPunchRegister_spoofIgnored(t *testing.T) {
	router, rel := newPunchRouter(t)
	// Body contains a spoofed IP that differs from RemoteAddr.
	body := `{"id":"spoofer","ip":"9.9.9.9","port":9999}`
	w := httptest.NewRecorder()
	req, _ := http.NewRequest(http.MethodPost, "/peers/register", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	req.RemoteAddr = "1.2.3.4:54321"
	router.ServeHTTP(w, req)

	if w.Code != http.StatusNoContent {
		t.Fatalf("got %d want 204: %s", w.Code, w.Body.String())
	}
	addr := rel.LookupNamed("spoofer")
	if addr == nil {
		t.Fatal("peer not stored in registry")
	}
	// Must be RemoteAddr IP, not the spoofed body IP.
	if addr.IP.String() != "1.2.3.4" {
		t.Errorf("spoof not ignored: got %s want 1.2.3.4", addr.IP)
	}
}

// TestPunchRegister_noRemoteAddr verifies a 400 when the server cannot
// determine the client IP (e.g. RemoteAddr is empty/unparseable).
func TestPunchRegister_noRemoteAddr(t *testing.T) {
	router, _ := newPunchRouter(t)
	body := `{"id":"x","port":1}`
	w := httptest.NewRecorder()
	req, _ := http.NewRequest(http.MethodPost, "/peers/register", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	// Leave RemoteAddr empty to trigger the "could not determine client IP" path.
	req.RemoteAddr = ""
	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Fatalf("got %d want 400", w.Code)
	}
	if !strings.Contains(w.Body.String(), "could not determine client IP") {
		t.Errorf("unexpected body: %s", w.Body.String())
	}
}
