package handler

import (
	"encoding/json"
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/store"
	"github.com/gin-gonic/gin"
)

type StorageHandler struct {
	storage *store.StorageStore
}

func NewStorageHandler(storage *store.StorageStore) *StorageHandler {
	return &StorageHandler{storage: storage}
}

// List handles GET /storage/1/data/:datasetId
func (h *StorageHandler) List(c *gin.Context) {
	playerId := c.GetString("playerId")
	datasetId := c.Param("datasetId")

	items, err := h.storage.List(datasetId, playerId)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	result := make([]map[string]interface{}, len(items))
	for i, item := range items {
		result[i] = map[string]interface{}{
			"id":        item.Id,
			"datasetId": item.DatasetId,
			"playerId":  item.PlayerId,
			"key":       item.ItemKey,
			"data":      json.RawMessage(item.Data),
		}
	}
	c.JSON(http.StatusOK, gin.H{"items": result})
}

// Put handles PUT /storage/1/data/:datasetId/:key
func (h *StorageHandler) Put(c *gin.Context) {
	playerId := c.GetString("playerId")
	datasetId := c.Param("datasetId")
	itemKey := c.Param("key")

	body, err := c.GetRawData()
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "cannot read body"})
		return
	}
	if !json.Valid(body) {
		c.JSON(http.StatusBadRequest, gin.H{"error": "body must be valid JSON"})
		return
	}

	if err := h.storage.Put(datasetId, playerId, itemKey, body); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	c.Status(http.StatusNoContent)
}

// Delete handles DELETE /storage/1/data/:datasetId/:key
func (h *StorageHandler) Delete(c *gin.Context) {
	playerId := c.GetString("playerId")
	datasetId := c.Param("datasetId")
	itemKey := c.Param("key")

	if err := h.storage.Delete(datasetId, playerId, itemKey); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	c.Status(http.StatusNoContent)
}
