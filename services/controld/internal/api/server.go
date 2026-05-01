package api

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"html/template"
	"io/fs"
	"net/http"
	"net/url"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/lumelo/controld/internal/audiodevice"
	"github.com/lumelo/controld/internal/auth"
	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/mediaimport"
	"github.com/lumelo/controld/internal/playbackclient"
	"github.com/lumelo/controld/internal/provisioningclient"
	"github.com/lumelo/controld/internal/settings"
	"github.com/lumelo/controld/internal/sshctl"
)

type Dependencies struct {
	Auth             *auth.Service
	Playback         *playbackclient.Client
	Library          *libraryclient.Client
	MediaImport      MediaImportSource
	Logs             LogSource
	Provisioning     ProvisioningSource
	Settings         settings.Config
	SSH              *sshctl.Controller
	Templates        fs.FS
	Static           fs.FS
	ArtworkCacheRoot string
	AudioOutput      AudioOutputSource
}

type LogSource interface {
	Recent(ctx context.Context, lines int) (string, error)
}

type ProvisioningSource interface {
	Snapshot(ctx context.Context) provisioningclient.Snapshot
}

type AudioOutputSource interface {
	Snapshot(ctx context.Context) audiodevice.Snapshot
}

type MediaImportSource interface {
	Snapshot(ctx context.Context) mediaimport.Snapshot
	Execute(ctx context.Context, request mediaimport.CommandRequest) (mediaimport.CommandResult, error)
}

type Server struct {
	handler http.Handler
}

type homeViewData struct {
	InlineCSS            template.CSS
	CurrentPage          string
	Mode                 string
	InterfaceMode        string
	DSDPolicy            string
	PasswordConfigured   bool
	SSHEnabled           bool
	CommandSocket        string
	EventSocket          string
	LibraryDBPath        string
	ConfigPath           string
	PlaybackStatus       playbackclient.Status
	NowPlaying           libraryNowPlayingView
	QueueSnapshot        playbackclient.QueueSnapshot
	QueueEntries         []queueEntryView
	HistorySnapshot      playbackclient.HistorySnapshot
	HistoryEntries       []historyEntryView
	FeaturedAlbums       []libraryAlbumView
	CurrentOrderLabel    string
	CommandMessage       string
	CommandError         string
	SuggestedTrackID     string
	SystemSummaryPath    string
	SystemHealthPath     string
	PlaybackCommandsPath string
	LibrarySnapshotPath  string
	ProvisioningPath     string
	ProvisioningAPIPath  string
	PlaybackStatusPath   string
	PlaybackQueuePath    string
	PlaybackHistoryPath  string
	PlaybackStreamPath   string
	Provisioning         provisioningclient.Snapshot
}

type libraryViewData struct {
	InlineCSS                   template.CSS
	CurrentPage                 string
	LibraryDBPath               string
	LibrarySnapshotPath         string
	LibraryCommandsPath         string
	LibraryMediaPath            string
	LibraryMediaCommandsPath    string
	LibraryMediaFormPath        string
	PlaybackStatusPath          string
	PlaybackQueuePath           string
	PlaybackStreamPath          string
	LibrarySnapshot             libraryclient.Snapshot
	MediaSnapshot               mediaimport.Snapshot
	PlaybackStatus              playbackclient.Status
	QueueSnapshot               playbackclient.QueueSnapshot
	PlaybackScanBlock           bool
	MediaCommandOutput          string
	NowPlaying                  libraryNowPlayingView
	CommandMessage              string
	CommandError                string
	SelectedAlbumUID            string
	SelectedAlbumTitle          string
	SelectedDirectoryVolumeUUID string
	SelectedDirectoryPath       string
	SelectedDirectoryTitle      string
	ParentDirectoryBrowsePath   string
	VolumeEntries               []libraryVolumeView
	DirectoryEntries            []libraryDirectoryView
	AlbumEntries                []libraryAlbumView
	TrackEntries                []libraryTrackView
}

type logsViewData struct {
	InlineCSS   template.CSS
	CurrentPage string
	Lines       int
	LogText     string
	LogError    string
	LogTextPath string
}

type provisioningViewData struct {
	InlineCSS    template.CSS
	CurrentPage  string
	Provisioning provisioningclient.Snapshot
	AudioOutput  audiodevice.Snapshot
	RawJSON      string
}

type queueEntryView struct {
	DisplayIndex string
	QueueEntryID string
	TrackUID     string
	RelativePath string
	Title        string
	IsCurrent    bool
}

type historyEntryView struct {
	DisplayIndex string
	TrackUID     string
	RelativePath string
	Title        string
	IsCurrent    bool
}

type libraryVolumeView struct {
	Label       string
	MountPath   string
	VolumeUUID  string
	LastSeenAt  string
	IsAvailable bool
	BrowsePath  string
}

type libraryDirectoryView struct {
	DisplayName  string
	RelativePath string
	BrowsePath   string
	IsSelected   bool
}

type libraryAlbumView struct {
	AlbumUID        string
	Title           string
	AlbumArtist     string
	YearLabel       string
	TrackCount      int
	DurationLabel   string
	RootDirHint     string
	CoverThumbLabel string
	CoverThumbPath  string
	BrowsePath      string
	IsSelected      bool
	IsSynthetic     bool
}

type libraryTrackView struct {
	TrackNumberLabel string
	TrackUID         string
	AlbumUID         string
	AlbumTitle       string
	Title            string
	Artist           string
	RelativePath     string
	FormatLabel      string
	DurationLabel    string
	IsCurrent        bool
	CanPlay          bool
	SupportLabel     string
}

type libraryNowPlayingView struct {
	Known            bool
	Title            string
	Artist           string
	AlbumTitle       string
	RelativePath     string
	CoverThumbPath   string
	AudioFormatLabel string
	State            string
	TrackUID         string
}

type healthView struct {
	Status                string `json:"status"`
	Mode                  string `json:"mode"`
	InterfaceMode         string `json:"interface_mode"`
	SSHEnabled            bool   `json:"ssh_enabled"`
	PlaybackAvailable     bool   `json:"playback_available"`
	PlaybackState         string `json:"playback_state,omitempty"`
	PlaybackError         string `json:"playback_error,omitempty"`
	LibraryAvailable      bool   `json:"library_available"`
	LibraryDBPath         string `json:"library_db_path"`
	LibraryError          string `json:"library_error,omitempty"`
	ProvisioningAvailable bool   `json:"provisioning_available"`
	ProvisioningState     string `json:"provisioning_state,omitempty"`
	ProvisioningMessage   string `json:"provisioning_message,omitempty"`
	ProvisioningErrorCode string `json:"provisioning_error_code,omitempty"`
	ProvisioningBTAddress string `json:"provisioning_bluetooth_address,omitempty"`
	ProvisioningRFCOMM    int    `json:"provisioning_rfcomm_channel,omitempty"`
	ProvisioningSDPCount  int    `json:"provisioning_sdp_record_count,omitempty"`
	ProvisioningReadError string `json:"provisioning_read_error,omitempty"`
}

