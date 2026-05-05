package mediaimport

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

const (
	defaultCommandPath  = "lumelo-media-import"
	defaultMountBase    = "/media"
	listTimeout         = 6 * time.Second
	shortCommandTimeout = 30 * time.Second
	scanCommandTimeout  = 15 * time.Minute
)

type Client struct {
	CommandPath   string
	LibraryDBPath string
	MountBase     string
}

type Snapshot struct {
	Available bool     `json:"available"`
	Devices   []Device `json:"devices,omitempty"`
	Error     string   `json:"error,omitempty"`
}

type Device struct {
	Name         string `json:"name"`
	Path         string `json:"path"`
	Type         string `json:"type"`
	Removable    bool   `json:"removable"`
	Transport    string `json:"transport,omitempty"`
	Label        string `json:"label,omitempty"`
	UUID         string `json:"uuid,omitempty"`
	PartUUID     string `json:"partuuid,omitempty"`
	FSType       string `json:"fstype,omitempty"`
	Mountpoint   string `json:"mountpoint,omitempty"`
	IsMounted    bool   `json:"is_mounted"`
	VolumeUUID   string `json:"volume_uuid,omitempty"`
	SourceClass  string `json:"source_class,omitempty"`
	SourceReason string `json:"source_reason,omitempty"`
}

type CommandRequest struct {
	Action     string
	DevicePath string
	ScanPath   string
}

type CommandResult struct {
	Action  string `json:"action"`
	Message string `json:"message,omitempty"`
	Output  string `json:"output,omitempty"`
}

func New(commandPath, libraryDBPath string) *Client {
	if strings.TrimSpace(commandPath) == "" {
		commandPath = defaultCommandPath
	}
	return &Client{
		CommandPath:   commandPath,
		LibraryDBPath: libraryDBPath,
		MountBase:     defaultMountBase,
	}
}

func (c *Client) Snapshot(ctx context.Context) Snapshot {
	output, err := c.runStdout(ctx, listTimeout, "list-devices")
	if err != nil {
		return Snapshot{Error: err.Error()}
	}

	var devices []Device
	if err := json.Unmarshal([]byte(output), &devices); err != nil {
		return Snapshot{Error: fmt.Sprintf("decode list-devices: %v", err)}
	}

	return Snapshot{Available: true, Devices: devices}
}

func (c *Client) Execute(ctx context.Context, request CommandRequest) (CommandResult, error) {
	action := strings.ToLower(strings.TrimSpace(request.Action))
	result := CommandResult{Action: action}

	var (
		output string
		err    error
	)
	switch action {
	case "refresh":
		result.Message = "media devices refreshed"
		return result, nil
	case "mount_device":
		devicePath, validateErr := validateDevicePath(request.DevicePath)
		if validateErr != nil {
			return result, validateErr
		}
		output, err = c.runCombined(ctx, shortCommandTimeout, "import-device", devicePath, "--mount-only", "--db", c.LibraryDBPath, "--mount-base", c.mountBase())
		result.Message = "media device mounted"
	case "scan_device":
		devicePath, validateErr := validateDevicePath(request.DevicePath)
		if validateErr != nil {
			return result, validateErr
		}
		output, err = c.runCombined(ctx, scanCommandTimeout, "import-device", devicePath, "--db", c.LibraryDBPath, "--mount-base", c.mountBase())
		result.Message = "media device scanned"
	case "scan_mounted":
		output, err = c.runCombined(ctx, scanCommandTimeout, "scan-mounted", "--db", c.LibraryDBPath)
		result.Message = "mounted media scanned"
	case "scan_path":
		scanPath, validateErr := validateScanPath(request.ScanPath)
		if validateErr != nil {
			return result, validateErr
		}
		output, err = c.runCombined(ctx, scanCommandTimeout, "scan-path", scanPath, "--db", c.LibraryDBPath)
		result.Message = "media path scanned"
	case "reconcile_volumes":
		output, err = c.runCombined(ctx, shortCommandTimeout, "reconcile-volumes", "--db", c.LibraryDBPath)
		result.Message = "media volumes reconciled"
	default:
		return result, fmt.Errorf("unsupported media command: %s", fallback(action, "(empty)"))
	}

	result.Output = strings.TrimSpace(output)
	if err != nil {
		return result, err
	}
	return result, nil
}

func (c *Client) runStdout(ctx context.Context, timeout time.Duration, args ...string) (string, error) {
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	cmd := exec.CommandContext(ctx, c.commandPath(), args...)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		if ctx.Err() != nil {
			return "", fmt.Errorf("%s %s: %w", c.commandPath(), strings.Join(args, " "), ctx.Err())
		}
		return "", fmt.Errorf("%s %s: %w: %s", c.commandPath(), strings.Join(args, " "), err, strings.TrimSpace(stderr.String()))
	}
	return stdout.String(), nil
}

func (c *Client) runCombined(ctx context.Context, timeout time.Duration, args ...string) (string, error) {
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	cmd := exec.CommandContext(ctx, c.commandPath(), args...)
	output, err := cmd.CombinedOutput()
	text := string(output)
	if err != nil {
		if ctx.Err() != nil {
			return text, fmt.Errorf("%s %s: %w", c.commandPath(), strings.Join(args, " "), ctx.Err())
		}
		return text, fmt.Errorf("%s %s: %w: %s", c.commandPath(), strings.Join(args, " "), err, strings.TrimSpace(text))
	}
	return text, nil
}

func (c *Client) commandPath() string {
	if strings.TrimSpace(c.CommandPath) == "" {
		return defaultCommandPath
	}
	return c.CommandPath
}

func (c *Client) mountBase() string {
	if strings.TrimSpace(c.MountBase) == "" {
		return defaultMountBase
	}
	return c.MountBase
}

func validateDevicePath(raw string) (string, error) {
	path := filepath.Clean(strings.TrimSpace(raw))
	if path == "." || path == "/" || !strings.HasPrefix(path, "/dev/") {
		return "", fmt.Errorf("invalid media device path")
	}
	if strings.ContainsAny(path, "\x00\n\r\t") {
		return "", fmt.Errorf("invalid media device path")
	}
	return path, nil
}

func validateScanPath(raw string) (string, error) {
	path := filepath.Clean(strings.TrimSpace(raw))
	if path == "." || path == "/" || strings.ContainsAny(path, "\x00\n\r\t") {
		return "", fmt.Errorf("invalid media scan path")
	}
	if path == "/media" || path == "/mnt" || strings.HasPrefix(path, "/media/") || strings.HasPrefix(path, "/mnt/") {
		return path, nil
	}
	return "", fmt.Errorf("media scan path must be under /media or /mnt")
}

func fallback(value string, fallback string) string {
	if strings.TrimSpace(value) == "" {
		return fallback
	}
	return value
}
