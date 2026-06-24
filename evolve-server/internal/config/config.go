package config

import "os"

type Config struct {
	Port       string
	DBDSN      string
	ServerHost string
	RelayPort  string
	CertFile   string
	KeyFile    string
}

func Load() Config {
	return Config{
		Port:       getenv("PORT", "443"),
		DBDSN:      getenv("DATABASE_URL", "postgres://evolve:evolve@localhost/evolve?sslmode=disable"),
		ServerHost: getenv("SERVER_HOST", "localhost:443"),
		RelayPort:  getenv("RELAY_PORT", "47584"),
		CertFile:   getenv("CERT_FILE", "certs/server.crt"),
		KeyFile:    getenv("KEY_FILE", "certs/server.key"),
	}
}

func getenv(key, fallback string) string {
	if v, ok := os.LookupEnv(key); ok {
		return v
	}
	return fallback
}
