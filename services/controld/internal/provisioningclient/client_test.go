package provisioningclient_test

import (
	"context"
	"os"
	"path/filepath"
	"testing"

	"github.com/lumelo/controld/internal/provisioningclient"
)

func TestSnapshotReadsProvisioningStateFile(t *testing.T) {
	tempDir := t.TempDir()
	statusPath := filepath.Join(tempDir, "provisioning-status.json")
	if err := os.WriteFile(statusPath, []byte("{\"state\":\"connected\",\"message\":\"wifi connected\",\"ssid\":\"Home WiFi\",\"ip\":\"192.168.1.44\",\"wifi_ip\":\"192.168.43.170\",\"wired_ip\":\"192.168.1.120\",\"all_ips\":[\"192.168.1.120\",\"192.168.43.170\"],\"web_url\":\"http://192.168.1.44:18080/\",\"wifi_interface\":\"wlan0\",\"wpa_unit\":\"wpa_supplicant@wlan0.service\",\"diagnostic_hint\":\"Open /provisioning, /healthz, and /logs from the phone browser\"}\n"), 0o644); err != nil {
		t.Fatalf("write status file: %v", err)
	}

	client := provisioningclient.New(statusPath)
	snapshot := client.Snapshot(context.Background())

	if !snapshot.Available {
		t.Fatalf("expected snapshot to be available: %+v", snapshot)
	}
	if snapshot.State != "connected" || snapshot.IP != "192.168.1.44" || snapshot.WiFiInterface != "wlan0" {
		t.Fatalf("unexpected snapshot: %+v", snapshot)
	}
	if snapshot.WPAUnit != "wpa_supplicant@wlan0.service" || snapshot.DiagnosticHint == "" {
		t.Fatalf("expected richer diagnostics: %+v", snapshot)
	}
	if snapshot.WiFiIP != "192.168.43.170" || snapshot.WiredIP != "192.168.1.120" || len(snapshot.AllIPs) != 2 {
		t.Fatalf("expected dual-ip details: %+v", snapshot)
	}
	if snapshot.ReadError != "" {
		t.Fatalf("expected empty read error: %+v", snapshot)
	}
}

func TestSnapshotReportsReadErrorWhenFileIsMissing(t *testing.T) {
	client := provisioningclient.New(filepath.Join(t.TempDir(), "missing.json"))
	snapshot := client.Snapshot(context.Background())

	if snapshot.Available {
		t.Fatalf("expected snapshot to be unavailable: %+v", snapshot)
	}
	if snapshot.ReadError == "" {
		t.Fatalf("expected read error: %+v", snapshot)
	}
}
