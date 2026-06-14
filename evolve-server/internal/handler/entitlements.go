package handler

import (
	"net/http"
	"time"

	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

type EntitlementsHandler struct{}

func NewEntitlementsHandler() *EntitlementsHandler { return &EntitlementsHandler{} }

func buildItems(playerId string) []model.EntitlementItem {
	now := int(time.Now().Unix())
	items := make([]model.EntitlementItem, len(model.EntitlementIds))
	for i, id := range model.EntitlementIds {
		items[i] = model.EntitlementItem{
			CreatedOn:             now,
			EntitlementDefId:      id,
			IsServerAuthoritative: true,
			IsValid:               true,
			RuleData:              model.RuleData{Grant: true},
			EntitlementId:         uuid.New().String(),
			AppGroupId:            model.AppGroupId,
			PlayerPublicId:        playerId,
			IsAvailable:           true,
			IsShared:              false,
		}
	}
	return items
}

// GetFirstPartyMapping handles GET /entitlements/1/firstPartyMapping/:platform/:platformId
func (h *EntitlementsHandler) GetFirstPartyMapping(c *gin.Context) {
	playerId := c.GetString("playerId")
	c.JSON(http.StatusOK, gin.H{"entitlements": buildItems(playerId)})
}

// GetMapping handles GET /entitlements/1/mapping/:appGroupId
func (h *EntitlementsHandler) GetMapping(c *gin.Context) {
	playerId := c.GetString("playerId")
	c.JSON(http.StatusOK, gin.H{"entitlements": buildItems(playerId)})
}

// CheckAppOwnership handles GET /entitlements/1/appOwnership/:appGroupId
func (h *EntitlementsHandler) CheckAppOwnership(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"ownsApp": true})
}
