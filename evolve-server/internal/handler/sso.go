package handler

import (
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/middleware"
	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

type SSOHandler struct {
	players *store.PlayerStore
}

func NewSSOHandler(players *store.PlayerStore) *SSOHandler {
	return &SSOHandler{players: players}
}

// Logon handles POST /sso/1/logon/:game
// The client sends steamId and displayName; we upsert a player row, mint a token,
// and return an SSOResponse so all subsequent requests can authenticate.
func (h *SSOHandler) Logon(c *gin.Context) {
	var req struct {
		SteamId     string `json:"steamId"`
		DisplayName string `json:"displayName"`
	}
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	player, err := h.players.UpsertBySteamId(req.SteamId, req.DisplayName)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	token := uuid.New().String()
	middleware.StoreToken(token, player.Id)

	c.JSON(http.StatusOK, model.SSOResponse{
		IsNewPlayer: player.IsNew,
		HasPlayedApp: !player.IsNew,
		AccessToken: token,
		ExpiresIn:   86400,
		TokenType:   "Bearer",
		PlayerId:    player.Id,
		SessionId:   uuid.New().String(),
	})
}
