package model

type SSOResponse struct {
	IsNewPlayer      bool   `json:"isNewPlayer"`
	HasPlayedApp     bool   `json:"hasPlayedApp"`
	AccessToken      string `json:"accessToken"`
	ExpiresIn        int    `json:"expiresIn"`
	TokenType        string `json:"tokenType"`
	PlayerId         string `json:"playerId"`
	SessionId        string `json:"sessionId"`
	RefreshToken     string `json:"refreshToken"`
	RefreshExpiresIn int    `json:"refreshExpiresIn"`
	DobNeeded        bool   `json:"dobNeeded"`
}