const defaultLogLines = 300

func effectiveInterfaceMode(configured string, provisioning provisioningclient.Snapshot) string {
	if mode := interfaceModeFromProvisioning(provisioning); mode != "" {
		return mode
	}
	return configured
}

func modeLabel(mode string) string {
	switch strings.ToLower(strings.TrimSpace(mode)) {
	case "local":
		return "本地模式"
	case "bridge":
		return "桥接模式"
	case "":
		return "-"
	default:
		return mode
	}
}

func interfaceModeLabel(mode string) string {
	switch strings.ToLower(strings.TrimSpace(mode)) {
	case "ethernet":
		return "有线"
	case "wifi":
		return "无线"
	case "":
		return "-"
	default:
		return mode
	}
}

func playbackStateLabel(state string) string {
	switch strings.ToLower(strings.TrimSpace(state)) {
	case "quiet_active", "playing":
		return "播放中"
	case "pre_quiet", "buffering":
		return "准备播放"
	case "paused":
		return "已暂停"
	case "stopped":
		return "已停止"
	case "idle":
		return "空闲"
	case "failed":
		return "失败"
	case "":
		return "未知状态"
	default:
		return state
	}
}

func transportPrimaryAction(state string) string {
	switch strings.ToLower(strings.TrimSpace(state)) {
	case "playing", "quiet_active", "pre_quiet":
		return "pause"
	default:
		return "play"
	}
}

func transportPrimaryLabel(state string) string {
	switch strings.ToLower(strings.TrimSpace(state)) {
	case "playing", "quiet_active", "pre_quiet":
		return "暂停"
	default:
		return "播放"
	}
}

func orderModeLabel(mode string) string {
	switch strings.ToLower(strings.TrimSpace(mode)) {
	case "sequential":
		return "顺序播放"
	case "shuffle":
		return "随机播放"
	case "":
		return "-"
	default:
		return mode
	}
}

func repeatModeLabel(mode string) string {
	switch strings.ToLower(strings.TrimSpace(mode)) {
	case "off":
		return "不循环"
	case "one":
		return "单曲循环"
	case "all":
		return "列表循环"
	case "":
		return "-"
	default:
		return mode
	}
}

func provisioningStateLabel(state string, available bool) string {
	if !available {
		return "不可用"
	}

	switch strings.ToLower(strings.TrimSpace(state)) {
	case "connected":
		return "已连接"
	case "failed":
		return "失败"
	case "idle":
		return "空闲"
	case "pending", "applying":
		return "处理中"
	case "connecting":
		return "连接中"
	case "scanning":
		return "扫描中"
	case "":
		return "未知状态"
	default:
		return state
	}
}

func boolLabel(value bool, trueLabel, falseLabel string) string {
	if value {
		return trueLabel
	}
	return falseLabel
}

func interfaceModeFromProvisioning(snapshot provisioningclient.Snapshot) string {
	currentIP := strings.TrimSpace(snapshot.IP)
	wifiIP := strings.TrimSpace(snapshot.WiFiIP)
	wiredIP := strings.TrimSpace(snapshot.WiredIP)

	switch {
	case currentIP != "" && currentIP == wifiIP:
		return "wifi"
	case currentIP != "" && currentIP == wiredIP:
		return "ethernet"
	case wifiIP != "" && wiredIP == "":
		return "wifi"
	case wiredIP != "" && wifiIP == "":
		return "ethernet"
	case currentIP == "" && strings.EqualFold(strings.TrimSpace(snapshot.State), "connected") &&
		strings.TrimSpace(snapshot.WiFiInterface) != "" && wiredIP == "":
		return "wifi"
	default:
		return ""
	}
}

