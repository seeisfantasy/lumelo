package api_test

import (
	"context"
	"database/sql"
	"encoding/json"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/lumelo/controld/internal/api"
	"github.com/lumelo/controld/internal/audiodevice"
	"github.com/lumelo/controld/internal/auth"
	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/mediaimport"
	"github.com/lumelo/controld/internal/playbackclient"
	"github.com/lumelo/controld/internal/provisioningclient"
	"github.com/lumelo/controld/internal/settings"
	"github.com/lumelo/controld/internal/sshctl"
	"github.com/lumelo/controld/web"
	_ "modernc.org/sqlite"
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
		ProvisioningErrorCode string `json:"provisioning_error_code"`
		ProvisioningBTAddress string `json:"provisioning_bluetooth_address"`
		ProvisioningRFCOMM    int    `json:"provisioning_rfcomm_channel"`
		ProvisioningSDPCount  int    `json:"provisioning_sdp_record_count"`
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
	if payload.ProvisioningErrorCode != "" || payload.ProvisioningBTAddress != "" || payload.ProvisioningRFCOMM != 0 || payload.ProvisioningSDPCount != 0 {
		t.Fatalf("expected empty provisioning diagnostics for minimal snapshot: %+v", payload)
	}
}

func TestHealthzPrefersConnectedWiFiInterfaceModeFromProvisioningSnapshot(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available:     true,
			State:         "connected",
			IP:            "192.168.1.44",
			WiFiIP:        "192.168.1.44",
			WiFiInterface: "wlan0",
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		InterfaceMode string `json:"interface_mode"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode health response: %v", err)
	}

	if payload.InterfaceMode != "wifi" {
		t.Fatalf("InterfaceMode = %q, want wifi", payload.InterfaceMode)
	}
}

func TestHealthzPrefersConnectedWiredInterfaceModeFromProvisioningSnapshot(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available: true,
			State:     "connected",
			IP:        "192.168.1.120",
			WiredIP:   "192.168.1.120",
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		InterfaceMode string `json:"interface_mode"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode health response: %v", err)
	}

	if payload.InterfaceMode != "ethernet" {
		t.Fatalf("InterfaceMode = %q, want ethernet", payload.InterfaceMode)
	}
}

func TestHealthzIncludesProvisioningDiagnostics(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available:        true,
			State:            "failed",
			Message:          "classic bluetooth socket bind failed: bind boom",
			ErrorCode:        "classic_bluetooth_socket_bind_failed",
			BluetoothAddress: "C0:84:7D:1F:37:C7",
			RFCOMMChannel:    1,
			SDPRecordHandles: []string{"0x10008"},
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		ProvisioningState     string `json:"provisioning_state"`
		ProvisioningErrorCode string `json:"provisioning_error_code"`
		ProvisioningBTAddress string `json:"provisioning_bluetooth_address"`
		ProvisioningRFCOMM    int    `json:"provisioning_rfcomm_channel"`
		ProvisioningSDPCount  int    `json:"provisioning_sdp_record_count"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode health response: %v", err)
	}

	if payload.ProvisioningState != "failed" ||
		payload.ProvisioningErrorCode != "classic_bluetooth_socket_bind_failed" ||
		payload.ProvisioningBTAddress != "C0:84:7D:1F:37:C7" ||
		payload.ProvisioningRFCOMM != 1 ||
		payload.ProvisioningSDPCount != 1 {
		t.Fatalf("unexpected provisioning diagnostics: %+v", payload)
	}
}

func TestAPISystemSummaryReturnsStableConfigAndLiveInterfaceMode(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available:     true,
			State:         "connected",
			IP:            "192.168.1.44",
			WiFiIP:        "192.168.1.44",
			WiFiInterface: "wlan0",
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/api/v1/system/summary", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if contentType := response.Header().Get("Content-Type"); contentType != "application/json; charset=utf-8" {
		t.Fatalf("unexpected content type: %s", contentType)
	}

	var payload struct {
		Mode               string `json:"mode"`
		InterfaceMode      string `json:"interface_mode"`
		DSDPolicy          string `json:"dsd_policy"`
		PasswordConfigured bool   `json:"password_configured"`
		SSHEnabled         bool   `json:"ssh_enabled"`
		CommandSocket      string `json:"command_socket"`
		EventSocket        string `json:"event_socket"`
		LibraryDBPath      string `json:"library_db_path"`
		ConfigPath         string `json:"config_path"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode summary response: %v", err)
	}

	if payload.Mode != "local" || payload.InterfaceMode != "wifi" || payload.DSDPolicy != "native_dsd" {
		t.Fatalf("unexpected summary payload: %+v", payload)
	}
	if payload.PasswordConfigured || payload.SSHEnabled {
		t.Fatalf("expected disabled auth/ssh defaults: %+v", payload)
	}
	if payload.CommandSocket == "" || payload.EventSocket == "" || payload.LibraryDBPath == "" || payload.ConfigPath == "" {
		t.Fatalf("expected diagnostics paths in payload: %+v", payload)
	}
}

