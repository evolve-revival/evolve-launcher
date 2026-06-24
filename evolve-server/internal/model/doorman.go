package model

type DoormanResponse struct {
	AccessToken           string       `json:"accessToken"`
	ExpiresIn             int          `json:"expiresIn"`
	RefreshToken          string       `json:"refreshToken"`
	RefreshExpiresIn      int          `json:"refreshExpiresIn"`
	TokenType             string       `json:"tokenType"`
	PlayerId              interface{}  `json:"playerId"`
	SessionId             string       `json:"sessionId"`
	Services              []Service    `json:"services"`
	ClientConfigSettings  ClientConfig `json:"clientConfigSettings"`
	PlatformType          int          `json:"platformType"`
	OnlineServicePlatform int          `json:"onlineServicePlatform"`
}

type Service struct {
	ServiceName      string            `json:"serviceName"`
	ServiceInstances []ServiceInstance `json:"serviceInstances"`
}

type ServiceInstance struct {
	Protocol     string   `json:"protocol"`
	Host         string   `json:"host"`
	Port         int      `json:"port"`
	BaseUri      string   `json:"baseUri"`
	Actions      []string `json:"actions"`
	IsProduction bool     `json:"isProduction"`
}

type ClientConfig struct {
	DoormanConnectTimeout    int  `json:"doormanConnectTimeout"`
	DoormanRequestTimeout    int  `json:"doormanRequestTimeout"`
	LogLevelMin              int  `json:"logLevelMin"`
	LogLevelMax              int  `json:"logLevelMax"`
	LogLevel                 int  `json:"logLevel"`
	DebugModeMin             bool `json:"debugModeMin"`
	DebugModeMax             bool `json:"debugModeMax"`
	DebugMode                bool `json:"debugMode"`
	RestLogToFileModeMin     bool `json:"restLogToFileModeMin"`
	RestLogToFileModeMax     bool `json:"restLogToFileModeMax"`
	RestLogToFileMode        bool `json:"restLogToFileMode"`
	AutoCacheLogin           bool `json:"autoCacheLogin"`
	MinRequestTimeout        int  `json:"minRequestTimeout"`
	DefaultRequestTimeout    int  `json:"defaultRequestTimeout"`
	AssertTelemetry          bool `json:"assertTelemetry"`
	AppHasStorefront         bool `json:"appHasStorefront"`
	StorefrontHasConsumables bool `json:"storefrontHasConsumables"`
}