func New(deps Dependencies) (*Server, error) {
	tmpl, err := template.New("").Funcs(template.FuncMap{
		"boolLabel":              boolLabel,
		"interfaceModeLabel":     interfaceModeLabel,
		"modeLabel":              modeLabel,
		"orderModeLabel":         orderModeLabel,
		"playbackStateLabel":     playbackStateLabel,
		"provisioningStateLabel": provisioningStateLabel,
		"repeatModeLabel":        repeatModeLabel,
		"transportPrimaryAction": transportPrimaryAction,
		"transportPrimaryLabel":  transportPrimaryLabel,
	}).ParseFS(deps.Templates, "templates/*.html")
	if err != nil {
		return nil, fmt.Errorf("parse templates: %w", err)
	}

	staticFS, err := fs.Sub(deps.Static, "static")
	if err != nil {
		return nil, fmt.Errorf("load static assets: %w", err)
	}
	appCSSBytes, err := fs.ReadFile(staticFS, "css/app.css")
	if err != nil {
		return nil, fmt.Errorf("read app.css: %w", err)
	}
	appCSS := template.CSS(string(appCSSBytes))

	logs := deps.Logs
	if logs == nil {
		logs = unavailableLogSource{}
	}
	provisioning := deps.Provisioning
	if provisioning == nil {
		provisioning = unavailableProvisioningSource{}
	}
	audioOutput := deps.AudioOutput
	if audioOutput == nil {
		audioOutput = audiodevice.New("")
	}
	mediaImport := deps.MediaImport
	if mediaImport == nil {
		mediaImport = unavailableMediaImportSource{}
	}

	mux := http.NewServeMux()
	mux.Handle("/static/", http.StripPrefix("/static/", http.FileServer(http.FS(staticFS))))
	if deps.ArtworkCacheRoot != "" {
		mux.Handle("/artwork/", http.StripPrefix("/artwork/", http.FileServer(http.Dir(deps.ArtworkCacheRoot))))
	}

	renderHealth := func(w http.ResponseWriter, r *http.Request) {
		playbackStatus := deps.Playback.Status(r.Context())
		librarySnapshot := deps.Library.Snapshot(r.Context())
		provisioningSnapshot := provisioning.Snapshot(r.Context())
		response := buildHealthPayload(deps, playbackStatus, librarySnapshot, provisioningSnapshot)
		if err := writeJSON(w, response); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		renderHealth(w, r)
	})

	mux.HandleFunc("/api/v1/system/summary", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		if err := writeJSON(w, buildSystemSummaryView(deps, provisioning.Snapshot(r.Context()))); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/system/health", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		renderHealth(w, r)
	})

	mux.HandleFunc("/api/v1/system/audio-output", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		if err := writeJSON(w, audioOutput.Snapshot(r.Context())); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	renderHome := func(w http.ResponseWriter, r *http.Request, commandMessage, commandError string) {
		status := deps.Playback.Status(r.Context())
		queueSnapshot := deps.Playback.QueueSnapshot(r.Context())
		historySnapshot := deps.Playback.HistorySnapshot(r.Context())
		librarySnapshot := deps.Library.Snapshot(r.Context())
		provisioningSnapshot := provisioning.Snapshot(r.Context())

		data := homeViewData{
			InlineCSS:            appCSS,
			CurrentPage:          "home",
			Mode:                 deps.Settings.Mode,
			InterfaceMode:        effectiveInterfaceMode(deps.Settings.InterfaceMode, provisioningSnapshot),
			DSDPolicy:            deps.Settings.DSDPolicy,
			PasswordConfigured:   deps.Auth.PasswordConfigured(),
			SSHEnabled:           deps.SSH.Enabled(),
			CommandSocket:        deps.Playback.CommandSocket,
			EventSocket:          deps.Playback.EventSocket,
			LibraryDBPath:        deps.Library.LibraryDBPath,
			ConfigPath:           deps.Settings.ConfigPath,
			PlaybackStatus:       status,
			NowPlaying:           buildLibraryNowPlayingView(status, queueSnapshot, librarySnapshot),
			QueueSnapshot:        queueSnapshot,
			QueueEntries:         buildQueueEntryViews(queueSnapshot),
			HistorySnapshot:      historySnapshot,
			HistoryEntries:       buildHistoryEntryViews(historySnapshot, status, librarySnapshot),
			FeaturedAlbums:       buildHomeFeaturedAlbums(librarySnapshot, 6),
			CurrentOrderLabel:    currentOrderLabel(queueSnapshot),
			CommandMessage:       commandMessage,
			CommandError:         commandError,
			SuggestedTrackID:     suggestedTrackID(status, librarySnapshot),
			SystemSummaryPath:    "/api/v1/system/summary",
			SystemHealthPath:     "/api/v1/system/health",
			PlaybackCommandsPath: "/api/v1/playback/commands",
			LibrarySnapshotPath:  "/api/v1/library/snapshot",
			ProvisioningPath:     "/provisioning",
			ProvisioningAPIPath:  "/api/v1/provisioning/status",
			PlaybackStatusPath:   "/api/v1/playback/status",
			PlaybackQueuePath:    "/api/v1/playback/queue",
			PlaybackHistoryPath:  "/api/v1/playback/history",
			PlaybackStreamPath:   "/api/v1/playback/events",
			Provisioning:         provisioningSnapshot,
		}

		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if err := tmpl.ExecuteTemplate(w, "index.html", data); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" {
			http.NotFound(w, r)
			return
		}
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		renderHome(w, r, "", "")
	})

	renderLibrary := func(
		w http.ResponseWriter,
		r *http.Request,
		query libraryclient.Query,
		commandMessage, commandError, mediaCommandOutput string,
	) {
		snapshot := deps.Library.QuerySnapshot(r.Context(), query)
		mediaSnapshot := mediaImport.Snapshot(r.Context())
		playbackStatus := deps.Playback.Status(r.Context())
		queueSnapshot := deps.Playback.QueueSnapshot(r.Context())

		data := libraryViewData{
			InlineCSS:                   appCSS,
			CurrentPage:                 "library",
			LibraryDBPath:               deps.Library.LibraryDBPath,
			LibrarySnapshotPath:         "/api/v1/library/snapshot",
			LibraryCommandsPath:         "/api/v1/library/commands",
			LibraryMediaPath:            "/api/v1/library/media",
			LibraryMediaCommandsPath:    "/api/v1/library/media/commands",
			LibraryMediaFormPath:        "/library/media/commands",
			PlaybackStatusPath:          "/api/v1/playback/status",
			PlaybackQueuePath:           "/api/v1/playback/queue",
			PlaybackStreamPath:          "/api/v1/playback/events",
			LibrarySnapshot:             snapshot,
			MediaSnapshot:               mediaSnapshot,
			PlaybackStatus:              playbackStatus,
			QueueSnapshot:               queueSnapshot,
			PlaybackScanBlock:           playbackBlocksScan(playbackStatus),
			MediaCommandOutput:          mediaCommandOutput,
			NowPlaying:                  buildLibraryNowPlayingView(playbackStatus, queueSnapshot, snapshot),
			CommandMessage:              commandMessage,
			CommandError:                commandError,
			SelectedAlbumUID:            query.AlbumUID,
			SelectedAlbumTitle:          selectedAlbumTitle(snapshot, query.AlbumUID),
			SelectedDirectoryVolumeUUID: query.DirectoryVolumeUUID,
			SelectedDirectoryPath:       query.DirectoryPath,
			SelectedDirectoryTitle:      selectedDirectoryTitle(query),
			ParentDirectoryBrowsePath:   parentDirectoryBrowsePath(query),
			VolumeEntries:               buildLibraryVolumeViews(snapshot),
			DirectoryEntries:            buildLibraryDirectoryViews(snapshot, query),
			AlbumEntries:                buildLibraryAlbumViews(snapshot, query.AlbumUID),
			TrackEntries:                buildLibraryTrackViews(snapshot, playbackStatus),
		}

		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if err := tmpl.ExecuteTemplate(w, "library.html", data); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/library", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		renderLibrary(w, r, libraryQueryFromValues(r.URL.Query()), "", "", "")
	})

	mux.HandleFunc("/library/commands", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		if err := r.ParseForm(); err != nil {
			renderLibrary(w, r, libraryQueryFromValues(r.Form), "", fmt.Sprintf("parse command form: %v", err), "")
			return
		}

		request := libraryCommandRequest{
			Action:  r.Form.Get("action"),
			TrackID: r.Form.Get("track_id"),
			Query:   buildLibraryQueryView(libraryQueryFromValues(r.Form)),
		}
		query := libraryclient.Query{
			AlbumUID:            request.Query.AlbumUID,
			DirectoryVolumeUUID: request.Query.DirectoryVolumeUUID,
			DirectoryPath:       request.Query.DirectoryPath,
		}
		message, err := executeLibraryCommand(
			r.Context(),
			deps.Playback,
			deps.Library,
			request.Action,
			request.TrackID,
			query,
		)
		if err != nil {
			renderLibrary(w, r, query, "", err.Error(), "")
			return
		}

		renderLibrary(w, r, query, message, "", "")
	})

	mux.HandleFunc("/library/media/commands", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		if err := r.ParseForm(); err != nil {
			renderLibrary(w, r, libraryQueryFromValues(r.Form), "", fmt.Sprintf("parse media command form: %v", err), "")
			return
		}

		request := normalizeLibraryMediaCommandRequest(libraryMediaCommandRequest{
			Action:     r.Form.Get("action"),
			DevicePath: r.Form.Get("device_path"),
			ScanPath:   r.Form.Get("scan_path"),
		})
		query := libraryQueryFromValues(r.Form)
		result, err := executeLibraryMediaCommand(r.Context(), mediaImport, deps.Playback, request)
		if err != nil {
			renderLibrary(w, r, query, "", err.Error(), result.Output)
			return
		}

		renderLibrary(w, r, query, result.Message, "", result.Output)
	})

	renderLogs := func(w http.ResponseWriter, r *http.Request) {
		lines := parseLogLines(r)
		logText, err := logs.Recent(r.Context(), lines)
		data := logsViewData{
			InlineCSS:   appCSS,
			CurrentPage: "logs",
			Lines:       lines,
			LogText:     logText,
			LogTextPath: fmt.Sprintf("/logs.txt?lines=%d", lines),
		}
		if err != nil {
			data.LogError = err.Error()
		}

		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if err := tmpl.ExecuteTemplate(w, "logs.html", data); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/logs", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		renderLogs(w, r)
	})

	mux.HandleFunc("/logs.txt", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		lines := parseLogLines(r)
		logText, err := logs.Recent(r.Context(), lines)
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		if err != nil {
			_, _ = fmt.Fprintf(w, "log read error: %v\n\n", err)
		}
		_, _ = fmt.Fprint(w, logText)
	})

	renderProvisioning := func(w http.ResponseWriter, r *http.Request) {
		snapshot := provisioning.Snapshot(r.Context())
		audioSnapshot := audioOutput.Snapshot(r.Context())
		rawJSON, err := json.MarshalIndent(snapshot, "", "  ")
		if err != nil {
			rawJSON = []byte(fmt.Sprintf("{\"read_error\":%q}", err.Error()))
		}

		data := provisioningViewData{
			InlineCSS:    appCSS,
			CurrentPage:  "provisioning",
			Provisioning: snapshot,
			AudioOutput:  audioSnapshot,
			RawJSON:      string(rawJSON),
		}

		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if err := tmpl.ExecuteTemplate(w, "provisioning.html", data); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/provisioning", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		renderProvisioning(w, r)
	})

	mux.HandleFunc("/provisioning-status", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		if err := writeJSON(w, provisioning.Snapshot(r.Context())); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/provisioning/status", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		if err := writeJSON(w, provisioning.Snapshot(r.Context())); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/playback/status", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		if err := writeJSON(w, buildPlaybackStatusView(deps.Playback.Status(r.Context()))); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/playback/queue", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		if err := writeJSON(w, buildPlaybackQueueView(deps.Playback.QueueSnapshot(r.Context()))); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/playback/history", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		if err := writeJSON(w, buildPlaybackHistoryView(deps.Playback.HistorySnapshot(r.Context()))); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/library/snapshot", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		query := libraryQueryFromValues(r.URL.Query())
		if err := writeJSON(w, buildLibrarySnapshotView(deps.Library.QuerySnapshot(r.Context(), query))); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/library/media", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		if err := writeJSON(w, mediaImport.Snapshot(r.Context())); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/commands", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		if err := r.ParseForm(); err != nil {
			renderHome(w, r, "", fmt.Sprintf("parse command form: %v", err))
			return
		}

		request := playbackCommandRequest{
			Action:  r.Form.Get("action"),
			TrackID: r.Form.Get("track_id"),
		}
		message, err := executePlaybackCommand(r.Context(), deps.Playback, request)
		if err != nil {
			renderHome(w, r, "", err.Error())
			return
		}

		renderHome(w, r, message, "")
	})

	mux.HandleFunc("/api/v1/playback/commands", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		request, err := decodePlaybackCommandRequest(r)
		if err != nil {
			response := buildPlaybackCommandResponse(r.Context(), deps.Playback, playbackCommandRequest{}, "", err)
			if writeErr := writeJSONStatus(w, http.StatusBadRequest, response); writeErr != nil {
				http.Error(w, writeErr.Error(), http.StatusInternalServerError)
			}
			return
		}

		message, commandErr := executePlaybackCommand(r.Context(), deps.Playback, request)
		response := buildPlaybackCommandResponse(r.Context(), deps.Playback, request, message, commandErr)
		status := http.StatusOK
		if commandErr != nil {
			status = http.StatusBadRequest
		}
		if err := writeJSONStatus(w, status, response); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/library/commands", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		request, err := decodeLibraryCommandRequest(r)
		if err != nil {
			response := buildLibraryCommandResponse(r.Context(), deps, libraryCommandRequest{}, "", err)
			if writeErr := writeJSONStatus(w, http.StatusBadRequest, response); writeErr != nil {
				http.Error(w, writeErr.Error(), http.StatusInternalServerError)
			}
			return
		}

		message, commandErr := executeLibraryCommand(
			r.Context(),
			deps.Playback,
			deps.Library,
			request.Action,
			request.TrackID,
			libraryclient.Query{
				AlbumUID:            request.Query.AlbumUID,
				DirectoryVolumeUUID: request.Query.DirectoryVolumeUUID,
				DirectoryPath:       request.Query.DirectoryPath,
			},
		)
		response := buildLibraryCommandResponse(r.Context(), deps, request, message, commandErr)
		status := http.StatusOK
		if commandErr != nil {
			status = http.StatusBadRequest
		}
		if err := writeJSONStatus(w, status, response); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/api/v1/library/media/commands", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		request, err := decodeLibraryMediaCommandRequest(r)
		if err != nil {
			response := buildLibraryMediaCommandResponse(r.Context(), deps, mediaImport, libraryMediaCommandRequest{}, mediaimport.CommandResult{}, err)
			if writeErr := writeJSONStatus(w, http.StatusBadRequest, response); writeErr != nil {
				http.Error(w, writeErr.Error(), http.StatusInternalServerError)
			}
			return
		}

		result, commandErr := executeLibraryMediaCommand(r.Context(), mediaImport, deps.Playback, request)
		response := buildLibraryMediaCommandResponse(r.Context(), deps, mediaImport, request, result, commandErr)
		status := http.StatusOK
		if commandErr != nil {
			status = http.StatusBadRequest
		}
		if err := writeJSONStatus(w, status, response); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	handlePlaybackEvents := func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		flusher, ok := w.(http.Flusher)
		if !ok {
			http.Error(w, "streaming unsupported", http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "text/event-stream")
		w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
		w.Header().Set("Connection", "keep-alive")
		w.Header().Set("X-Accel-Buffering", "no")

		_, _ = fmt.Fprint(w, ": lumelo playback stream\n\n")
		flusher.Flush()

		ctx := r.Context()
		eventCh := make(chan playbackclient.Event, 8)
		errCh := make(chan error, 1)

		go func() {
			errCh <- deps.Playback.SubscribeEvents(ctx, func(event playbackclient.Event) error {
				select {
				case eventCh <- event:
					return nil
				case <-ctx.Done():
					return ctx.Err()
				}
			})
		}()

		keepAlive := time.NewTicker(20 * time.Second)
		defer keepAlive.Stop()

		for {
			select {
			case <-ctx.Done():
				return
			case event := <-eventCh:
				payload, err := json.Marshal(event)
				if err != nil {
					http.Error(w, fmt.Sprintf("marshal event: %v", err), http.StatusInternalServerError)
					return
				}
				if _, err := fmt.Fprintf(w, "event: %s\ndata: %s\n\n", event.Name, payload); err != nil {
					return
				}
				flusher.Flush()
			case err := <-errCh:
				if err == nil || errors.Is(err, context.Canceled) {
					return
				}
				payload, _ := json.Marshal(map[string]string{"message": err.Error()})
				if _, writeErr := fmt.Fprintf(w, "event: STREAM_ERROR\ndata: %s\n\n", payload); writeErr != nil {
					return
				}
				flusher.Flush()
				return
			case <-keepAlive.C:
				if _, err := fmt.Fprint(w, ": keepalive\n\n"); err != nil {
					return
				}
				flusher.Flush()
			}
		}
	}

	mux.HandleFunc("/events/playback", handlePlaybackEvents)
	mux.HandleFunc("/api/v1/playback/events", handlePlaybackEvents)

	return &Server{handler: mux}, nil
}

