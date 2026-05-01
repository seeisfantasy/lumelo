package settings

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type Config struct {
	Mode          string
	InterfaceMode string
	DSDPolicy     string
	SSHEnabled    bool
	ConfigPath    string
}

func Default() Config {
	return Config{
		Mode:          "local",
		InterfaceMode: "ethernet",
		DSDPolicy:     "native_dsd",
		SSHEnabled:    false,
		ConfigPath:    "/etc/lumelo/config.toml",
	}
}

func Load(path string) (Config, error) {
	cfg := Default()
	cfg.ConfigPath = path

	file, err := os.Open(path)
	if err != nil {
		return cfg, err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	lineNo := 0
	for scanner.Scan() {
		lineNo++
		line := strings.TrimSpace(scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		key, value, ok := strings.Cut(line, "=")
		if !ok {
			return cfg, fmt.Errorf("%s:%d: expected key = value", path, lineNo)
		}

		key = strings.TrimSpace(key)
		value = strings.TrimSpace(value)

		switch key {
		case "mode":
			cfg.Mode, err = parseStringValue(value)
		case "interface_mode":
			cfg.InterfaceMode, err = parseStringValue(value)
		case "dsd_output_policy":
			cfg.DSDPolicy, err = parseStringValue(value)
		case "ssh_enabled":
			cfg.SSHEnabled, err = strconv.ParseBool(value)
		default:
			continue
		}
		if err != nil {
			return cfg, fmt.Errorf("%s:%d: parse %s: %w", path, lineNo, key, err)
		}
	}

	if err := scanner.Err(); err != nil {
		return cfg, fmt.Errorf("scan %s: %w", path, err)
	}

	return cfg, nil
}

func parseStringValue(value string) (string, error) {
	value = strings.TrimSpace(value)
	if len(value) < 2 || value[0] != '"' || value[len(value)-1] != '"' {
		return "", fmt.Errorf("expected quoted string")
	}

	unquoted, err := strconv.Unquote(value)
	if err != nil {
		return "", err
	}
	return unquoted, nil
}
