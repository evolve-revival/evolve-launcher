package handler

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

// StubsHandler returns 200 OK for endpoints the client contacts but that require
// no real implementation for basic multiplayer to work.
type StubsHandler struct{}

func NewStubsHandler() *StubsHandler { return &StubsHandler{} }

func (h *StubsHandler) Stub200(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{})
}

func (h *StubsHandler) StatsConfigs(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"configs": []interface{}{}})
}

func (h *StubsHandler) GrantsFind(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"grants": []interface{}{}})
}

func (h *StubsHandler) QueueWaittime(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"waitTimeSeconds": 0})
}

func (h *StubsHandler) Heartbeat(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"ok": true})
}
