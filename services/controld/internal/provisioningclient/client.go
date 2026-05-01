package provisioningclient

import (
	"context"
	"encoding/json"
	"os"
)

type Snapshot struct {
	Available        bool     `json:"available"`
	State            string   `json:"state,omitempty"`
	Message          string   `json:"message,omitempty"`
	SSID             string   `json:"ssid,omitempty"`
	IP               string   `json:"ip,omitempty"`
	WiFiIP           string   `json:"wifi_ip,omitempty"`
	WiredIP          string   `json:"wired_ip,omitempty"`
	AllIPs           []string `json:"all_ips,omitempty"`
	WebURL           string   `json:"web_url,omitempty"`
	Hostname         string   `json:"hostname,omitempty"`
	WiFiInterface    string   `json:"wifi_interface,omitempty"`
	WPAUnit          string   `json:"wpa_unit,omitempty"`
	StatusPath       string   `json:"status_path,omitempty"`
	UpdatedAt        string   `json:"updated_at,omitempty"`
	Error            string   `json:"error,omitempty"`
	ErrorCode        string   `json:"error_code,omitempty"`
	ApplyOutput      string   `json:"apply_output,omitempty"`
	DiagnosticHint   string   `json:"diagnostic_hint,omitempty"`
	IPWaitSeconds    int      `json:"ip_wait_seconds,omitempty"`
	BluetoothAlias   string   `json:"bluetooth_alias,omitempty"`
	BluetoothAddress string   `json:"bluetooth_address,omitempty"`
	RFCOMMChannel    int      `json:"rfcomm_channel,omitempty"`
	SDPRecordHandles []string `json:"sdp_record_handles,omitempty"`
	ReadError        string   `json:"read_error,omitempty"`
}

type Client struct {
	StatusPath string
}

func New(statusPath string) *Client {
	return &Client{StatusPath: statusPath}
}

func (c *Client) Snapshot(context.Context) Snapshot {
	snapshot := Snapshot{}
	if c == nil {
		snapshot.ReadError = "provisioning status path is not configured"
		return snapshot
	}

	snapshot.StatusPath = c.StatusPath
	if c.StatusPath == "" {
		snapshot.ReadError = "provisioning status path is not configured"
		return snapshot
	}

	payload, err := os.ReadFile(c.StatusPath)
	if err != nil {
		snapshot.ReadError = err.Error()
		return snapshot
	}
	if err := json.Unmarshal(payload, &snapshot); err != nil {
		snapshot.ReadError = err.Error()
		return snapshot
	}

	snapshot.Available = true
	if snapshot.StatusPath == "" {
		snapshot.StatusPath = c.StatusPath
	}
	return snapshot
}
