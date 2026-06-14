package handler_test

import (
	"bytes"
	"database/sql"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/handler"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

func setupSSO(t *testing.T) (*gin.Engine, *sql.DB) {
	t.Helper()
	db := openTestDB(t)
	players := store.NewPlayerStore(db)
	h := handler.NewSSOHandler(players)

	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.POST("/sso/1/logon/:game", h.Logon)
	return r, db
}

func TestSSO_Logon_NewPlayer(t *testing.T) {
	r, _ := setupSSO(t)

	body, _ := json.Marshal(map[string]string{
		"steamId":     "76561198000000001",
		"displayName": "TestPlayer",
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodPost, "/sso/1/logon/evolve", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d; body = %s", w.Code, w.Body)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatal(err)
	}
	if resp["accessToken"] == "" || resp["accessToken"] == nil {
		t.Error("expected non-empty accessToken")
	}
	if resp["playerId"] == "" || resp["playerId"] == nil {
		t.Error("expected non-empty playerId")
	}
	if resp["isNewPlayer"] != true {
		t.Errorf("isNewPlayer = %v, want true", resp["isNewPlayer"])
	}
}

func TestSSO_Logon_ExistingPlayer(t *testing.T) {
	r, _ := setupSSO(t)

	body, _ := json.Marshal(map[string]string{
		"steamId":     "76561198000000002",
		"displayName": "ExistingPlayer",
	})

	// First logon creates the player
	w1 := httptest.NewRecorder()
	req1 := httptest.NewRequest(http.MethodPost, "/sso/1/logon/evolve", bytes.NewReader(body))
	req1.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w1, req1)
	if w1.Code != http.StatusOK {
		t.Fatalf("first logon status = %d", w1.Code)
	}

	var resp1 map[string]interface{}
	json.Unmarshal(w1.Body.Bytes(), &resp1)
	firstId := resp1["playerId"].(string)

	// Second logon returns same player
	body2, _ := json.Marshal(map[string]string{
		"steamId":     "76561198000000002",
		"displayName": "ExistingPlayer",
	})
	w2 := httptest.NewRecorder()
	req2 := httptest.NewRequest(http.MethodPost, "/sso/1/logon/evolve", bytes.NewReader(body2))
	req2.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w2, req2)
	if w2.Code != http.StatusOK {
		t.Fatalf("second logon status = %d", w2.Code)
	}

	var resp2 map[string]interface{}
	json.Unmarshal(w2.Body.Bytes(), &resp2)

	if resp2["playerId"] != firstId {
		t.Errorf("playerId mismatch: got %s, want %s", resp2["playerId"], firstId)
	}
	if resp2["isNewPlayer"] != false {
		t.Errorf("isNewPlayer = %v, want false", resp2["isNewPlayer"])
	}
}
