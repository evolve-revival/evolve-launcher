package handler

import (
	"net/http"

	"github.com/evolve-revival/evolve-server/internal/model"
	"github.com/gin-gonic/gin"
)

type DoormanHandler struct {
	serverHost string
}

func NewDoormanHandler(serverHost string) *DoormanHandler {
	return &DoormanHandler{serverHost: serverHost}
}

// ConfigsGenerate handles GET /doorman/1/configs/generate
// Returns the service directory so the game client knows where to reach each service.
func (h *DoormanHandler) ConfigsGenerate(c *gin.Context) {
	inst := func(actions ...string) model.ServiceInstance {
		return model.ServiceInstance{
			Protocol:     "http",
			Host:         h.serverHost,
			Port:         0,
			BaseUri:      "/",
			Actions:      actions,
			IsProduction: true,
		}
	}

	services := []model.Service{
		{ServiceName: "Doorman", ServiceInstances: []model.ServiceInstance{inst("doorman")}},
		{ServiceName: "Sso", ServiceInstances: []model.ServiceInstance{inst("sso")}},
		{ServiceName: "Entitlements", ServiceInstances: []model.ServiceInstance{inst("entitlements")}},
		{ServiceName: "Storage", ServiceInstances: []model.ServiceInstance{inst("storage")}},
		{ServiceName: "Peers", ServiceInstances: []model.ServiceInstance{inst("peers")}},
		{ServiceName: "Telemetry", ServiceInstances: []model.ServiceInstance{inst("telemetry")}},
		{ServiceName: "Stats", ServiceInstances: []model.ServiceInstance{inst("stats")}},
		{ServiceName: "Grants", ServiceInstances: []model.ServiceInstance{inst("grants")}},
	}

	c.JSON(http.StatusOK, model.DoormanResponse{
		Services: services,
		ClientConfigSettings: model.ClientConfig{
			DoormanConnectTimeout: 5000,
			DoormanRequestTimeout: 10000,
			DefaultRequestTimeout: 10000,
		},
	})
}
