package handler_test

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/middleware"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

func setupPeers(t *testing.T) *gin.Engine {
	t.Helper()
	db := openTestDB(t)
	h := handler.NewPeersHandler(store.NewPeerStore(db))

	middleware.StoreToken("peers-tok", "player-peers-id")

	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.Use(middleware.Auth())
	r.POST("/peers/register", h.Register)
	r.GET("/peers/:lobbyId", h.GetPeers)
	return r
}

func TestPeers_RegisterAndGet(t *testing.T) {
	r := setupPeers(t)

	lobbyId := "lobby-abc-123"

	// Register
	body, _ := json.Marshal(map[string]interface{}{
		"lobbyId": lobbyId,
		"port":    27015,
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodPost, "/peers/register", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer peers-tok")
	r.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("register status = %d; body = %s", w.Code, w.Body)
	}

	// Get
	w2 := httptest.NewRecorder()
	req2 := httptest.NewRequest(http.MethodGet, "/peers/"+lobbyId, nil)
	r.ServeHTTP(w2, req2)
	if w2.Code != http.StatusOK {
		t.Fatalf("get status = %d; body = %s", w2.Code, w2.Body)
	}

	var resp map[string]interface{}
	json.Unmarshal(w2.Body.Bytes(), &resp)
	peers := resp["peers"].([]interface{})
	if len(peers) == 0 {
		t.Error("expected at least 1 peer")
	}
}
