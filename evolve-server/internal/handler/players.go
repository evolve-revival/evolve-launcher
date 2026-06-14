package handler

import (
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

type PlayersHandler struct {
	players *store.PlayerStore
}

func NewPlayersHandler(players *store.PlayerStore) *PlayersHandler {
	return &PlayersHandler{players: players}
}

// Get handles GET /players/1/:playerId
// Returns player profile data so the lobby UI can display names.
func (h *PlayersHandler) Get(c *gin.Context) {
	id := c.Param("playerId")
	player, err := h.players.GetById(id)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	if player == nil {
		c.JSON(http.StatusNotFound, gin.H{"error": "player not found"})
		return
	}
	c.JSON(http.StatusOK, gin.H{
		"playerId":       player.Id,
		"playerPublicId": player.Id,
		"displayName":    player.DisplayName,
		"xp":             0,
		"level":          1,
		"avatarUrl":      "",
	})
}
