package middleware

import (
	"strings"
	"sync"

	"github.com/gin-gonic/gin"
)

var tokenStore sync.Map // token → playerId

// StoreToken records an accessToken → playerId mapping after successful SSO logon.
func StoreToken(token, playerId string) {
	tokenStore.Store(token, playerId)
}

// Auth reads Authorization: Bearer <token>, looks up playerId, and injects it
// into the gin context. Requests without a valid token continue with empty playerId
// (doorman and SSO work unauthenticated).
func Auth() gin.HandlerFunc {
	return func(c *gin.Context) {
		if auth := c.GetHeader("Authorization"); strings.HasPrefix(auth, "Bearer ") {
			token := strings.TrimPrefix(auth, "Bearer ")
			if val, ok := tokenStore.Load(token); ok {
				c.Set("playerId", val.(string))
			}
		}
		c.Next()
	}
}
