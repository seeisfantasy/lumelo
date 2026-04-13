package api

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"html/template"
	"io/fs"
	"net/http"
	"strconv"
	"time"

	"github.com/lumelo/controld/internal/auth"
	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/playbackclient"
	"github.com/lumelo/controld/internal/provisioningclient"
	"github.com/lumelo/controld/internal/settings"
	"github.com/lumelo/controld/internal/sshctl"
)

type Dependencies struct {
	Auth             *auth.Service
	Playback         *playbackclient.Client
	Library          *libraryclient.Client
	Logs             LogSource
	Provisioning     ProvisioningSource
	Settings         settings.Config
	SSH              *sshctl.Controller
	Templates        fs.FS
	Static           fs.FS
	ArtworkCacheRoot string
}

type LogSource interface {
	Recent(ctx context.Context, lines int) (string, error)
}

type ProvisioningSource interface {
	Snapshot(ctx context.Context) provisioningclient.Snapshot
}

type Server struct {
	handler http.Handler
}

type homeViewData struct {
	CurrentPage        string
	Mode               string
	InterfaceMode      string
	DSDPolicy          string
	PasswordConfigured bool
	SSHEnabled         bool
	CommandSocket      string
	EventSocket        string
	LibraryDBPath      string
	ConfigPath         string
	PlaybackStatus     playbackclient.Status
	QueueSnapshot      playbackclient.QueueSnapshot
	QueueEntries       []queueEntryView
	CurrentOrderLabel  string
	CommandMessage     string
	CommandError       string
	SuggestedTrackID   string
	PlaybackStreamPath string
	Provisioning       provisioningclient.Snapshot
}

type libraryViewData struct {
	CurrentPage       string
	LibraryDBPath     string
	LibrarySnapshot   libraryclient.Snapshot
	PlaybackStatus    playbackclient.Status
	PlaybackScanBlock bool
	VolumeEntries     []libraryVolumeView
	AlbumEntries      []libraryAlbumView
	TrackEntries      []libraryTrackView
}

type logsViewData struct {
	CurrentPage string
	Lines       int
	LogText     string
	LogError    string
	LogTextPath string
}