func (s *Server) Handler() http.Handler {
	return s.handler
}

func decodePlaybackCommandRequest(r *http.Request) (playbackCommandRequest, error) {
	var request playbackCommandRequest
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&request); err != nil {
		return request, fmt.Errorf("decode playback command request: %w", err)
	}
	request.Action = strings.TrimSpace(request.Action)
	request.TrackID = strings.TrimSpace(request.TrackID)
	if request.Action == "" {
		return request, fmt.Errorf("action is required")
	}

	return request, nil
}

func decodeLibraryCommandRequest(r *http.Request) (libraryCommandRequest, error) {
	var request libraryCommandRequest
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&request); err != nil {
		return request, fmt.Errorf("decode library command request: %w", err)
	}
	request.Action = strings.TrimSpace(request.Action)
	request.TrackID = strings.TrimSpace(request.TrackID)
	request.Query.AlbumUID = strings.TrimSpace(request.Query.AlbumUID)
	request.Query.DirectoryVolumeUUID = strings.TrimSpace(request.Query.DirectoryVolumeUUID)
	request.Query.DirectoryPath = strings.TrimSpace(request.Query.DirectoryPath)
	if request.Action == "" {
		return request, fmt.Errorf("action is required")
	}

	return request, nil
}

func decodeLibraryMediaCommandRequest(r *http.Request) (libraryMediaCommandRequest, error) {
	var request libraryMediaCommandRequest
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&request); err != nil {
		return request, fmt.Errorf("decode library media command request: %w", err)
	}
	request = normalizeLibraryMediaCommandRequest(request)
	if request.Action == "" {
		return request, fmt.Errorf("action is required")
	}

	return request, nil
}

