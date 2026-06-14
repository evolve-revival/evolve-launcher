package handler

import (
	"net"
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/relay"
	"github.com/gin-gonic/gin"
)

type PunchHandler struct {
	relay *relay.Relay
}

func NewPunchHandler(r *relay.Relay) *PunchHandler {
	return &PunchHandler{relay: r}
}

type registerRequest struct {
	ID   string `json:"id"   binding:"required"`
	Port int    `json:"port" binding:"required"`
}

// Register stores the caller's external IP:port under their session ID and
// triggers hole-punch signals between them and all other registered peers.
// The IP is derived from the actual client connection, never from the request
// body, to prevent spoofed IP registration and relay amplification attacks.
func (h *PunchHandler) Register(c *gin.Context) {
	var req registerRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	clientIP := c.ClientIP()
	ip := net.ParseIP(clientIP)
	if ip == nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "could not determine client IP"})
		return
	}

	addr := &net.UDPAddr{IP: ip, Port: req.Port}
	h.relay.RegisterNamed(req.ID, addr)
	c.Status(http.StatusNoContent)
}
