package api_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"strings"
	"testing"

	"github.com/lumelo/controld/internal/api"
	"github.com/lumelo/controld/internal/auth"
	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/playbackclient"
	"github.com/lumelo/controld/internal/provisioningclient"
	"github.com/lumelo/controld/internal/settings"
	"github.com/lumelo/controld/internal/sshctl"
	"github.com/lumelo/controld/web"
)

func TestHealthzReportsControlPlaneAndDependencyState(t *testing.T) {
	tempDir := t.TempDir()
	server, err := api.New(api.Dependencies{
		Auth: auth.NewService(false),
		Playback: playbackclient.New(
			filepath.Join(tempDir, "missing-playback-cmd.sock"),
			filepath.Join(tempDir, "missing-playback-evt.sock"),
		),
		Library:      libraryclient.New(filepath.Join(tempDir, "missing-library.db")),
		Logs:         &fakeLogSource{text: "boot ok\ncontrold online\n"},
		Provisioning: &fakeProvisioningSource{snapshot: provisioningclient.Snapshot{Available: true, State: "waiting_for_ip", Message: "credentials applied; waiting for DHCP"}},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})
	if err != nil {
		t.Fatalf("build server: %v", err)
	}

	request := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if contentType := response.Header().Get("Content-Type"); contentType != "application/json; charset=utf-8" {
		t.Fatalf("unexpected content type: %s", contentType)
	}

	var payload struct {
		Status                string `json:"status"`
		Mode                  string `json:"mode"`
		InterfaceMode         string `json:"interface_mode"`
		SSHEnabled            bool   `json:"ssh_enabled"`
		PlaybackAvailable     bool   `json:"playback_available"`
		PlaybackError         string `json:"playback_error"`
		LibraryAvailable      bool   `json:"library_available"`
		LibraryDBPath         string `json:"library_db_path"`
		LibraryError          string `json:"library_error"`
		ProvisioningAvailable bool   `json:"provisioning_available"`
		ProvisioningState     string `json:"provisioning_state"`
		ProvisioningMessage   string `json:"provisioning_message"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode health response: %v", err)
	}

	if payload.Status != "ok" {
		t.Fatalf("unexpected status payload: %+v", payload)
	}
	if payload.Mode != "local" || payload.InterfaceMode != "ethernet" {
		t.Fatalf("unexpected mode payload: %+v", payload)
	}
	if payload.SSHEnabled {
		t.Fatalf("expected SSH to be disabled: %+v", payload)
	}
	if payload.PlaybackAvailable || payload.PlaybackError == "" {
		t.Fatalf("expected playback dependency error: %+v", payload)
	}
	if payload.LibraryAvailable || payload.LibraryError == "" || payload.LibraryDBPath == "" {
		t.Fatalf("expected library dependency error: %+v", payload)
	}
	if !payload.ProvisioningAvailable || payload.ProvisioningState != "waiting_for_ip" || payload.ProvisioningMessage == "" {
		t.Fatalf("expected provisioning payload: %+v", payload)
	}
}

func TestLogsPageRendersRecentJournal(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\ncontrold online\n"}, nil)

	request := httptest.NewRequest(http.MethodGet, "/logs?lines=100", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	if !strings.Contains(body, "System logs") {
		t.Fatalf("expected logs page heading, body: %s", body)
	}
	if !strings.Contains(body, "controld online") {
		t.Fatalf("expected log output, body: %s", body)
	}
}

func TestLogsTextReturnsCopyFriendlyJournal(t *testing.T) {
	source := &fakeLogSource{text: "boot ok\nplaybackd ready\n"}
	server := newTestServer(t, source, nil)

	request := httptest.NewRequest(http.MethodGet, "/logs.txt?lines=10", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if contentType := response.Header().Get("Content-Type"); contentType != "text/plain; charset=utf-8" {
		t.Fatalf("unexpected content type: %s", contentType)
	}
	if response.Body.String() != "boot ok\nplaybackd ready\n" {
		t.Fatalf("unexpected logs text: %q", response.Body.String())
	}
	if source.lastLines != 50 {
		t.Fatalf("expected line count to clamp to 50, got %d", source.lastLines)
	}
}

func TestHomePageRendersProvisioningSummary(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available:      true,
			State:          "failed",
			Message:        "timed out waiting for DHCP on wlan0",
			SSID:           "Studio WiFi",
			WiFiInterface:  "wlan0",
			ErrorCode:      "dhcp_timeout",
			ApplyOutput:    "Wi-Fi credentials written for SSID: Studio WiFi on interface: wlan0",
			DiagnosticHint: "Check wpa_supplicant@wlan0.service, networkctl status wlan0, and /run/lumelo/provisioning-status.json",
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	if !strings.Contains(body, "Provisioning Summary") || !strings.Contains(body, "dhcp_timeout") || !strings.Contains(body, "Studio WiFi") {
		t.Fatalf("expected provisioning summary on home page, body: %s", body)
	}
}

func TestProvisioningStatusReturnsLatestSnapshot(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available:     true,
			State:         "connected",
			Message:       "wifi connected",
			SSID:          "Home WiFi",
			IP:            "192.168.1.44",
			WebURL:        "http://192.168.1.44:18080/",
			WiFiInterface: "wlan0",
			WPAUnit:       "wpa_supplicant@wlan0.service",
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/provisioning-status", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if contentType := response.Header().Get("Content-Type"); contentType != "application/json; charset=utf-8" {
		t.Fatalf("unexpected content type: %s", contentType)
	}

	var payload provisioningclient.Snapshot
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode provisioning response: %v", err)
	}
	if !payload.Available || payload.State != "connected" || payload.IP != "192.168.1.44" {
		t.Fatalf("unexpected provisioning payload: %+v", payload)
	}
}

func TestProvisioningPageRendersSnapshotDetails(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available: true,
			State:     "waiting_for_ip",
			Message:   "credentials applied; waiting for DHCP",
			SSID:      "Studio WiFi",
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/provisioning", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	if !strings.Contains(body, "Provisioning status") || !strings.Contains(body, "Studio WiFi") {
		t.Fatalf("expected provisioning page content, body: %s", body)
	}
}

func newTestServer(t *testing.T, logs api.LogSource, provisioning api.ProvisioningSource) *api.Server {
	t.Helper()

	tempDir := t.TempDir()
	if provisioning == nil {
		provisioning = &fakeProvisioningSource{}
	}
	server, err := api.New(api.Dependencies{
		Auth: auth.NewService(false),
		Playback: playbackclient.New(
			filepath.Join(tempDir, "missing-playback-cmd.sock"),
			filepath.Join(tempDir, "missing-playback-evt.sock"),
		),
		Library:      libraryclient.New(filepath.Join(tempDir, "missing-library.db")),
		Logs:         logs,
		Provisioning: provisioning,
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})
	if err != nil {
		t.Fatalf("build server: %v", err)
	}

	return server
}

type fakeLogSource struct {
	text      string
	err       error
	lastLines int
}

func (f *fakeLogSource) Recent(_ context.Context, lines int) (string, error) {
	f.lastLines = lines
	return f.text, f.err
}

type fakeProvisioningSource struct {
	snapshot provisioningclient.Snapshot
}

func (f *fakeProvisioningSource) Snapshot(context.Context) provisioningclient.Snapshot {
	return f.snapshot
}