func normalizeLibraryMediaCommandRequest(request libraryMediaCommandRequest) libraryMediaCommandRequest {
	return libraryMediaCommandRequest{
		Action:     strings.ToLower(strings.TrimSpace(request.Action)),
		DevicePath: strings.TrimSpace(request.DevicePath),
		ScanPath:   strings.TrimSpace(request.ScanPath),
	}
}

func executePlaybackCommand(
	ctx context.Context,
	playback *playbackclient.Client,
	request playbackCommandRequest,
) (string, error) {
	if err := validateRemotePlaybackTarget(request.Action, request.TrackID); err != nil {
		return "", err
	}

	return playback.Execute(ctx, request.Action, request.TrackID)
}

func executeLibraryMediaCommand(
	ctx context.Context,
	mediaImport MediaImportSource,
	playback *playbackclient.Client,
	request libraryMediaCommandRequest,
) (mediaimport.CommandResult, error) {
	request = normalizeLibraryMediaCommandRequest(request)
	result := mediaimport.CommandResult{Action: request.Action}
	if request.Action == "" {
		return result, fmt.Errorf("action is required")
	}

	if mediaCommandRequiresScan(request.Action) {
		status := playback.Status(ctx)
		if playbackBlocksScan(status) {
			return result, fmt.Errorf("playback_quiet_mode_active: stop playback before scanning media")
		}
	}

	return mediaImport.Execute(ctx, mediaimport.CommandRequest{
		Action:     request.Action,
		DevicePath: request.DevicePath,
		ScanPath:   request.ScanPath,
	})
}

func mediaCommandRequiresScan(action string) bool {
	switch strings.ToLower(strings.TrimSpace(action)) {
	case "scan_device", "scan_mounted", "scan_path":
		return true
	default:
		return false
	}
}

func buildPlaybackCommandResponse(
	ctx context.Context,
	playback *playbackclient.Client,
	request playbackCommandRequest,
	message string,
	commandErr error,
) playbackCommandResponse {
	response := playbackCommandResponse{
		OK:             commandErr == nil,
		Action:         request.Action,
		TrackID:        request.TrackID,
		Message:        message,
		PlaybackStatus: buildPlaybackStatusView(playback.Status(ctx)),
		Queue:          buildPlaybackQueueView(playback.QueueSnapshot(ctx)),
	}
	if commandErr != nil {
		response.Error = commandErr.Error()
	}

	return response
}

func buildLibraryCommandResponse(
	ctx context.Context,
	deps Dependencies,
	request libraryCommandRequest,
	message string,
	commandErr error,
) libraryCommandResponse {
	response := libraryCommandResponse{
		OK:             commandErr == nil,
		Action:         request.Action,
		TrackID:        request.TrackID,
		Query:          request.Query,
		Message:        message,
		PlaybackStatus: buildPlaybackStatusView(deps.Playback.Status(ctx)),
		Queue:          buildPlaybackQueueView(deps.Playback.QueueSnapshot(ctx)),
	}
	if commandErr != nil {
		response.Error = commandErr.Error()
	}

	return response
}

func buildLibraryMediaCommandResponse(
	ctx context.Context,
	deps Dependencies,
	mediaImport MediaImportSource,
	request libraryMediaCommandRequest,
	result mediaimport.CommandResult,
	commandErr error,
) libraryMediaCommandResponse {
	playbackStatus := deps.Playback.Status(ctx)
	librarySnapshot := deps.Library.Snapshot(ctx)
	response := libraryMediaCommandResponse{
		OK:                  commandErr == nil,
		Action:              request.Action,
		DevicePath:          request.DevicePath,
		ScanPath:            request.ScanPath,
		Message:             result.Message,
		Output:              result.Output,
		PlaybackScanBlocked: playbackBlocksScan(playbackStatus),
		PlaybackStatus:      buildPlaybackStatusView(playbackStatus),
		Media:               mediaImport.Snapshot(ctx),
		Library:             buildLibrarySnapshotView(librarySnapshot),
	}
	if commandErr != nil {
		response.Error = commandErr.Error()
	}

	return response
}

func suggestedTrackID(status playbackclient.Status, snapshot libraryclient.Snapshot) string {
	if status.CurrentTrack != "" {
		return status.CurrentTrack
	}
	if len(snapshot.Tracks) > 0 && snapshot.Tracks[0].TrackUID != "" {
		return snapshot.Tracks[0].TrackUID
	}

	return "demo-track-001"
}