func TestAPIPlaybackStatusReturnsStructuredStatus(t *testing.T) {
	tempDir := t.TempDir()
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		buf := make([]byte, 128)
		_, _ = conn.Read(buf)
		_, _ = conn.Write([]byte("OK\tkind=status\tstate=quiet_active\torder_mode=sequential\trepeat_mode=off\tcurrent_track=demo-track\tlast_command=play:demo-track\tqueue_entries=2\n"))
	}()

	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(tempDir, "unused-evt.sock")),
		Library:      libraryclient.New(filepath.Join(tempDir, "missing-library.db")),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/api/v1/playback/status", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		Available    bool   `json:"available"`
		State        string `json:"state"`
		OrderMode    string `json:"order_mode"`
		RepeatMode   string `json:"repeat_mode"`
		CurrentTrack string `json:"current_track"`
		LastCommand  string `json:"last_command"`
		QueueEntries int    `json:"queue_entries"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode playback status response: %v", err)
	}

	if !payload.Available || payload.State != "quiet_active" || payload.CurrentTrack != "demo-track" || payload.QueueEntries != 2 {
		t.Fatalf("unexpected playback status payload: %+v", payload)
	}

	<-done
}

func TestAPIPlaybackQueueReturnsStructuredQueue(t *testing.T) {
	tempDir := t.TempDir()
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		buf := make([]byte, 128)
		_, _ = conn.Read(buf)
		_, _ = conn.Write([]byte("OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":1,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-a\",\"volume_uuid\":\"vol-1\",\"relative_path\":\"Album/track-a.flac\",\"title\":\"Track A\",\"duration_ms\":201000,\"is_current\":false},{\"order_index\":1,\"queue_entry_id\":\"q2\",\"track_uid\":\"track-b\",\"volume_uuid\":\"vol-1\",\"relative_path\":\"Album/track-b.flac\",\"title\":\"Track B\",\"duration_ms\":202000,\"is_current\":true}]}\n"))
	}()

	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(tempDir, "unused-evt.sock")),
		Library:      libraryclient.New(filepath.Join(tempDir, "missing-library.db")),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/api/v1/playback/queue", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		Available         bool `json:"available"`
		CurrentOrderIndex *int `json:"current_order_index"`
		Entries           []struct {
			QueueEntryID string `json:"queue_entry_id"`
			TrackUID     string `json:"track_uid"`
			IsCurrent    bool   `json:"is_current"`
		} `json:"entries"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode playback queue response: %v", err)
	}

	if !payload.Available || payload.CurrentOrderIndex == nil || *payload.CurrentOrderIndex != 1 {
		t.Fatalf("unexpected queue payload: %+v", payload)
	}
	if len(payload.Entries) != 2 || payload.Entries[1].QueueEntryID != "q2" || !payload.Entries[1].IsCurrent {
		t.Fatalf("unexpected queue entries: %+v", payload.Entries)
	}

	<-done
}

func TestAPIPlaybackHistoryReturnsStructuredHistory(t *testing.T) {
	tempDir := t.TempDir()
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		defer conn.Close()

		buf := make([]byte, 128)
		_, _ = conn.Read(buf)
		_, _ = conn.Write([]byte("OK\tkind=history_snapshot\tpayload={\"entries\":[{\"played_at\":123,\"track_uid\":\"track-a\",\"volume_uuid\":\"vol-1\",\"relative_path\":\"Album/track-a.flac\",\"title\":\"Track A\",\"duration_ms\":201000}]}\n"))
	}()

	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(tempDir, "unused-evt.sock")),
		Library:      libraryclient.New(filepath.Join(tempDir, "missing-library.db")),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/api/v1/playback/history", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		Available bool `json:"available"`
		Entries   []struct {
			TrackUID string `json:"track_uid"`
			Title    string `json:"title"`
		} `json:"entries"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode playback history response: %v", err)
	}

	if !payload.Available || len(payload.Entries) != 1 || payload.Entries[0].TrackUID != "track-a" || payload.Entries[0].Title != "Track A" {
		t.Fatalf("unexpected history payload: %+v", payload)
	}

	<-done
}

func TestAPILibrarySnapshotReturnsStructuredLibraryData(t *testing.T) {
	dbPath := writeLibraryFixture(t)
	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(filepath.Join(t.TempDir(), "missing-playback-cmd.sock"), filepath.Join(t.TempDir(), "missing-playback-evt.sock")),
		Library:      libraryclient.New(dbPath),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/api/v1/library/snapshot?album_uid=album-001", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		Available bool   `json:"available"`
		DBPath    string `json:"db_path"`
		Query     struct {
			AlbumUID string `json:"album_uid"`
		} `json:"query"`
		Stats struct {
			AlbumCount int `json:"album_count"`
			TrackCount int `json:"track_count"`
		} `json:"stats"`
		Albums []struct {
			AlbumUID string `json:"album_uid"`
			Title    string `json:"title"`
		} `json:"albums"`
		Tracks []struct {
			TrackUID string `json:"track_uid"`
			AlbumUID string `json:"album_uid"`
			Title    string `json:"title"`
		} `json:"tracks"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode library snapshot response: %v", err)
	}

	if !payload.Available || payload.DBPath == "" || payload.Query.AlbumUID != "album-001" {
		t.Fatalf("unexpected library payload: %+v", payload)
	}
	if payload.Stats.AlbumCount != 1 || payload.Stats.TrackCount != 2 {
		t.Fatalf("unexpected library stats: %+v", payload.Stats)
	}
	if len(payload.Albums) != 1 || payload.Albums[0].AlbumUID != "album-001" {
		t.Fatalf("unexpected albums payload: %+v", payload.Albums)
	}
	if len(payload.Tracks) != 2 || payload.Tracks[0].TrackUID != "track-001" || payload.Tracks[1].TrackUID != "track-002" {
		t.Fatalf("unexpected tracks payload: %+v", payload.Tracks)
	}
}

