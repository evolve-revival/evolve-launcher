package handler

import (
	"net/http"
	"runtime"

	"github.com/gin-gonic/gin"
)

type StatusHandler struct {
	version string
}

func NewStatusHandler(version string) *StatusHandler {
	return &StatusHandler{version: version}
}

func (h *StatusHandler) Status(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":  "ok",
		"version": h.version,
	})
}

func (h *StatusHandler) BuildConfig(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"version":   h.version,
		"goVersion": runtime.Version(),
		"os":        runtime.GOOS,
		"arch":      runtime.GOARCH,
	})
}