func playbackBlocksScan(status playbackclient.Status) bool {
	return status.State == "pre_quiet" || status.State == "quiet_active"
}

func parseLogLines(r *http.Request) int {
	raw := r.URL.Query().Get("lines")
	if raw == "" {
		return defaultLogLines
	}

	lines, err := strconv.Atoi(raw)
	if err != nil {
		return defaultLogLines
	}
	if lines < 50 {
		return 50
	}
	if lines > 1000 {
		return 1000
	}

	return lines
}

type unavailableLogSource struct{}

func (unavailableLogSource) Recent(context.Context, int) (string, error) {
	return "", errors.New("log source is not configured")
}

type unavailableProvisioningSource struct{}

func (unavailableProvisioningSource) Snapshot(context.Context) provisioningclient.Snapshot {
	return provisioningclient.Snapshot{
		ReadError: "provisioning source is not configured",
	}
}

type unavailableMediaImportSource struct{}

func (unavailableMediaImportSource) Snapshot(context.Context) mediaimport.Snapshot {
	return mediaimport.Snapshot{
		Error: "media import source is not configured",
	}
}

func (unavailableMediaImportSource) Execute(_ context.Context, request mediaimport.CommandRequest) (mediaimport.CommandResult, error) {
	return mediaimport.CommandResult{Action: strings.ToLower(strings.TrimSpace(request.Action))}, errors.New("media import source is not configured")
}

func buildQueueEntryViews(snapshot playbackclient.QueueSnapshot) []queueEntryView {
	views := make([]queueEntryView, 0, len(snapshot.Entries))
	for _, entry := range snapshot.Entries {
		title := entry.TrackUID
		if entry.Title != nil && *entry.Title != "" {
			title = *entry.Title
		}
		if title == "" {
			title = entry.TrackUID
		}

		views = append(views, queueEntryView{
			DisplayIndex: fmt.Sprintf("%02d", entry.OrderIndex+1),
			QueueEntryID: entry.QueueEntryID,
			TrackUID:     entry.TrackUID,
			RelativePath: entry.RelativePath,
			Title:        title,
			IsCurrent:    entry.IsCurrent,
		})
	}

	return views
}

func buildHistoryEntryViews(snapshot playbackclient.HistorySnapshot, status playbackclient.Status, librarySnapshot libraryclient.Snapshot) []historyEntryView {
	views := make([]historyEntryView, 0, len(snapshot.Entries))
	currentTrack := strings.TrimSpace(status.CurrentTrack)
	showCurrent := currentTrack != "" && playbackIsActive(status.State)
	libraryTracks := make(map[string]libraryclient.TrackSummary, len(librarySnapshot.Tracks))
	for _, track := range librarySnapshot.Tracks {
		if track.TrackUID != "" {
			libraryTracks[track.TrackUID] = track
		}
	}

	currentMarked := false
	for _, entry := range snapshot.Entries {
		title, relativePath := historyEntryDisplay(entry, libraryTracks)
		isCurrent := false
		if showCurrent && !currentMarked && entry.TrackUID == currentTrack {
			isCurrent = true
			currentMarked = true
		}

		views = append(views, historyEntryView{
			DisplayIndex: fmt.Sprintf("%02d", len(views)+1),
			TrackUID:     entry.TrackUID,
			RelativePath: relativePath,
			Title:        title,
			IsCurrent:    isCurrent,
		})
	}

	return views
}

func historyEntryDisplay(entry playbackclient.HistoryEntry, libraryTracks map[string]libraryclient.TrackSummary) (string, string) {
	title := ""
	relativePath := entry.RelativePath

	if track, ok := libraryTracks[entry.TrackUID]; ok {
		title = strings.TrimSpace(track.Title)
		if path := strings.TrimSpace(track.RelativePath); path != "" {
			relativePath = path
		}
	}

	if title == "" && entry.Title != nil {
		title = strings.TrimSpace(*entry.Title)
	}
	if title == "" || title == entry.TrackUID {
		if pathTitle := titleFromRelativePath(relativePath); pathTitle != "" && pathTitle != entry.TrackUID {
			title = pathTitle
		}
	}
	if title == "" {
		title = entry.TrackUID
	}

	return title, relativePath
}

func titleFromRelativePath(relativePath string) string {
	trimmed := strings.TrimSpace(relativePath)
	if trimmed == "" {
		return ""
	}
	if slash := strings.LastIndex(trimmed, "/"); slash >= 0 {
		trimmed = trimmed[slash+1:]
	}
	for _, suffix := range []string{".flac", ".wav", ".mp3", ".m4a", ".aac", ".ogg", ".dsf", ".dff"} {
		if strings.HasSuffix(strings.ToLower(trimmed), suffix) {
			return strings.TrimSpace(trimmed[:len(trimmed)-len(suffix)])
		}
	}
	return trimmed
}

func playbackIsActive(state string) bool {
	switch state {
	case "playing", "quiet_active":
		return true
	default:
		return false
	}
}

func currentOrderLabel(snapshot playbackclient.QueueSnapshot) string {
	if snapshot.CurrentOrderIndex == nil {
		return "-"
	}

	return fmt.Sprintf("%d", *snapshot.CurrentOrderIndex)
}

func buildLibraryVolumeViews(snapshot libraryclient.Snapshot) []libraryVolumeView {
	views := make([]libraryVolumeView, 0, len(snapshot.Volumes))
	for _, volume := range snapshot.Volumes {
		query := url.Values{}
		query.Set("volume_uuid", volume.VolumeUUID)
		views = append(views, libraryVolumeView{
			Label:       volume.Label,
			MountPath:   volume.MountPath,
			VolumeUUID:  volume.VolumeUUID,
			LastSeenAt:  fmt.Sprintf("%d", volume.LastSeenAt),
			IsAvailable: volume.IsAvailable,
			BrowsePath:  "/library?" + query.Encode(),
		})
	}

	return views
}

func buildLibraryDirectoryViews(snapshot libraryclient.Snapshot, query libraryclient.Query) []libraryDirectoryView {
	views := make([]libraryDirectoryView, 0, len(snapshot.Directories))
	for _, directory := range snapshot.Directories {
		nextQuery := url.Values{}
		nextQuery.Set("volume_uuid", directory.VolumeUUID)
		nextQuery.Set("directory", directory.RelativePath)
		views = append(views, libraryDirectoryView{
			DisplayName:  directory.DisplayName,
			RelativePath: directory.RelativePath,
			BrowsePath:   "/library?" + nextQuery.Encode(),
			IsSelected:   query.DirectoryVolumeUUID == directory.VolumeUUID && query.DirectoryPath == directory.RelativePath,
		})
	}

	return views
}