func TestAPILibraryMediaReturnsDetectedDevices(t *testing.T) {
	mediaSource := &fakeMediaImportSource{
		snapshot: mediaimport.Snapshot{
			Available: true,
			Devices: []mediaimport.Device{{
				Label:      "TF Music",
				Path:       "/dev/sda1",
				FSType:     "exfat",
				Mountpoint: "/media/tf-music",
				IsMounted:  true,
				VolumeUUID: "media-uuid-demo",
			}},
		},
	}
	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(filepath.Join(t.TempDir(), "missing-playback-cmd.sock"), filepath.Join(t.TempDir(), "missing-playback-evt.sock")),
		Library:      libraryclient.New(filepath.Join(t.TempDir(), "missing-library.db")),
		MediaImport:  mediaSource,
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/api/v1/library/media", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload mediaimport.Snapshot
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode library media response: %v", err)
	}
	if !payload.Available || len(payload.Devices) != 1 || payload.Devices[0].Path != "/dev/sda1" || payload.Devices[0].Mountpoint != "/media/tf-music" {
		t.Fatalf("unexpected media payload: %+v", payload)
	}
}

func TestAPILibraryMediaCommandsBlockScansDuringQuietMode(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := servePlaybackSequence(t, listener, []playbackExchange{
		{
			expectedLine: "STATUS\n",
			responseLine: "OK\tkind=status\tstate=quiet_active\torder_mode=sequential\trepeat_mode=off\tcurrent_track=track-001\tlast_command=status\tqueue_entries=1\n",
		},
		{
			expectedLine: "STATUS\n",
			responseLine: "OK\tkind=status\tstate=quiet_active\torder_mode=sequential\trepeat_mode=off\tcurrent_track=track-001\tlast_command=status\tqueue_entries=1\n",
		},
	})
	mediaSource := &fakeMediaImportSource{snapshot: mediaimport.Snapshot{Available: true}}
	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(t.TempDir(), "unused-evt.sock")),
		Library:      libraryclient.New(filepath.Join(t.TempDir(), "missing-library.db")),
		MediaImport:  mediaSource,
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodPost, "/api/v1/library/media/commands", strings.NewReader(`{"action":"scan_mounted"}`))
	request.Header.Set("Content-Type", "application/json")
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusBadRequest {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if mediaSource.executeCount != 0 {
		t.Fatalf("expected scan to be blocked before media import execution")
	}

	var payload struct {
		OK                  bool   `json:"ok"`
		Error               string `json:"error"`
		PlaybackScanBlocked bool   `json:"playback_scan_blocked"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode library media command response: %v", err)
	}
	if payload.OK || !payload.PlaybackScanBlocked || !strings.Contains(payload.Error, "playback_quiet_mode_active") {
		t.Fatalf("unexpected blocked media command payload: %+v", payload)
	}

	<-done
}

func TestAPILibraryMediaCommandsExecuteNonScanAction(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := servePlaybackSequence(t, listener, []playbackExchange{
		{
			expectedLine: "STATUS\n",
			responseLine: "OK\tkind=status\tstate=stopped\torder_mode=sequential\trepeat_mode=off\tcurrent_track=\tlast_command=status\tqueue_entries=0\n",
		},
	})
	mediaSource := &fakeMediaImportSource{
		snapshot: mediaimport.Snapshot{Available: true},
		result: mediaimport.CommandResult{
			Action:  "reconcile_volumes",
			Message: "media volumes reconciled",
			Output:  `{"reconciled":[]}`,
		},
	}
	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(t.TempDir(), "unused-evt.sock")),
		Library:      libraryclient.New(filepath.Join(t.TempDir(), "missing-library.db")),
		MediaImport:  mediaSource,
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodPost, "/api/v1/library/media/commands", strings.NewReader(`{"action":"reconcile_volumes"}`))
	request.Header.Set("Content-Type", "application/json")
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if mediaSource.executeCount != 1 || mediaSource.lastRequest.Action != "reconcile_volumes" {
		t.Fatalf("unexpected media command execution: count=%d request=%+v", mediaSource.executeCount, mediaSource.lastRequest)
	}

	var payload struct {
		OK      bool   `json:"ok"`
		Message string `json:"message"`
		Output  string `json:"output"`
		Error   string `json:"error"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode library media command response: %v", err)
	}
	if !payload.OK || payload.Message != "media volumes reconciled" || payload.Output == "" || payload.Error != "" {
		t.Fatalf("unexpected media command payload: %+v", payload)
	}

	<-done
}

