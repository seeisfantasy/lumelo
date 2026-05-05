package settings

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
)

type Config struct {
	Mode          string
	InterfaceMode string
	DSDPolicy     string
	SSHEnabled    bool
	ConfigPath    string
	Warning       string
}

type Store struct {
	mu  sync.RWMutex
	cfg Config
}

type Update struct {
	Mode          string
	InterfaceMode string
	DSDPolicy     string
	SSHEnabled    bool
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

func NewStore(cfg Config) *Store {
	return &Store{cfg: Normalize(cfg)}
}

func (s *Store) Current() Config {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.cfg
}

func (s *Store) Commit(cfg Config) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.cfg = Normalize(cfg)
}

func Normalize(cfg Config) Config {
	defaults := Default()
	if cfg.Mode == "" {
		cfg.Mode = defaults.Mode
	}
	if cfg.InterfaceMode == "" {
		cfg.InterfaceMode = defaults.InterfaceMode
	}
	if cfg.DSDPolicy == "" {
		cfg.DSDPolicy = defaults.DSDPolicy
	}
	if cfg.ConfigPath == "" {
		cfg.ConfigPath = defaults.ConfigPath
	}
	return cfg
}

func ApplyUpdate(current Config, update Update) Config {
	next := current
	next.Mode = strings.TrimSpace(update.Mode)
	next.InterfaceMode = strings.TrimSpace(update.InterfaceMode)
	next.DSDPolicy = strings.TrimSpace(update.DSDPolicy)
	next.SSHEnabled = update.SSHEnabled
	return Normalize(next)
}

func RequiresReboot(current, next Config) bool {
	current = Normalize(current)
	next = Normalize(next)
	return current.Mode != next.Mode ||
		current.InterfaceMode != next.InterfaceMode ||
		current.DSDPolicy != next.DSDPolicy
}

func Validate(cfg Config) error {
	cfg = Normalize(cfg)
	switch cfg.Mode {
	case "local", "bridge":
	default:
		return fmt.Errorf("mode must be local or bridge")
	}
	switch cfg.InterfaceMode {
	case "ethernet", "wifi":
	default:
		return fmt.Errorf("interface_mode must be ethernet or wifi")
	}
	switch cfg.DSDPolicy {
	case "native_dsd", "dop":
	default:
		return fmt.Errorf("dsd_output_policy must be native_dsd or dop")
	}
	return nil
}

func SaveAtomic(path string, cfg Config) error {
	cfg = Normalize(cfg)
	if err := Validate(cfg); err != nil {
		return err
	}
	body := fmt.Sprintf(
		"mode = %q\ninterface_mode = %q\ndsd_output_policy = %q\nssh_enabled = %t\n",
		cfg.Mode,
		cfg.InterfaceMode,
		cfg.DSDPolicy,
		cfg.SSHEnabled,
	)
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return fmt.Errorf("create config dir %s: %w", dir, err)
	}
	temp, err := os.CreateTemp(dir, ".config.toml.*")
	if err != nil {
		return fmt.Errorf("create temp config in %s: %w", dir, err)
	}
	tempPath := temp.Name()
	defer func() {
		_ = os.Remove(tempPath)
	}()

	if _, err := temp.WriteString(body); err != nil {
		_ = temp.Close()
		return fmt.Errorf("write temp config %s: %w", tempPath, err)
	}
	if err := temp.Sync(); err != nil {
		_ = temp.Close()
		return fmt.Errorf("fsync temp config %s: %w", tempPath, err)
	}
	if err := temp.Close(); err != nil {
		return fmt.Errorf("close temp config %s: %w", tempPath, err)
	}
	if err := os.Chmod(tempPath, 0o644); err != nil {
		return fmt.Errorf("chmod temp config %s: %w", tempPath, err)
	}
	if err := os.Rename(tempPath, path); err != nil {
		return fmt.Errorf("rename temp config %s to %s: %w", tempPath, path, err)
	}
	dirHandle, err := os.Open(dir)
	if err != nil {
		return fmt.Errorf("open config dir %s for fsync: %w", dir, err)
	}
	defer dirHandle.Close()
	if err := dirHandle.Sync(); err != nil {
		return fmt.Errorf("fsync config dir %s: %w", dir, err)
	}
	return nil
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