func buildLibraryAlbumViews(snapshot libraryclient.Snapshot, selectedAlbumUID string) []libraryAlbumView {
	views := make([]libraryAlbumView, 0, len(snapshot.Albums)+1)
	uncategorizedTrackCount := 0
	for _, album := range snapshot.Albums {
		if album.SourceMode == "directory_fallback" && album.TrackCount == 1 {
			uncategorizedTrackCount += album.TrackCount
			continue
		}
		coverThumbLabel := fallback(album.CoverThumbRelPath, "-")
		coverThumbPath := ""
		if album.CoverThumbRelPath != "" {
			coverThumbPath = "/artwork/" + album.CoverThumbRelPath
		}
		query := url.Values{}
		query.Set("album_uid", album.AlbumUID)
		views = append(views, libraryAlbumView{
			AlbumUID:        album.AlbumUID,
			Title:           album.Title,
			AlbumArtist:     album.AlbumArtist,
			YearLabel:       intLabel(album.Year),
			TrackCount:      album.TrackCount,
			DurationLabel:   durationMSLabel(album.TotalDurationMS),
			RootDirHint:     fallback(album.RootDirHint, "-"),
			CoverThumbLabel: coverThumbLabel,
			CoverThumbPath:  coverThumbPath,
			BrowsePath:      "/library?" + query.Encode(),
			IsSelected:      selectedAlbumUID != "" && selectedAlbumUID == album.AlbumUID,
		})
	}
	if uncategorizedTrackCount > 0 {
		query := url.Values{}
		query.Set("album_uid", libraryclient.UncategorizedAlbumUID)
		views = append(views, libraryAlbumView{
			AlbumUID:        libraryclient.UncategorizedAlbumUID,
			Title:           "未分类",
			AlbumArtist:     "抓不到专辑信息的散曲",
			YearLabel:       "-",
			TrackCount:      uncategorizedTrackCount,
			DurationLabel:   "-",
			RootDirHint:     "混合文件夹 / 缺少专辑元数据",
			CoverThumbLabel: "-",
			BrowsePath:      "/library?" + query.Encode(),
			IsSelected:      selectedAlbumUID == libraryclient.UncategorizedAlbumUID,
			IsSynthetic:     true,
		})
	}

	return views
}

func buildHomeFeaturedAlbums(snapshot libraryclient.Snapshot, limit int) []libraryAlbumView {
	albums := buildLibraryAlbumViews(snapshot, "")
	if limit <= 0 || len(albums) <= limit {
		return albums
	}
	return albums[:limit]
}

func buildLibraryTrackViews(snapshot libraryclient.Snapshot, playbackStatus playbackclient.Status) []libraryTrackView {
	views := make([]libraryTrackView, 0, len(snapshot.Tracks))
	for _, track := range snapshot.Tracks {
		canPlay, supportLabel := trackPlaybackSupport(track)
		views = append(views, libraryTrackView{
			TrackNumberLabel: trackOrdinalLabel(track),
			TrackUID:         track.TrackUID,
			AlbumUID:         track.AlbumUID,
			AlbumTitle:       track.AlbumTitle,
			Title:            track.Title,
			Artist:           track.Artist,
			RelativePath:     track.RelativePath,
			FormatLabel:      formatTrackFormat(track),
			DurationLabel:    pointerDurationMSLabel(track.DurationMS),
			IsCurrent:        playbackStatus.CurrentTrack != "" && playbackStatus.CurrentTrack == track.TrackUID,
			CanPlay:          canPlay,
			SupportLabel:     supportLabel,
		})
	}

	return views
}

func trackOrdinalLabel(track libraryclient.TrackSummary) string {
	if track.TrackNo != nil && *track.TrackNo > 0 {
		if track.DiscNo != nil && *track.DiscNo > 1 {
			return fmt.Sprintf("%d-%02d", *track.DiscNo, *track.TrackNo)
		}
		return fmt.Sprintf("%02d", *track.TrackNo)
	}
	if track.DiscNo != nil && *track.DiscNo > 1 {
		return fmt.Sprintf("%d", *track.DiscNo)
	}
	return "•"
}

func buildLibraryNowPlayingView(
	playbackStatus playbackclient.Status,
	queueSnapshot playbackclient.QueueSnapshot,
	snapshot libraryclient.Snapshot,
) libraryNowPlayingView {
	view := libraryNowPlayingView{
		State:    playbackStatus.State,
		TrackUID: playbackStatus.CurrentTrack,
	}
	if playbackStatus.CurrentTrack == "" {
		return view
	}

	for _, track := range snapshot.Tracks {
		if track.TrackUID != playbackStatus.CurrentTrack {
			continue
		}
		view.Known = true
		view.Title = fallback(track.Title, track.TrackUID)
		view.Artist = track.Artist
		view.AlbumTitle = track.AlbumTitle
		view.RelativePath = track.RelativePath
		view.AudioFormatLabel = audioOutputFormatLabel(track)
		for _, album := range snapshot.Albums {
			if album.AlbumUID != track.AlbumUID || album.CoverThumbRelPath == "" {
				continue
			}
			view.CoverThumbPath = "/artwork/" + album.CoverThumbRelPath
			break
		}
		break
	}

	for _, entry := range queueSnapshot.Entries {
		if entry.TrackUID != playbackStatus.CurrentTrack {
			continue
		}
		view.Known = true
		if entry.Title != nil && *entry.Title != "" && view.Title == "" {
			view.Title = *entry.Title
		}
		if view.RelativePath == "" {
			view.RelativePath = entry.RelativePath
		}
		return view
	}

	return view
}

func durationMSLabel(durationMS int64) string {
	if durationMS <= 0 {
		return "-"
	}

	totalSeconds := durationMS / 1000
	minutes := totalSeconds / 60
	seconds := totalSeconds % 60
	return fmt.Sprintf("%d:%02d", minutes, seconds)
}

func pointerDurationMSLabel(durationMS *int64) string {
	if durationMS == nil {
		return "-"
	}

	return durationMSLabel(*durationMS)
}

func formatTrackFormat(track libraryclient.TrackSummary) string {
	if track.Format == "" && track.SampleRate == nil {
		return "-"
	}
	if track.SampleRate == nil {
		return track.Format
	}
	if track.Format == "" {
		return fmt.Sprintf("%d Hz", *track.SampleRate)
	}

	return fmt.Sprintf("%s · %d Hz", track.Format, *track.SampleRate)
}

func audioOutputFormatLabel(track libraryclient.TrackSummary) string {
	format := strings.ToLower(strings.TrimSpace(track.Format))
	formatLabel := strings.ToUpper(format)
	if formatLabel == "" {
		formatLabel = "UNKNOWN"
	}

	if format == "dsf" || format == "dff" || format == "dsd" {
		if track.SampleRate == nil || *track.SampleRate <= 0 {
			return "DSD · " + formatLabel
		}
		return fmt.Sprintf("%s · %s · %s", dsdRateLabel(*track.SampleRate), formatLabel, sampleRateLabel(*track.SampleRate))
	}

	if track.SampleRate == nil || *track.SampleRate <= 0 {
		return "PCM · " + formatLabel
	}

	return fmt.Sprintf("PCM · %s · %s", formatLabel, sampleRateLabel(*track.SampleRate))
}