func TestAPIPlaybackCommandsReturnsStructuredResult(t *testing.T) {
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := servePlaybackSequence(t, listener, []playbackExchange{
		{
			expectedLine: "PAUSE\n",
			responseLine: "OK\tkind=ack\taction=pause\tstate=paused\tcurrent_track=track-002\n",
		},
		{
			expectedLine: "STATUS\n",
			responseLine: "OK\tkind=status\tstate=paused\torder_mode=sequential\trepeat_mode=off\tcurrent_track=track-002\tlast_command=pause\tqueue_entries=2\n",
		},
		{
			expectedLine: "QUEUE_SNAPSHOT\n",
			responseLine: "OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":1,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-001\",\"volume_uuid\":\"vol-1\",\"relative_path\":\"Album/track-001.flac\",\"title\":\"Track 1\",\"duration_ms\":201000,\"is_current\":false},{\"order_index\":1,\"queue_entry_id\":\"q2\",\"track_uid\":\"track-002\",\"volume_uuid\":\"vol-1\",\"relative_path\":\"Album/track-002.flac\",\"title\":\"Track 2\",\"duration_ms\":202000,\"is_current\":true}]}\n",
		},
	})

	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(t.TempDir(), "unused-evt.sock")),
		Library:      libraryclient.New(filepath.Join(t.TempDir(), "missing-library.db")),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodPost, "/api/v1/playback/commands", strings.NewReader(`{"action":"pause"}`))
	request.Header.Set("Content-Type", "application/json")
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		OK             bool   `json:"ok"`
		Action         string `json:"action"`
		Message        string `json:"message"`
		Error          string `json:"error"`
		PlaybackStatus struct {
			State        string `json:"state"`
			CurrentTrack string `json:"current_track"`
		} `json:"playback_status"`
		Queue struct {
			CurrentOrderIndex *int `json:"current_order_index"`
			Entries           []struct {
				QueueEntryID string `json:"queue_entry_id"`
			} `json:"entries"`
		} `json:"queue"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode playback command response: %v", err)
	}

	if !payload.OK || payload.Action != "pause" || payload.Message != "" || payload.Error != "" {
		t.Fatalf("unexpected playback command payload: %+v", payload)
	}
	if payload.PlaybackStatus.State != "paused" || payload.PlaybackStatus.CurrentTrack != "track-002" {
		t.Fatalf("unexpected playback status after command: %+v", payload.PlaybackStatus)
	}
	if payload.Queue.CurrentOrderIndex == nil || *payload.Queue.CurrentOrderIndex != 1 || len(payload.Queue.Entries) != 2 {
		t.Fatalf("unexpected queue payload after command: %+v", payload.Queue)
	}

	<-done
}

func TestAPIPlaybackCommandsRejectsAbsolutePathTargets(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, nil)

	for _, body := range []string{
		`{"action":"play","track_id":"/tmp/manual.wav"}`,
		`{"action":"queue_play","track_id":"[\"track-001\",\"/tmp/manual.wav\"]"}`,
	} {
		request := httptest.NewRequest(http.MethodPost, "/api/v1/playback/commands", strings.NewReader(body))
		request.Header.Set("Content-Type", "application/json")
		response := httptest.NewRecorder()

		server.Handler().ServeHTTP(response, request)

		if response.Code != http.StatusBadRequest {
			t.Fatalf("unexpected status for %s: %d", body, response.Code)
		}

		var payload struct {
			OK    bool   `json:"ok"`
			Error string `json:"error"`
		}
		if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
			t.Fatalf("decode playback command response: %v", err)
		}
		if payload.OK || !strings.Contains(payload.Error, "absolute_path_playback_forbidden") {
			t.Fatalf("expected absolute path rejection for %s, got %+v", body, payload)
		}
	}
}

func TestAPILibraryCommandsReturnsStructuredPlayFromHereResult(t *testing.T) {
	dbPath := writeLibraryFixture(t)
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := servePlaybackSequence(t, listener, []playbackExchange{
		{
			expectedLine: "STATUS\n",
			responseLine: "OK\tkind=status\tstate=stopped\torder_mode=sequential\trepeat_mode=off\tcurrent_track=\tlast_command=status\tqueue_entries=0\n",
		},
		{
			expectedLine: "QUEUE_PLAY [\"track-001\",\"track-002\"]\n",
			responseLine: "OK\tkind=ack\taction=queue_play\tstate=quiet_active\tcurrent_track=track-001\n",
		},
		{
			expectedLine: "STATUS\n",
			responseLine: "OK\tkind=status\tstate=quiet_active\torder_mode=sequential\trepeat_mode=off\tcurrent_track=track-001\tlast_command=queue_play\tqueue_entries=2\n",
		},
		{
			expectedLine: "QUEUE_SNAPSHOT\n",
			responseLine: "OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":0,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-001\",\"volume_uuid\":\"vol-001\",\"relative_path\":\"/Albums/Blue Room Sessions/01-opening.flac\",\"title\":\"Opening\",\"duration_ms\":201000,\"is_current\":true},{\"order_index\":1,\"queue_entry_id\":\"q2\",\"track_uid\":\"track-002\",\"volume_uuid\":\"vol-001\",\"relative_path\":\"/Albums/Blue Room Sessions/02-night-signal.flac\",\"title\":\"Night Signal\",\"duration_ms\":280000,\"is_current\":false}]}\n",
		},
	})

	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(t.TempDir(), "unused-evt.sock")),
		Library:      libraryclient.New(dbPath),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodPost, "/api/v1/library/commands", strings.NewReader(`{"action":"play","track_id":"track-001","query":{"album_uid":"album-001"}}`))
	request.Header.Set("Content-Type", "application/json")
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		OK      bool   `json:"ok"`
		Action  string `json:"action"`
		TrackID string `json:"track_id"`
		Query   struct {
			AlbumUID string `json:"album_uid"`
		} `json:"query"`
		Message        string `json:"message"`
		Error          string `json:"error"`
		PlaybackStatus struct {
			State        string `json:"state"`
			CurrentTrack string `json:"current_track"`
			QueueEntries int    `json:"queue_entries"`
		} `json:"playback_status"`
		Queue struct {
			Entries []struct {
				TrackUID string `json:"track_uid"`
			} `json:"entries"`
		} `json:"queue"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode library command response: %v", err)
	}

	if !payload.OK || payload.Action != "play" || payload.TrackID != "track-001" || payload.Query.AlbumUID != "album-001" {
		t.Fatalf("unexpected library command payload: %+v", payload)
	}
	if payload.Message != "" || payload.Error != "" {
		t.Fatalf("unexpected library command result fields: %+v", payload)
	}
	if payload.PlaybackStatus.State != "quiet_active" || payload.PlaybackStatus.CurrentTrack != "track-001" || payload.PlaybackStatus.QueueEntries != 2 {
		t.Fatalf("unexpected playback status after library command: %+v", payload.PlaybackStatus)
	}
	if len(payload.Queue.Entries) != 2 || payload.Queue.Entries[0].TrackUID != "track-001" || payload.Queue.Entries[1].TrackUID != "track-002" {
		t.Fatalf("unexpected queue after library command: %+v", payload.Queue)
	}

	<-done
}

