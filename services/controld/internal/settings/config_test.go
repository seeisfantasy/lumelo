package settings

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadParsesKnownKeys(t *testing.T) {
	tempDir := t.TempDir()
	configPath := filepath.Join(tempDir, "config.toml")
	configBody := `mode = "bridge"
interface_mode = "wifi"
dsd_output_policy = "dop"
ssh_enabled = true
ui_theme = "system"
`
	if err := os.WriteFile(configPath, []byte(configBody), 0o644); err != nil {
		t.Fatalf("write config: %v", err)
	}

	cfg, err := Load(configPath)
	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	if cfg.Mode != "bridge" {
		t.Fatalf("Mode = %q, want bridge", cfg.Mode)
	}
	if cfg.InterfaceMode != "wifi" {
		t.Fatalf("InterfaceMode = %q, want wifi", cfg.InterfaceMode)
	}
	if cfg.DSDPolicy != "dop" {
		t.Fatalf("DSDPolicy = %q, want dop", cfg.DSDPolicy)
	}
	if !cfg.SSHEnabled {
		t.Fatalf("SSHEnabled = false, want true")
	}
	if cfg.ConfigPath != configPath {
		t.Fatalf("ConfigPath = %q, want %q", cfg.ConfigPath, configPath)
	}
}

func TestLoadRejectsInvalidBool(t *testing.T) {
	tempDir := t.TempDir()
	configPath := filepath.Join(tempDir, "config.toml")
	if err := os.WriteFile(configPath, []byte("ssh_enabled = maybe\n"), 0o644); err != nil {
		t.Fatalf("write config: %v", err)
	}

	if _, err := Load(configPath); err == nil {
		t.Fatalf("Load() error = nil, want parse error")
	}
}