type provisioningViewData struct {
	CurrentPage  string
	Provisioning provisioningclient.Snapshot
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

type libraryVolumeView struct {
	Label       string
	MountPath   string
	VolumeUUID  string
	LastSeenAt  string
	IsAvailable bool
}

type libraryAlbumView struct {
	Title           string
	AlbumArtist     string
	YearLabel       string
	TrackCount      int
	DurationLabel   string
	RootDirHint     string
	CoverThumbLabel string
	CoverThumbPath  string
}

type libraryTrackView struct {
	Title         string
	Artist        string
	RelativePath  string
	FormatLabel   string
	DurationLabel string
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
	ProvisioningReadError string `json:"provisioning_read_error,omitempty"`
}

const defaultLogLines = 300

func New(deps Dependencies) (*Server, error) {
	tmpl, err := template.ParseFS(deps.Templates, "templates/*.html")
	if err != nil {
		return nil, fmt.Errorf("parse templates: %w", err)
	}

	staticFS, err := fs.Sub(deps.Static, "static")
	if err != nil {
		return nil, fmt.Errorf("load static assets: %w", err)
	}

	logs := deps.Logs
	if logs == nil {
		logs = unavailableLogSource{}
	}
	provisioning := deps.Provisioning
	if provisioning == nil {
		provisioning = unavailableProvisioningSource{}
	}

	mux := http.NewServeMux()
	mux.Handle("/static/", http.StripPrefix("/static/", http.FileServer(http.FS(staticFS))))
	if deps.ArtworkCacheRoot != "" {
		mux.Handle("/artwork/", http.StripPrefix("/artwork/", http.FileServer(http.Dir(deps.ArtworkCacheRoot))))
	}

	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		playbackStatus := deps.Playback.Status(r.Context())
		librarySnapshot := deps.Library.Snapshot(r.Context())
		provisioningSnapshot := provisioning.Snapshot(r.Context())
		response := healthView{
			Status:                "ok",
			Mode:                  deps.Settings.Mode,
			InterfaceMode:         deps.Settings.InterfaceMode,
			SSHEnabled:            deps.SSH.Enabled(),
			PlaybackAvailable:     playbackStatus.Available,
			PlaybackState:         playbackStatus.State,
			PlaybackError:         playbackStatus.Error,
			LibraryAvailable:      librarySnapshot.Available,
			LibraryDBPath:         deps.Library.LibraryDBPath,
			LibraryError:          librarySnapshot.Error,
			ProvisioningAvailable: provisioningSnapshot.Available,
			ProvisioningState:     provisioningSnapshot.State,
			ProvisioningMessage:   provisioningSnapshot.Message,
			ProvisioningReadError: provisioningSnapshot.ReadError,
		}

		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		if err := json.NewEncoder(w).Encode(response); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	renderHome := func(w http.ResponseWriter, r *http.Request, commandMessage, commandError string) {
		status := deps.Playback.Status(r.Context())
		queueSnapshot := deps.Playback.QueueSnapshot(r.Context())
		librarySnapshot := deps.Library.Snapshot(r.Context())
		provisioningSnapshot := provisioning.Snapshot(r.Context())

		data := homeViewData{
			CurrentPage:        "home",
			Mode:               deps.Settings.Mode,
			InterfaceMode:      deps.Settings.InterfaceMode,
			DSDPolicy:          deps.Settings.DSDPolicy,
			PasswordConfigured: deps.Auth.PasswordConfigured(),
			SSHEnabled:         deps.SSH.Enabled(),
			CommandSocket:      deps.Playback.CommandSocket,
			EventSocket:        deps.Playback.EventSocket,
			LibraryDBPath:      deps.Library.LibraryDBPath,
			ConfigPath:         deps.Settings.ConfigPath,
			PlaybackStatus:     status,
			QueueSnapshot:      queueSnapshot,
			QueueEntries:       buildQueueEntryViews(queueSnapshot),
			CurrentOrderLabel:  currentOrderLabel(queueSnapshot),
			CommandMessage:     commandMessage,
			CommandError:       commandError,
			SuggestedTrackID:   suggestedTrackID(status, librarySnapshot),
			PlaybackStreamPath: "/events/playback",
			Provisioning:       provisioningSnapshot,
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

	renderLibrary := func(w http.ResponseWriter, r *http.Request) {
		snapshot := deps.Library.Snapshot(r.Context())
		playbackStatus := deps.Playback.Status(r.Context())

		data := libraryViewData{
			CurrentPage:       "library",
			LibraryDBPath:     deps.Library.LibraryDBPath,
			LibrarySnapshot:   snapshot,
			PlaybackStatus:    playbackStatus,
			PlaybackScanBlock: playbackBlocksScan(playbackStatus),
			VolumeEntries:     buildLibraryVolumeViews(snapshot),
			AlbumEntries:      buildLibraryAlbumViews(snapshot),
			TrackEntries:      buildLibraryTrackViews(snapshot),
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

		renderLibrary(w, r)
	})

	renderLogs := func(w http.ResponseWriter, r *http.Request) {
		lines := parseLogLines(r)
		logText, err := logs.Recent(r.Context(), lines)
		data := logsViewData{
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
		rawJSON, err := json.MarshalIndent(snapshot, "", "  ")
		if err != nil {
			rawJSON = []byte(fmt.Sprintf("{\"read_error\":%q}", err.Error()))
		}

		data := provisioningViewData{
			CurrentPage:  "provisioning",
			Provisioning: snapshot,
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

		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		if err := json.NewEncoder(w).Encode(provisioning.Snapshot(r.Context())); err != nil {
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

		action := r.Form.Get("action")
		trackID := r.Form.Get("track_id")
		message, err := deps.Playback.Execute(r.Context(), action, trackID)
		if err != nil {
			renderHome(w, r, "", err.Error())
			return
		}

		renderHome(w, r, message, "")
	})

	mux.HandleFunc("/events/playback", func(w http.ResponseWriter, r *http.Request) {
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
	})

	return &Server{handler: mux}, nil
}

func (s *Server) Handler() http.Handler {
	return s.handler
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

func currentOrderLabel(snapshot playbackclient.QueueSnapshot) string {
	if snapshot.CurrentOrderIndex == nil {
		return "-"
	}

	return fmt.Sprintf("%d", *snapshot.CurrentOrderIndex)
}

func buildLibraryVolumeViews(snapshot libraryclient.Snapshot) []libraryVolumeView {
	views := make([]libraryVolumeView, 0, len(snapshot.Volumes))
	for _, volume := range snapshot.Volumes {
		views = append(views, libraryVolumeView{
			Label:       volume.Label,
			MountPath:   volume.MountPath,
			VolumeUUID:  volume.VolumeUUID,
			LastSeenAt:  fmt.Sprintf("%d", volume.LastSeenAt),
			IsAvailable: volume.IsAvailable,
		})
	}

	return views
}

func buildLibraryAlbumViews(snapshot libraryclient.Snapshot) []libraryAlbumView {
	views := make([]libraryAlbumView, 0, len(snapshot.Albums))
	for _, album := range snapshot.Albums {
		coverThumbLabel := fallback(album.CoverThumbRelPath, "-")
		coverThumbPath := ""
		if album.CoverThumbRelPath != "" {
			coverThumbPath = "/artwork/" + album.CoverThumbRelPath
		}
		views = append(views, libraryAlbumView{
			Title:           album.Title,
			AlbumArtist:     album.AlbumArtist,
			YearLabel:       intLabel(album.Year),
			TrackCount:      album.TrackCount,
			DurationLabel:   durationMSLabel(album.TotalDurationMS),
			RootDirHint:     fallback(album.RootDirHint, "-"),
			CoverThumbLabel: coverThumbLabel,
			CoverThumbPath:  coverThumbPath,
		})
	}

	return views
}

func buildLibraryTrackViews(snapshot libraryclient.Snapshot) []libraryTrackView {
	views := make([]libraryTrackView, 0, len(snapshot.Tracks))
	for _, track := range snapshot.Tracks {
		views = append(views, libraryTrackView{
			Title:         track.Title,
			Artist:        track.Artist,
			RelativePath:  track.RelativePath,
			FormatLabel:   formatTrackFormat(track),
			DurationLabel: pointerDurationMSLabel(track.DurationMS),
		})
	}

	return views
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
