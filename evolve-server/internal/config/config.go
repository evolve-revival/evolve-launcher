package config

import "os"

type Config struct {
	Port       string
	DBDSN      string
	ServerHost string
	RelayPort  string
}

func Load() Config {
	return Config{
		Port:       getenv("PORT", "8080"),
		DBDSN:      getenv("DATABASE_URL", "postgres://evolve:evolve@localhost/evolve?sslmode=disable"),
		ServerHost: getenv("SERVER_HOST", "localhost:8080"),
		RelayPort:  getenv("RELAY_PORT", "47584"),
	}
}

func getenv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
