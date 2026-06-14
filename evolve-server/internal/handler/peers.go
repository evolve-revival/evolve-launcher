package handler

import (
	"net/http"
	"strconv"

	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

type PeersHandler struct {
	peers *store.PeerStore
}

func NewPeersHandler(peers *store.PeerStore) *PeersHandler {
	return &PeersHandler{peers: peers}
}

// Register handles POST /peers/register
// Body: { "lobbyId": "...", "port": 27015 }
func (h *PeersHandler) Register(c *gin.Context) {
	playerId := c.GetString("playerId")

	var req struct {
		LobbyId string `json:"lobbyId"`
		Port    int    `json:"port"`
	}
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	ip := c.ClientIP()
	if err := h.peers.Register(req.LobbyId, playerId, ip, req.Port); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	c.JSON(http.StatusOK, gin.H{"ok": true})
}

// GetPeers handles GET /peers/:lobbyId
func (h *PeersHandler) GetPeers(c *gin.Context) {
	lobbyId := c.Param("lobbyId")
	peers, err := h.peers.GetByLobby(lobbyId)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	result := make([]map[string]interface{}, len(peers))
	for i, p := range peers {
		result[i] = map[string]interface{}{
			"lobbyId":  p.LobbyId,
			"playerId": p.PlayerId,
			"ip":       p.IP,
			"port":     strconv.Itoa(p.Port),
		}
	}
	c.JSON(http.StatusOK, gin.H{"peers": result})
}