func sampleRateLabel(sampleRate int64) string {
	if sampleRate >= 1_000_000 {
		value := float64(sampleRate) / 1_000_000
		return strings.TrimRight(strings.TrimRight(fmt.Sprintf("%.4f", value), "0"), ".") + " MHz"
	}
	value := float64(sampleRate) / 1_000
	return strings.TrimRight(strings.TrimRight(fmt.Sprintf("%.1f", value), "0"), ".") + " kHz"
}

func dsdRateLabel(sampleRate int64) string {
	switch sampleRate {
	case 2_822_400:
		return "DSD64"
	case 5_644_800:
		return "DSD128"
	case 11_289_600:
		return "DSD256"
	case 22_579_200:
		return "DSD512"
	default:
		return "DSD"
	}
}

func intLabel(value int) string {
	if value <= 0 {
		return "-"
	}

	return fmt.Sprintf("%d", value)
}

func fallback(value, fallbackValue string) string {
	if value == "" {
		return fallbackValue
	}

	return value
}

func libraryQueryFromValues(values url.Values) libraryclient.Query {
	albumUID := strings.TrimSpace(values.Get("album_uid"))
	return libraryclient.Query{
		AlbumUID:            albumUID,
		DirectoryVolumeUUID: strings.TrimSpace(values.Get("volume_uuid")),
		DirectoryPath:       strings.TrimSpace(values.Get("directory")),
	}
}

func selectedAlbumTitle(snapshot libraryclient.Snapshot, albumUID string) string {
	if albumUID == "" {
		return ""
	}
	if albumUID == libraryclient.UncategorizedAlbumUID {
		return "未分类"
	}

	for _, album := range snapshot.Albums {
		if album.AlbumUID == albumUID {
			return album.Title
		}
	}

	return ""
}

func selectedDirectoryTitle(query libraryclient.Query) string {
	if query.DirectoryVolumeUUID == "" {
		return ""
	}
	if query.DirectoryPath == "" {
		return "存储根目录"
	}

	parts := strings.Split(query.DirectoryPath, "/")
	return parts[len(parts)-1]
}

func parentDirectoryBrowsePath(query libraryclient.Query) string {
	if query.DirectoryVolumeUUID == "" {
		return ""
	}

	values := url.Values{}
	values.Set("volume_uuid", query.DirectoryVolumeUUID)
	if query.DirectoryPath == "" {
		return "/library?" + values.Encode()
	}

	parent := query.DirectoryPath
	if index := strings.LastIndex(parent, "/"); index >= 0 {
		parent = parent[:index]
	} else {
		parent = ""
	}
	if parent != "" {
		values.Set("directory", parent)
	}
	return "/library?" + values.Encode()
}

func executeLibraryCommand(
	ctx context.Context,
	playback *playbackclient.Client,
	library *libraryclient.Client,
	action string,
	trackID string,
	query libraryclient.Query,
) (string, error) {
	if err := validateRemotePlaybackTarget(action, trackID); err != nil {
		return "", err
	}

	if strings.EqualFold(strings.TrimSpace(action), "play") {
		status := playback.Status(ctx)
		if status.Available && status.State == "paused" && status.CurrentTrack == strings.TrimSpace(trackID) {
			return playback.Execute(ctx, action, trackID)
		}

		snapshot := library.QuerySnapshot(ctx, query)
		if !snapshot.Available || snapshot.Error != "" {
			return "", fmt.Errorf("library snapshot unavailable: %s", fallback(snapshot.Error, "unknown_error"))
		}
		trackIDs, err := trackIDsFromPlaybackContext(snapshot, trackID)
		if err != nil {
			return "", err
		}
		return playback.PlayQueue(ctx, trackIDs)
	}

	return playback.Execute(ctx, action, trackID)
}

func validateRemotePlaybackTarget(action string, trackID string) error {
	action = strings.ToLower(strings.TrimSpace(action))
	trackID = strings.TrimSpace(trackID)

	switch action {
	case "play", "play_history", "queue_append", "queue_insert_next":
		return rejectAbsolutePlaybackTarget(trackID)
	case "queue_play", "queue_replace":
		return validateRemoteTrackIDList(trackID)
	default:
		return nil
	}
}

func validateRemoteTrackIDList(raw string) error {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return nil
	}
	if err := rejectAbsolutePlaybackTarget(raw); err != nil {
		return err
	}

	var trackIDs []string
	if err := json.Unmarshal([]byte(raw), &trackIDs); err != nil {
		return fmt.Errorf("invalid track id list: %w", err)
	}
	for _, trackID := range trackIDs {
		if err := rejectAbsolutePlaybackTarget(trackID); err != nil {
			return err
		}
	}

	return nil
}

func rejectAbsolutePlaybackTarget(trackID string) error {
	if isAbsolutePlaybackTarget(trackID) {
		return fmt.Errorf("absolute_path_playback_forbidden")
	}
	return nil
}

func isAbsolutePlaybackTarget(trackID string) bool {
	trackID = strings.TrimSpace(trackID)
	return trackID != "" && (filepath.IsAbs(trackID) || strings.HasPrefix(trackID, "/"))
}

func trackIDsFromPlaybackContext(snapshot libraryclient.Snapshot, startTrackID string) ([]string, error) {
	startTrackID = strings.TrimSpace(startTrackID)
	if startTrackID == "" {
		return nil, fmt.Errorf("track id is required for PLAY")
	}

	startIndex := -1
	for index, track := range snapshot.Tracks {
		if track.TrackUID == startTrackID {
			startIndex = index
			break
		}
	}
	if startIndex < 0 {
		return nil, fmt.Errorf("track_not_found_in_library_context")
	}
	if supported, _ := trackPlaybackSupport(snapshot.Tracks[startIndex]); !supported {
		return nil, fmt.Errorf("unsupported_format_in_library_ui")
	}

	trackIDs := make([]string, 0, len(snapshot.Tracks)-startIndex)
	for _, track := range snapshot.Tracks[startIndex:] {
		if supported, _ := trackPlaybackSupport(track); !supported {
			continue
		}
		if err := rejectAbsolutePlaybackTarget(track.TrackUID); err != nil {
			return nil, err
		}
		trackIDs = append(trackIDs, track.TrackUID)
	}
	if len(trackIDs) == 0 {
		return nil, fmt.Errorf("no_playable_tracks_in_context")
	}

	return trackIDs, nil
}

func trackPlaybackSupport(track libraryclient.TrackSummary) (bool, string) {
	switch strings.ToLower(strings.TrimSpace(track.Format)) {
	case "ape", "opus", "wma":
		return false, "unsupported"
	default:
		return true, ""
	}
}
