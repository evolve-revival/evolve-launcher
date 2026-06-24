package config_test

import (
	"os"
	"testing"

	"github.com/evolve-revival/evolve-server/internal/config"
)

func TestLoad_defaults(t *testing.T) {
	os.Unsetenv("PORT")
	os.Unsetenv("DATABASE_URL")
	os.Unsetenv("SERVER_HOST")
	cfg := config.Load()
	if cfg.Port != "443" {
		t.Errorf("Port = %q, want 443", cfg.Port)
	}
	if cfg.ServerHost == "" {
		t.Error("ServerHost must not be empty")
	}
}

func TestLoad_env(t *testing.T) {
	os.Setenv("PORT", "9090")
	defer os.Unsetenv("PORT")
	cfg := config.Load()
	if cfg.Port != "9090" {
		t.Errorf("Port = %q, want 9090", cfg.Port)
	}
}

func TestLoadCertDefaults(t *testing.T) {
	os.Unsetenv("CERT_FILE")
	os.Unsetenv("KEY_FILE")
	os.Unsetenv("PORT")
	cfg := config.Load()
	if cfg.CertFile != "certs/server.crt" {
		t.Errorf("CertFile default: want certs/server.crt, got %q", cfg.CertFile)
	}
	if cfg.KeyFile != "certs/server.key" {
		t.Errorf("KeyFile default: want certs/server.key, got %q", cfg.KeyFile)
	}
	if cfg.Port != "443" {
		t.Errorf("Port default: want 443, got %q", cfg.Port)
	}
}

func TestLoadCertOverride(t *testing.T) {
	t.Setenv("CERT_FILE", "/tmp/test.crt")
	t.Setenv("KEY_FILE", "/tmp/test.key")
	cfg := config.Load()
	if cfg.CertFile != "/tmp/test.crt" {
		t.Errorf("CertFile override: want /tmp/test.crt, got %q", cfg.CertFile)
	}
	if cfg.KeyFile != "/tmp/test.key" {
		t.Errorf("KeyFile override: want /tmp/test.key, got %q", cfg.KeyFile)
	}
}