func TestAPILibraryCommandsRejectsAbsolutePathTargets(t *testing.T) {
	dbPath := writeLibraryFixture(t)
	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(filepath.Join(t.TempDir(), "missing-playback-cmd.sock"), filepath.Join(t.TempDir(), "unused-evt.sock")),
		Library:      libraryclient.New(dbPath),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodPost, "/api/v1/library/commands", strings.NewReader(`{"action":"play","track_id":"/tmp/manual.wav","query":{"album_uid":"album-001"}}`))
	request.Header.Set("Content-Type", "application/json")
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusBadRequest {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload struct {
		OK    bool   `json:"ok"`
		Error string `json:"error"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode library command response: %v", err)
	}
	if payload.OK || !strings.Contains(payload.Error, "absolute_path_playback_forbidden") {
		t.Fatalf("expected absolute path rejection, got %+v", payload)
	}
}

func TestLibraryPageBootstrapsReadOnlyAPIContractForLiveSections(t *testing.T) {
	dbPath := writeLibraryFixture(t)
	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(filepath.Join(t.TempDir(), "missing-playback-cmd.sock"), filepath.Join(t.TempDir(), "missing-playback-evt.sock")),
		Library:      libraryclient.New(dbPath),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/library?album_uid=album-001", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	for _, fragment := range []string{
		"api\\/v1\\/library\\/snapshot",
		"api\\/v1\\/library\\/commands",
		"api\\/v1\\/library\\/media",
		"api\\/v1\\/library\\/media\\/commands",
		"api\\/v1\\/playback\\/status",
		"api\\/v1\\/playback\\/queue",
		"api\\/v1\\/playback\\/events",
	} {
		if !strings.Contains(body, fragment) {
			t.Fatalf("expected library page to reference %s, body: %s", fragment, body)
		}
	}
	if strings.Contains(body, "window.location.reload()") {
		t.Fatalf("expected library page to refresh sections via API instead of full reload, body: %s", body)
	}
	if !strings.Contains(body, "data-track-uid=\"track-001\"") {
		t.Fatalf("expected track rows to expose track uid markers, body: %s", body)
	}
	for _, fragment := range []string{
		"id=\"library-volumes-list\"",
		"id=\"library-directories-list\"",
		"id=\"library-albums-list\"",
		"id=\"library-tracks-list\"",
		"renderLibraryCollections",
		"browser-side browse navigation",
		"readLibraryQueryFromURL",
		"window.addEventListener(\"popstate\"",
		"window.history.replaceState",
	} {
		if !strings.Contains(body, fragment) {
			t.Fatalf("expected library page to expose API render hook %s, body: %s", fragment, body)
		}
	}
}

func TestLibraryPageRendersMediaImportControls(t *testing.T) {
	dbPath := writeLibraryFixture(t)
	server := newServerWithDeps(t, api.Dependencies{
		Auth:     auth.NewService(false),
		Playback: playbackclient.New(filepath.Join(t.TempDir(), "missing-playback-cmd.sock"), filepath.Join(t.TempDir(), "missing-playback-evt.sock")),
		Library:  libraryclient.New(dbPath),
		MediaImport: &fakeMediaImportSource{
			snapshot: mediaimport.Snapshot{
				Available: true,
				Devices: []mediaimport.Device{{
					Label:      "TF Music",
					Path:       "/dev/sda1",
					FSType:     "exfat",
					Mountpoint: "/media/tf-music",
					IsMounted:  true,
					VolumeUUID: "media-uuid-demo",
				}},
			},
		},
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/library", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	for _, fragment := range []string{
		"本地介质",
		"TF / USB 歌曲扫描",
		"重新检测 TF / USB",
		"扫描全部已挂载 TF / USB",
		"扫描指定目录",
		"TF Music",
		"/dev/sda1",
		"/media/tf-music",
		"扫描这张 TF / USB 卡",
	} {
		if !strings.Contains(body, fragment) {
			t.Fatalf("expected library page media controls to include %s, body: %s", fragment, body)
		}
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
	if !strings.Contains(body, "系统日志") {
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
	if !strings.Contains(body, "设备摘要") || !strings.Contains(body, "dhcp_timeout") || !strings.Contains(body, "Studio WiFi") {
		t.Fatalf("expected provisioning summary on home page, body: %s", body)
	}
}

func TestHomePagePrefersLiveInterfaceModeFromProvisioningSnapshot(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available:     true,
			State:         "connected",
			IP:            "192.168.1.44",
			WiFiIP:        "192.168.1.44",
			WiFiInterface: "wlan0",
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	if !strings.Contains(body, "<dt>网络</dt>") || !strings.Contains(body, "id=\"defaults-interface\">无线</dd>") {
		t.Fatalf("expected live interface mode on home page, body: %s", body)
	}
}

func TestHomePageBootstrapsReadOnlyAPIContractForLiveSections(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available:     true,
			State:         "connected",
			IP:            "192.168.1.44",
			WiFiIP:        "192.168.1.44",
			WiFiInterface: "wlan0",
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	for _, fragment := range []string{
		"api\\/v1\\/system\\/summary",
		"api\\/v1\\/provisioning\\/status",
		"api\\/v1\\/playback\\/commands",
		"api\\/v1\\/playback\\/status",
		"api\\/v1\\/playback\\/queue",
		"api\\/v1\\/playback\\/events",
		"data-order-mode-value=\"sequential\"",
		"data-repeat-mode-value=\"all\"",
	} {
		if !strings.Contains(body, fragment) {
			t.Fatalf("expected home page to reference %s, body: %s", fragment, body)
		}
	}
	if strings.Contains(body, "window.location.reload()") {
		t.Fatalf("expected home page to refresh sections via API instead of full reload, body: %s", body)
	}
}

func TestHomePageRendersNowPlayingAudioFormat(t *testing.T) {
	dbPath := writeLibraryFixture(t)
	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := servePlaybackSequence(t, listener, []playbackExchange{
		{
			expectedLine: "STATUS\n",
			responseLine: "OK\tkind=status\tstate=quiet_active\torder_mode=sequential\trepeat_mode=off\tcurrent_track=track-001\tlast_command=play:track-001\tqueue_entries=1\n",
		},
		{
			expectedLine: "QUEUE_SNAPSHOT\n",
			responseLine: "OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":0,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-001\",\"volume_uuid\":\"vol-001\",\"relative_path\":\"/Albums/Blue Room Sessions/01-opening.flac\",\"title\":\"Opening\",\"duration_ms\":201000,\"is_current\":true}]}\n",
		},
		{
			expectedLine: "HISTORY_SNAPSHOT\n",
			responseLine: "OK\tkind=history_snapshot\tpayload={\"entries\":[]}\n",
		},
	})

	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(t.TempDir(), "unused-evt.sock")),
		Library:      libraryclient.New(dbPath),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if !strings.Contains(response.Body.String(), "音频格式：PCM · FLAC · 44.1 kHz") {
		t.Fatalf("expected now playing audio format on home page, body: %s", response.Body.String())
	}

	<-done
}

func TestHomePageRendersDSDRateAudioFormat(t *testing.T) {
	dbPath := writeLibraryFixture(t)
	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		t.Fatalf("open sqlite db: %v", err)
	}
	if _, err := db.Exec("UPDATE tracks SET format = 'dff', sample_rate = 2822400 WHERE track_uid = 'track-001'"); err != nil {
		t.Fatalf("update fixture track format: %v", err)
	}
	if err := db.Close(); err != nil {
		t.Fatalf("close sqlite db: %v", err)
	}

	socketPath := shortSocketPath(t)
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatalf("listen unix socket: %v", err)
	}
	defer listener.Close()

	done := servePlaybackSequence(t, listener, []playbackExchange{
		{
			expectedLine: "STATUS\n",
			responseLine: "OK\tkind=status\tstate=quiet_active\torder_mode=sequential\trepeat_mode=off\tcurrent_track=track-001\tlast_command=play:track-001\tqueue_entries=1\n",
		},
		{
			expectedLine: "QUEUE_SNAPSHOT\n",
			responseLine: "OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":0,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-001\",\"volume_uuid\":\"vol-001\",\"relative_path\":\"/Albums/Blue Room Sessions/01-opening.dff\",\"title\":\"Opening\",\"duration_ms\":201000,\"is_current\":true}]}\n",
		},
		{
			expectedLine: "HISTORY_SNAPSHOT\n",
			responseLine: "OK\tkind=history_snapshot\tpayload={\"entries\":[]}\n",
		},
	})

	server := newServerWithDeps(t, api.Dependencies{
		Auth:         auth.NewService(false),
		Playback:     playbackclient.New(socketPath, filepath.Join(t.TempDir(), "unused-evt.sock")),
		Library:      libraryclient.New(dbPath),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		Settings:     settings.Default(),
		SSH:          sshctl.NewController(false),
		Templates:    web.Assets,
		Static:       web.Assets,
	})

	request := httptest.NewRequest(http.MethodGet, "/", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if !strings.Contains(response.Body.String(), "音频格式：DSD64 · DFF · 2.8224 MHz") {
		t.Fatalf("expected DSD rate on home page, body: %s", response.Body.String())
	}

	<-done
}

func TestProvisioningStatusReturnsLatestSnapshot(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available:     true,
			State:         "connected",
			Message:       "wifi connected",
			SSID:          "Home WiFi",
			IP:            "192.168.1.44",
			WiFiIP:        "192.168.43.170",
			WiredIP:       "192.168.1.120",
			AllIPs:        []string{"192.168.1.120", "192.168.43.170"},
			WebURL:        "http://192.168.1.44/",
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
	if payload.WiFiIP != "192.168.43.170" || payload.WiredIP != "192.168.1.120" || len(payload.AllIPs) != 2 {
		t.Fatalf("expected dual-ip payload: %+v", payload)
	}
}

func TestProvisioningPageRendersSnapshotDetails(t *testing.T) {
	server := newTestServer(t, &fakeLogSource{text: "boot ok\n"}, &fakeProvisioningSource{
		snapshot: provisioningclient.Snapshot{
			Available: true,
			State:     "connected",
			Message:   "wifi connected on wlan0; classic bluetooth provisioning remains available",
			SSID:      "Studio WiFi",
			IP:        "192.168.43.170",
			WiFiIP:    "192.168.43.170",
			WiredIP:   "192.168.1.120",
			AllIPs:    []string{"192.168.1.120", "192.168.43.170"},
		},
	})

	request := httptest.NewRequest(http.MethodGet, "/provisioning", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	if !strings.Contains(body, "网络与设备") || !strings.Contains(body, "Studio WiFi") || !strings.Contains(body, "192.168.1.120") {
		t.Fatalf("expected provisioning page content, body: %s", body)
	}
}

func TestProvisioningPageRendersAudioOutputSelector(t *testing.T) {
	tempDir := t.TempDir()
	server, err := api.New(api.Dependencies{
		Auth: auth.NewService(false),
		Playback: playbackclient.New(
			filepath.Join(tempDir, "missing-playback-cmd.sock"),
			filepath.Join(tempDir, "missing-playback-evt.sock"),
		),
		Library:      libraryclient.New(filepath.Join(tempDir, "missing-library.db")),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		AudioOutput: &fakeAudioOutputSource{snapshot: audiodevice.Snapshot{
			Connected: true,
			Current: audiodevice.Device{
				CardIndex:  1,
				CardID:     "Audio",
				Name:       "XMOS xCORE USB Audio 2.0",
				ALSADevice: "plughw:CARD=Audio,DEV=0",
			},
		}},
		Settings:  settings.Default(),
		SSH:       sshctl.NewController(false),
		Templates: web.Assets,
		Static:    web.Assets,
	})
	if err != nil {
		t.Fatalf("build server: %v", err)
	}

	request := httptest.NewRequest(http.MethodGet, "/provisioning", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	body := response.Body.String()
	if !strings.Contains(body, "当前解码器") || !strings.Contains(body, "XMOS xCORE USB Audio 2.0") || !strings.Contains(body, "plughw:CARD=Audio,DEV=0") {
		t.Fatalf("expected audio output selector, body: %s", body)
	}
}

func TestAudioOutputAPIReportsSnapshot(t *testing.T) {
	tempDir := t.TempDir()
	server, err := api.New(api.Dependencies{
		Auth: auth.NewService(false),
		Playback: playbackclient.New(
			filepath.Join(tempDir, "missing-playback-cmd.sock"),
			filepath.Join(tempDir, "missing-playback-evt.sock"),
		),
		Library:      libraryclient.New(filepath.Join(tempDir, "missing-library.db")),
		Logs:         &fakeLogSource{text: "boot ok\n"},
		Provisioning: &fakeProvisioningSource{},
		AudioOutput: &fakeAudioOutputSource{snapshot: audiodevice.Snapshot{
			Connected: true,
			Current: audiodevice.Device{
				CardIndex:  1,
				CardID:     "Audio",
				Name:       "DAC One",
				ALSADevice: "plughw:CARD=Audio,DEV=0",
			},
		}},
		Settings:  settings.Default(),
		SSH:       sshctl.NewController(false),
		Templates: web.Assets,
		Static:    web.Assets,
	})
	if err != nil {
		t.Fatalf("build server: %v", err)
	}

	request := httptest.NewRequest(http.MethodGet, "/api/v1/system/audio-output", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}

	var payload audiodevice.Snapshot
	if err := json.Unmarshal(response.Body.Bytes(), &payload); err != nil {
		t.Fatalf("decode audio output payload: %v", err)
	}
	if !payload.Connected || payload.Current.Name != "DAC One" {
		t.Fatalf("unexpected audio output payload: %+v", payload)
	}
}

func TestArtworkRouteServesCachedThumbnail(t *testing.T) {
	tempDir := t.TempDir()
	artworkDir := filepath.Join(tempDir, "artwork")
	thumbPath := filepath.Join(artworkDir, "thumb", "320", "aa", "bb", "fixture.jpg")
	if err := os.MkdirAll(filepath.Dir(thumbPath), 0o755); err != nil {
		t.Fatalf("mkdir artwork dir: %v", err)
	}
	if err := os.WriteFile(thumbPath, []byte("jpeg-fixture"), 0o644); err != nil {
		t.Fatalf("write artwork fixture: %v", err)
	}

	server, err := api.New(api.Dependencies{
		Auth: auth.NewService(false),
		Playback: playbackclient.New(
			filepath.Join(tempDir, "missing-playback-cmd.sock"),
			filepath.Join(tempDir, "missing-playback-evt.sock"),
		),
		Library:          libraryclient.New(filepath.Join(tempDir, "missing-library.db")),
		Logs:             &fakeLogSource{text: "boot ok\n"},
		Provisioning:     &fakeProvisioningSource{},
		Settings:         settings.Default(),
		SSH:              sshctl.NewController(false),
		Templates:        web.Assets,
		Static:           web.Assets,
		ArtworkCacheRoot: artworkDir,
	})
	if err != nil {
		t.Fatalf("build server: %v", err)
	}

	request := httptest.NewRequest(http.MethodGet, "/artwork/thumb/320/aa/bb/fixture.jpg", nil)
	response := httptest.NewRecorder()

	server.Handler().ServeHTTP(response, request)

	if response.Code != http.StatusOK {
		t.Fatalf("unexpected status: %d", response.Code)
	}
	if body := response.Body.String(); body != "jpeg-fixture" {
		t.Fatalf("unexpected artwork body: %q", body)
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
		AudioOutput:  &fakeAudioOutputSource{},
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

func newServerWithDeps(t *testing.T, deps api.Dependencies) *api.Server {
	t.Helper()

	server, err := api.New(deps)
	if err != nil {
		t.Fatalf("build server: %v", err)
	}
	return server
}

func writeLibraryFixture(t *testing.T) string {
	t.Helper()

	dbPath := filepath.Join(t.TempDir(), "library.db")
	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		t.Fatalf("open sqlite db: %v", err)
	}
	defer db.Close()

	if _, err := db.Exec(`
		CREATE TABLE volumes (
			volume_uuid TEXT PRIMARY KEY,
			label TEXT,
			mount_path TEXT NOT NULL,
			is_available INTEGER NOT NULL,
			last_seen_at INTEGER NOT NULL
		);
		CREATE TABLE directories (
			directory_id INTEGER PRIMARY KEY,
			volume_uuid TEXT NOT NULL,
			relative_path TEXT NOT NULL,
			parent_relative_path TEXT,
			display_name TEXT NOT NULL,
			indexed_at INTEGER NOT NULL
		);
		CREATE TABLE artwork_refs (
			artwork_ref_id INTEGER PRIMARY KEY,
			thumb_rel_path TEXT
		);
		CREATE TABLE albums (
			album_id INTEGER PRIMARY KEY,
			album_uid TEXT NOT NULL UNIQUE,
			album_title TEXT NOT NULL,
			album_artist TEXT,
			album_artist_norm TEXT,
			year INTEGER,
			track_count INTEGER NOT NULL,
			total_duration_ms INTEGER NOT NULL,
			album_root_dir_hint TEXT,
			cover_ref_id INTEGER,
			indexed_at INTEGER NOT NULL,
			album_title_norm TEXT NOT NULL,
			volume_uuid TEXT NOT NULL,
			source_mode TEXT NOT NULL DEFAULT 'folder'
		);
		CREATE TABLE tracks (
			track_uid TEXT PRIMARY KEY,
			album_id INTEGER,
			volume_uuid TEXT,
			title TEXT,
			filename TEXT NOT NULL,
			artist TEXT,
			album_artist TEXT,
			relative_path TEXT NOT NULL,
			track_no INTEGER,
			disc_no INTEGER,
			format TEXT,
			duration_ms INTEGER,
			sample_rate INTEGER,
			indexed_at INTEGER NOT NULL
		);
		CREATE TABLE artists (
			artist_id INTEGER PRIMARY KEY,
			artist_name TEXT NOT NULL
		);
		CREATE TABLE genres (
			genre_id INTEGER PRIMARY KEY,
			genre_name TEXT NOT NULL
		);
	`); err != nil {
		t.Fatalf("create library schema: %v", err)
	}

	if _, err := db.Exec(`
		INSERT INTO volumes (volume_uuid, label, mount_path, is_available, last_seen_at)
		VALUES ('vol-001', 'Demo TF', '/media/demo', 1, 1710000000);
		INSERT INTO artwork_refs (artwork_ref_id, thumb_rel_path)
		VALUES (1, 'artwork/demo-thumb.webp');
		INSERT INTO albums (
			album_id, album_uid, album_title, album_artist, album_artist_norm, year, track_count, total_duration_ms,
			album_root_dir_hint, cover_ref_id, indexed_at, album_title_norm, volume_uuid, source_mode
		) VALUES (
			1, 'album-001', 'Blue Room Sessions', 'Demo Artist', 'demo artist', 2024, 2, 481000,
			'/Albums/Blue Room Sessions', 1, 1710000100, 'blue room sessions', 'vol-001', 'tag'
		);
		INSERT INTO tracks (
			track_uid, album_id, volume_uuid, title, filename, artist, album_artist, relative_path,
			track_no, disc_no, format, duration_ms, sample_rate, indexed_at
		) VALUES
			('track-001', 1, 'vol-001', 'Opening', '01-opening.flac', 'Demo Artist', 'Demo Artist', '/Albums/Blue Room Sessions/01-opening.flac', 1, 1, 'flac', 201000, 44100, 1710000101),
			('track-002', 1, 'vol-001', 'Night Signal', '02-night-signal.flac', 'Demo Artist', 'Demo Artist', '/Albums/Blue Room Sessions/02-night-signal.flac', 2, 1, 'flac', 280000, 44100, 1710000102);
		INSERT INTO artists (artist_id, artist_name) VALUES (1, 'Demo Artist');
		INSERT INTO genres (genre_id, genre_name) VALUES (1, 'Ambient');
	`); err != nil {
		t.Fatalf("seed library rows: %v", err)
	}

	return dbPath
}

func shortSocketPath(t *testing.T) string {
	t.Helper()

	dir, err := os.MkdirTemp("/tmp", "lumelo-api-uds-")
	if err != nil {
		t.Fatalf("create short temp dir: %v", err)
	}
	t.Cleanup(func() {
		_ = os.RemoveAll(dir)
	})

	return filepath.Join(dir, "playback.sock")
}

type playbackExchange struct {
	expectedLine string
	responseLine string
}

func servePlaybackSequence(t *testing.T, listener net.Listener, exchanges []playbackExchange) <-chan struct{} {
	t.Helper()

	done := make(chan struct{})
	go func() {
		defer close(done)
		for _, exchange := range exchanges {
			conn, err := listener.Accept()
			if err != nil {
				return
			}

			buf := make([]byte, 4096)
			n, _ := conn.Read(buf)
			if string(buf[:n]) != exchange.expectedLine {
				t.Errorf("unexpected command line: got %q want %q", string(buf[:n]), exchange.expectedLine)
				_ = conn.Close()
				return
			}

			_, _ = conn.Write([]byte(exchange.responseLine))
			_ = conn.Close()
		}
	}()

	return done
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

type fakeAudioOutputSource struct {
	snapshot audiodevice.Snapshot
}

func (f *fakeAudioOutputSource) Snapshot(context.Context) audiodevice.Snapshot {
	return f.snapshot
}

type fakeMediaImportSource struct {
	snapshot     mediaimport.Snapshot
	result       mediaimport.CommandResult
	err          error
	lastRequest  mediaimport.CommandRequest
	executeCount int
}

func (f *fakeMediaImportSource) Snapshot(context.Context) mediaimport.Snapshot {
	return f.snapshot
}

func (f *fakeMediaImportSource) Execute(_ context.Context, request mediaimport.CommandRequest) (mediaimport.CommandResult, error) {
	f.lastRequest = request
	f.executeCount++
	if f.result.Action == "" {
		f.result.Action = request.Action
	}
	return f.result, f.err
}
