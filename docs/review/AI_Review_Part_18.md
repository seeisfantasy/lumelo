# AI Review Part 18

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `services/controld/internal/api/server_test.go`

- bytes: 11105
- segment: 1/1

~~~go
package api_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
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
			WiFiIP:        "192.168.43.170",
			WiredIP:       "192.168.1.120",
			AllIPs:        []string{"192.168.1.120", "192.168.43.170"},
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
	if !strings.Contains(body, "Provisioning status") || !strings.Contains(body, "Studio WiFi") || !strings.Contains(body, "192.168.1.120") {
		t.Fatalf("expected provisioning page content, body: %s", body)
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
~~~

## `services/controld/internal/auth/auth.go`

- bytes: 315
- segment: 1/1

~~~go
package auth

// Service owns the single-admin authentication state for V1.
type Service struct {
	passwordConfigured bool
}

func NewService(passwordConfigured bool) *Service {
	return &Service{passwordConfigured: passwordConfigured}
}

func (s *Service) PasswordConfigured() bool {
	return s.passwordConfigured
}
~~~

## `services/controld/internal/libraryclient/client.go`

- bytes: 6137
- segment: 1/1

~~~go
package libraryclient

import (
	"context"
	"database/sql"
	"fmt"
	"net/url"

	_ "modernc.org/sqlite"
)

// Client is the control-plane view of the library/index layer.
type Client struct {
	LibraryDBPath string
}

type Snapshot struct {
	Available bool
	DBPath    string
	Error     string
	Stats     Stats
	Volumes   []VolumeSummary
	Albums    []AlbumSummary
	Tracks    []TrackSummary
}

type Stats struct {
	VolumeCount int
	AlbumCount  int
	TrackCount  int
	ArtistCount int
	GenreCount  int
}

type VolumeSummary struct {
	VolumeUUID  string
	Label       string
	MountPath   string
	IsAvailable bool
	LastSeenAt  int64
}

type AlbumSummary struct {
	AlbumUID          string
	Title             string
	AlbumArtist       string
	Year              int
	TrackCount        int
	TotalDurationMS   int64
	RootDirHint       string
	CoverThumbRelPath string
}

type TrackSummary struct {
	TrackUID     string
	Title        string
	Artist       string
	RelativePath string
	Format       string
	DurationMS   *int64
	SampleRate   *int64
}

func New(libraryDBPath string) *Client {
	return &Client{LibraryDBPath: libraryDBPath}
}

func (c *Client) Snapshot(ctx context.Context) Snapshot {
	snapshot := Snapshot{DBPath: c.LibraryDBPath}

	db, err := sql.Open("sqlite", sqliteReadOnlyDSN(c.LibraryDBPath))
	if err != nil {
		snapshot.Error = fmt.Sprintf("open library db: %v", err)
		return snapshot
	}
	defer db.Close()

	if err := db.PingContext(ctx); err != nil {
		snapshot.Error = fmt.Sprintf("ping library db: %v", err)
		return snapshot
	}

	snapshot.Available = true

	stats, err := queryStats(ctx, db)
	if err != nil {
		snapshot.Error = fmt.Sprintf("query library stats: %v", err)
		return snapshot
	}
	volumes, err := queryVolumes(ctx, db)
	if err != nil {
		snapshot.Error = fmt.Sprintf("query volumes: %v", err)
		return snapshot
	}
	albums, err := queryAlbums(ctx, db)
	if err != nil {
		snapshot.Error = fmt.Sprintf("query albums: %v", err)
		return snapshot
	}
	tracks, err := queryTracks(ctx, db)
	if err != nil {
		snapshot.Error = fmt.Sprintf("query tracks: %v", err)
		return snapshot
	}

	snapshot.Stats = stats
	snapshot.Volumes = volumes
	snapshot.Albums = albums
	snapshot.Tracks = tracks
	return snapshot
}

func sqliteReadOnlyDSN(path string) string {
	u := &url.URL{Scheme: "file", Path: path}
	query := url.Values{}
	query.Set("mode", "ro")
	u.RawQuery = query.Encode()
	return u.String()
}

func queryStats(ctx context.Context, db *sql.DB) (Stats, error) {
	volumeCount, err := queryCount(ctx, db, "SELECT COUNT(*) FROM volumes")
	if err != nil {
		return Stats{}, err
	}
	albumCount, err := queryCount(ctx, db, "SELECT COUNT(*) FROM albums")
	if err != nil {
		return Stats{}, err
	}
	trackCount, err := queryCount(ctx, db, "SELECT COUNT(*) FROM tracks")
	if err != nil {
		return Stats{}, err
	}
	artistCount, err := queryCount(ctx, db, "SELECT COUNT(*) FROM artists")
	if err != nil {
		return Stats{}, err
	}
	genreCount, err := queryCount(ctx, db, "SELECT COUNT(*) FROM genres")
	if err != nil {
		return Stats{}, err
	}

	return Stats{
		VolumeCount: volumeCount,
		AlbumCount:  albumCount,
		TrackCount:  trackCount,
		ArtistCount: artistCount,
		GenreCount:  genreCount,
	}, nil
}

func queryCount(ctx context.Context, db *sql.DB, query string) (int, error) {
	var count int
	if err := db.QueryRowContext(ctx, query).Scan(&count); err != nil {
		return 0, err
	}

	return count, nil
}

func queryVolumes(ctx context.Context, db *sql.DB) ([]VolumeSummary, error) {
	rows, err := db.QueryContext(ctx, `
		SELECT
			volume_uuid,
			COALESCE(NULLIF(label, ''), '(unlabeled volume)'),
			mount_path,
			is_available,
			last_seen_at
		FROM volumes
		ORDER BY last_seen_at DESC, mount_path ASC
		LIMIT 8
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var volumes []VolumeSummary
	for rows.Next() {
		var item VolumeSummary
		var isAvailable int
		if err := rows.Scan(&item.VolumeUUID, &item.Label, &item.MountPath, &isAvailable, &item.LastSeenAt); err != nil {
			return nil, err
		}
		item.IsAvailable = isAvailable != 0
		volumes = append(volumes, item)
	}

	return volumes, rows.Err()
}

func queryAlbums(ctx context.Context, db *sql.DB) ([]AlbumSummary, error) {
	rows, err := db.QueryContext(ctx, `
		SELECT
			albums.album_uid,
			albums.album_title,
			COALESCE(NULLIF(albums.album_artist, ''), '(unknown artist)'),
			COALESCE(albums.year, 0),
			albums.track_count,
			albums.total_duration_ms,
			COALESCE(albums.album_root_dir_hint, ''),
			COALESCE(artwork_refs.thumb_rel_path, '')
		FROM albums
		LEFT JOIN artwork_refs ON artwork_refs.artwork_ref_id = albums.cover_ref_id
		ORDER BY albums.indexed_at DESC, albums.album_title_norm ASC
		LIMIT 8
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var albums []AlbumSummary
	for rows.Next() {
		var item AlbumSummary
		if err := rows.Scan(
			&item.AlbumUID,
			&item.Title,
			&item.AlbumArtist,
			&item.Year,
			&item.TrackCount,
			&item.TotalDurationMS,
			&item.RootDirHint,
			&item.CoverThumbRelPath,
		); err != nil {
			return nil, err
		}
		albums = append(albums, item)
	}

	return albums, rows.Err()
}

func queryTracks(ctx context.Context, db *sql.DB) ([]TrackSummary, error) {
	rows, err := db.QueryContext(ctx, `
		SELECT
			track_uid,
			COALESCE(NULLIF(title, ''), filename),
			COALESCE(NULLIF(artist, ''), NULLIF(album_artist, ''), '(unknown artist)'),
			relative_path,
			COALESCE(format, ''),
			duration_ms,
			sample_rate
		FROM tracks
		ORDER BY indexed_at DESC, track_uid ASC
		LIMIT 12
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var tracks []TrackSummary
	for rows.Next() {
		var item TrackSummary
		var duration sql.NullInt64
		var sampleRate sql.NullInt64
		if err := rows.Scan(
			&item.TrackUID,
			&item.Title,
			&item.Artist,
			&item.RelativePath,
			&item.Format,
			&duration,
			&sampleRate,
		); err != nil {
			return nil, err
		}
		if duration.Valid {
			value := duration.Int64
			item.DurationMS = &value
		}
		if sampleRate.Valid {
			value := sampleRate.Int64
			item.SampleRate = &value
		}
		tracks = append(tracks, item)
	}

	return tracks, rows.Err()
}
~~~

## `services/controld/internal/libraryclient/client_test.go`

- bytes: 4062
- segment: 1/1

~~~go
package libraryclient

import (
	"context"
	"database/sql"
	"path/filepath"
	"testing"

	_ "modernc.org/sqlite"
)

func TestSnapshotReadsLibraryOverview(t *testing.T) {
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
		CREATE TABLE artwork_refs (
			artwork_ref_id INTEGER PRIMARY KEY,
			thumb_rel_path TEXT
		);
		CREATE TABLE albums (
			album_uid TEXT PRIMARY KEY,
			album_title TEXT NOT NULL,
			album_artist TEXT,
			year INTEGER,
			track_count INTEGER NOT NULL,
			total_duration_ms INTEGER NOT NULL,
			album_root_dir_hint TEXT,
			cover_ref_id INTEGER,
			indexed_at INTEGER NOT NULL,
			album_title_norm TEXT NOT NULL
		);
		CREATE TABLE tracks (
			track_uid TEXT PRIMARY KEY,
			title TEXT,
			filename TEXT NOT NULL,
			artist TEXT,
			album_artist TEXT,
			relative_path TEXT NOT NULL,
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
			album_uid, album_title, album_artist, year, track_count, total_duration_ms,
			album_root_dir_hint, cover_ref_id, indexed_at, album_title_norm
		) VALUES (
			'album-001', 'Blue Room Sessions', 'Demo Artist', 2024, 2, 481000,
			'/Albums/Blue Room Sessions', 1, 1710000100, 'blue room sessions'
		);
		INSERT INTO tracks (
			track_uid, title, filename, artist, album_artist, relative_path,
			format, duration_ms, sample_rate, indexed_at
		) VALUES
			('track-001', 'Opening', '01-opening.flac', 'Demo Artist', 'Demo Artist', '/Albums/Blue Room Sessions/01-opening.flac', 'flac', 201000, 44100, 1710000101),
			('track-002', 'Night Signal', '02-night-signal.flac', 'Demo Artist', 'Demo Artist', '/Albums/Blue Room Sessions/02-night-signal.flac', 'flac', 280000, 44100, 1710000102);
		INSERT INTO artists (artist_id, artist_name) VALUES (1, 'Demo Artist');
		INSERT INTO genres (genre_id, genre_name) VALUES (1, 'Ambient');
	`); err != nil {
		t.Fatalf("seed library rows: %v", err)
	}

	snapshot := New(dbPath).Snapshot(context.Background())
	if !snapshot.Available {
		t.Fatalf("expected library snapshot to be available, got error: %s", snapshot.Error)
	}
	if snapshot.Error != "" {
		t.Fatalf("expected empty snapshot error, got %s", snapshot.Error)
	}
	if snapshot.Stats.VolumeCount != 1 || snapshot.Stats.AlbumCount != 1 || snapshot.Stats.TrackCount != 2 {
		t.Fatalf("unexpected stats: %+v", snapshot.Stats)
	}
	if snapshot.Stats.ArtistCount != 1 || snapshot.Stats.GenreCount != 1 {
		t.Fatalf("unexpected artist/genre counts: %+v", snapshot.Stats)
	}
	if len(snapshot.Volumes) != 1 || snapshot.Volumes[0].Label != "Demo TF" {
		t.Fatalf("unexpected volumes: %+v", snapshot.Volumes)
	}
	if len(snapshot.Albums) != 1 || snapshot.Albums[0].Title != "Blue Room Sessions" {
		t.Fatalf("unexpected albums: %+v", snapshot.Albums)
	}
	if len(snapshot.Tracks) != 2 || snapshot.Tracks[0].Title != "Night Signal" {
		t.Fatalf("unexpected tracks: %+v", snapshot.Tracks)
	}
}

func TestSnapshotReportsMissingLibraryDB(t *testing.T) {
	snapshot := New(filepath.Join(t.TempDir(), "missing-library.db")).Snapshot(context.Background())
	if snapshot.Available {
		t.Fatalf("expected missing library db to be unavailable")
	}
	if snapshot.Error == "" {
		t.Fatalf("expected missing library db error")
	}
}
~~~

## `services/controld/internal/logclient/client.go`

- bytes: 950
- segment: 1/1

~~~go
package logclient

import (
	"context"
	"fmt"
	"os/exec"
	"strconv"
	"strings"
	"time"
)

const (
	defaultLines = 300
	maxLines     = 1000
	readTimeout  = 4 * time.Second
)

type Client struct{}

func New() *Client {
	return &Client{}
}

func (c *Client) Recent(ctx context.Context, lines int) (string, error) {
	lines = clampLines(lines)
	ctx, cancel := context.WithTimeout(ctx, readTimeout)
	defer cancel()

	cmd := exec.CommandContext(ctx, "journalctl", "-b", "--no-pager", "-n", strconv.Itoa(lines))
	output, err := cmd.CombinedOutput()
	text := string(output)
	if err != nil {
		if strings.TrimSpace(text) == "" {
			text = err.Error() + "\n"
		}
		return text, fmt.Errorf("journalctl: %w", err)
	}
	if strings.TrimSpace(text) == "" {
		return "(journalctl returned no lines)\n", nil
	}

	return text, nil
}

func clampLines(lines int) int {
	if lines <= 0 {
		return defaultLines
	}
	if lines > maxLines {
		return maxLines
	}

	return lines
}
~~~

## `services/controld/internal/playbackclient/client.go`

- bytes: 10517
- segment: 1/1

~~~go
package playbackclient

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"strconv"
	"strings"
	"time"
)

type Client struct {
	CommandSocket string
	EventSocket   string
}

type Status struct {
	Available    bool
	State        string
	OrderMode    string
	RepeatMode   string
	CurrentTrack string
	LastCommand  string
	QueueEntries int
	Raw          string
	Error        string
}

type QueueSnapshot struct {
	Available         bool
	OrderMode         string
	RepeatMode        string
	CurrentOrderIndex *int
	Entries           []QueueEntry
	Raw               string
	Error             string
}

type QueueEntry struct {
	OrderIndex   int     `json:"order_index"`
	QueueEntryID string  `json:"queue_entry_id"`
	TrackUID     string  `json:"track_uid"`
	VolumeUUID   string  `json:"volume_uuid"`
	RelativePath string  `json:"relative_path"`
	Title        *string `json:"title"`
	DurationMS   *uint64 `json:"duration_ms"`
	IsCurrent    bool    `json:"is_current"`
}

type Event struct {
	Name        string `json:"name"`
	TrackID     string `json:"track_id,omitempty"`
	Reason      string `json:"reason,omitempty"`
	Class       string `json:"class,omitempty"`
	Recoverable bool   `json:"recoverable,omitempty"`
	KeepQuiet   bool   `json:"keep_quiet,omitempty"`
}

func New(commandSocket, eventSocket string) *Client {
	return &Client{
		CommandSocket: commandSocket,
		EventSocket:   eventSocket,
	}
}

func (c *Client) Status(ctx context.Context) Status {
	line, err := c.request(ctx, "STATUS")
	if err != nil {
		return Status{Error: err.Error()}
	}

	status, err := parseStatusResponse(line)
	if err != nil {
		return Status{Raw: line, Error: err.Error()}
	}

	status.Available = true
	status.Raw = line
	return status
}

func (c *Client) QueueSnapshot(ctx context.Context) QueueSnapshot {
	line, err := c.request(ctx, "QUEUE_SNAPSHOT")
	if err != nil {
		return QueueSnapshot{Error: err.Error()}
	}

	snapshot, err := parseQueueSnapshotResponse(line)
	if err != nil {
		return QueueSnapshot{Raw: line, Error: err.Error()}
	}

	snapshot.Available = true
	snapshot.Raw = line
	return snapshot
}

func (c *Client) Execute(ctx context.Context, action, trackID string) (string, error) {
	line, err := commandLine(action, trackID)
	if err != nil {
		return "", err
	}

	response, err := c.request(ctx, line)
	if err != nil {
		return "", err
	}

	kind, fields, err := parseResponse(response)
	if err != nil {
		return "", err
	}
	if kind == "ERR" {
		return "", fmt.Errorf("%s: %s", fields["code"], fields["message"])
	}

	if fields["kind"] == "status" {
		status, err := parseStatusResponse(response)
		if err != nil {
			return "", err
		}

		current := placeholder(status.CurrentTrack)
		return fmt.Sprintf("STATUS -> state=%s current=%s", status.State, current), nil
	}
	if fields["kind"] == "queue_snapshot" {
		snapshot, err := parseQueueSnapshotResponse(response)
		if err != nil {
			return "", err
		}

		return fmt.Sprintf("QUEUE_SNAPSHOT -> entries=%d current_index=%s", len(snapshot.Entries), pointerLabel(snapshot.CurrentOrderIndex)), nil
	}

	actionName := strings.ToUpper(fields["action"])
	state := fields["state"]
	currentTrack := placeholder(fields["current_track"])
	return fmt.Sprintf("%s -> state=%s current=%s", actionName, state, currentTrack), nil
}

func (c *Client) SubscribeEvents(ctx context.Context, handler func(Event) error) error {
	dialer := &net.Dialer{Timeout: 2 * time.Second}
	conn, err := dialer.DialContext(ctx, "unix", c.EventSocket)
	if err != nil {
		return fmt.Errorf("dial playback event socket: %w", err)
	}
	defer conn.Close()

	reader := bufio.NewReader(conn)
	for {
		if deadline, ok := ctx.Deadline(); ok {
			_ = conn.SetReadDeadline(deadline)
		} else {
			_ = conn.SetReadDeadline(time.Now().Add(30 * time.Second))
		}

		line, err := reader.ReadString('\n')
		if err != nil {
			if ctx.Err() != nil {
				return ctx.Err()
			}
			if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
				continue
			}
			if err == io.EOF {
				return nil
			}
			return fmt.Errorf("read playback event: %w", err)
		}

		event, err := parseEventLine(line)
		if err != nil {
			return err
		}
		if err := handler(event); err != nil {
			return err
		}
	}
}

func (c *Client) request(ctx context.Context, line string) (string, error) {
	dialer := &net.Dialer{Timeout: 2 * time.Second}
	conn, err := dialer.DialContext(ctx, "unix", c.CommandSocket)
	if err != nil {
		return "", fmt.Errorf("dial playback socket: %w", err)
	}
	defer conn.Close()

	if deadline, ok := ctx.Deadline(); ok {
		_ = conn.SetDeadline(deadline)
	} else {
		_ = conn.SetDeadline(time.Now().Add(2 * time.Second))
	}

	if _, err := io.WriteString(conn, line+"\n"); err != nil {
		return "", fmt.Errorf("write playback command: %w", err)
	}

	response, err := bufio.NewReader(conn).ReadString('\n')
	if err != nil && err != io.EOF {
		return "", fmt.Errorf("read playback response: %w", err)
	}

	return strings.TrimRight(response, "\r\n"), nil
}

func commandLine(action, trackID string) (string, error) {
	switch strings.ToLower(strings.TrimSpace(action)) {
	case "ping":
		return "PING", nil
	case "status":
		return "STATUS", nil
	case "queue_snapshot":
		return "QUEUE_SNAPSHOT", nil
	case "play":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("track id is required for PLAY")
		}
		return "PLAY " + trackID, nil
	case "play_history":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("track id is required for PLAY_HISTORY")
		}
		return "PLAY_HISTORY " + trackID, nil
	case "queue_append":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("track id is required for QUEUE_APPEND")
		}
		return "QUEUE_APPEND " + trackID, nil
	case "queue_insert_next":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("track id is required for QUEUE_INSERT_NEXT")
		}
		return "QUEUE_INSERT_NEXT " + trackID, nil
	case "queue_remove":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("queue entry id is required for QUEUE_REMOVE")
		}
		return "QUEUE_REMOVE " + trackID, nil
	case "queue_clear":
		return "QUEUE_CLEAR", nil
	case "queue_replace":
		trackID = strings.TrimSpace(trackID)
		if trackID == "" {
			return "", fmt.Errorf("JSON track list is required for QUEUE_REPLACE")
		}
		return "QUEUE_REPLACE " + trackID, nil
	case "pause":
		return "PAUSE", nil
	case "stop":
		return "STOP", nil
	case "next":
		return "NEXT", nil
	case "prev":
		return "PREV", nil
	default:
		return "", fmt.Errorf("unsupported action: %s", action)
	}
}

func parseStatusResponse(line string) (Status, error) {
	kind, fields, err := parseResponse(line)
	if err != nil {
		return Status{}, err
	}
	if kind != "OK" {
		return Status{}, fmt.Errorf("unexpected non-OK playback response")
	}
	if fields["kind"] != "status" {
		return Status{}, fmt.Errorf("unexpected playback response kind: %s", fields["kind"])
	}

	queueEntries, err := strconv.Atoi(fields["queue_entries"])
	if err != nil {
		return Status{}, fmt.Errorf("invalid queue_entries value: %w", err)
	}

	return Status{
		State:        fields["state"],
		OrderMode:    fields["order_mode"],
		RepeatMode:   fields["repeat_mode"],
		CurrentTrack: blankIfPlaceholder(fields["current_track"]),
		LastCommand:  blankIfPlaceholder(fields["last_command"]),
		QueueEntries: queueEntries,
	}, nil
}

func parseQueueSnapshotResponse(line string) (QueueSnapshot, error) {
	kind, fields, err := parseResponse(line)
	if err != nil {
		return QueueSnapshot{}, err
	}
	if kind != "OK" {
		return QueueSnapshot{}, fmt.Errorf("unexpected non-OK playback response")
	}
	if fields["kind"] != "queue_snapshot" {
		return QueueSnapshot{}, fmt.Errorf("unexpected playback response kind: %s", fields["kind"])
	}

	var payload struct {
		OrderMode         string       `json:"order_mode"`
		RepeatMode        string       `json:"repeat_mode"`
		CurrentOrderIndex *int         `json:"current_order_index"`
		Entries           []QueueEntry `json:"entries"`
	}
	if err := json.Unmarshal([]byte(fields["payload"]), &payload); err != nil {
		return QueueSnapshot{}, fmt.Errorf("invalid queue snapshot payload: %w", err)
	}

	return QueueSnapshot{
		OrderMode:         payload.OrderMode,
		RepeatMode:        payload.RepeatMode,
		CurrentOrderIndex: payload.CurrentOrderIndex,
		Entries:           payload.Entries,
	}, nil
}

func parseEventLine(line string) (Event, error) {
	trimmed := strings.TrimSpace(line)
	if trimmed == "" {
		return Event{}, fmt.Errorf("playback event is empty")
	}

	parts := strings.Split(trimmed, "\t")
	if parts[0] != "EVENT" {
		return Event{}, fmt.Errorf("unsupported playback event prefix: %s", parts[0])
	}

	fields := make(map[string]string, len(parts)-1)
	for _, part := range parts[1:] {
		key, value, ok := strings.Cut(part, "=")
		if !ok {
			return Event{}, fmt.Errorf("malformed playback event field: %s", part)
		}
		fields[key] = value
	}

	event := Event{Name: fields["name"]}
	switch event.Name {
	case "PLAY_REQUEST_ACCEPTED", "PLAYBACK_STARTED", "TRACK_CHANGED":
		event.TrackID = fields["track_id"]
	case "PLAYBACK_STOPPED":
		event.Reason = fields["reason"]
	case "PLAYBACK_FAILED":
		event.Reason = fields["reason"]
		event.Class = fields["class"]
		event.Recoverable = fields["recoverable"] == "true"
		event.KeepQuiet = fields["keep_quiet"] == "true"
	case "PLAYBACK_PAUSED", "PLAYBACK_RESUMED":
	default:
		return Event{}, fmt.Errorf("unsupported playback event name: %s", event.Name)
	}

	return event, nil
}

func parseResponse(line string) (string, map[string]string, error) {
	trimmed := strings.TrimSpace(line)
	if trimmed == "" {
		return "", nil, fmt.Errorf("playback response is empty")
	}

	parts := strings.Split(trimmed, "\t")
	kind := parts[0]
	if kind != "OK" && kind != "ERR" {
		return "", nil, fmt.Errorf("unsupported playback response prefix: %s", kind)
	}

	fields := make(map[string]string, len(parts)-1)
	for _, part := range parts[1:] {
		key, value, ok := strings.Cut(part, "=")
		if !ok {
			return "", nil, fmt.Errorf("malformed playback field: %s", part)
		}
		fields[key] = value
	}

	return kind, fields, nil
}

func blankIfPlaceholder(value string) string {
	if value == "-" {
		return ""
	}

	return value
}

func placeholder(value string) string {
	if strings.TrimSpace(value) == "" {
		return "-"
	}

	return value
}

func pointerLabel(value *int) string {
	if value == nil {
		return "-"
	}

	return strconv.Itoa(*value)
}
~~~

## `services/controld/internal/playbackclient/client_test.go`

- bytes: 8066
- segment: 1/1

~~~go
package playbackclient

import (
	"context"
	"net"
	"os"
	"path/filepath"
	"reflect"
	"testing"
	"time"
)

func TestStatusParsesUnixSocketResponse(t *testing.T) {
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
		_, _ = conn.Write([]byte("OK\tkind=status\tstate=quiet_active\torder_mode=sequential\trepeat_mode=off\tcurrent_track=demo track\tlast_command=play:demo track\tqueue_entries=2\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	status := client.Status(ctx)
	if !status.Available {
		t.Fatalf("expected status to be available, got error: %s", status.Error)
	}
	if status.State != "quiet_active" {
		t.Fatalf("unexpected state: %s", status.State)
	}
	if status.CurrentTrack != "demo track" {
		t.Fatalf("unexpected current track: %s", status.CurrentTrack)
	}
	if status.QueueEntries != 2 {
		t.Fatalf("unexpected queue entry count: %d", status.QueueEntries)
	}

	<-done
}

func TestExecuteFormatsPlayHistoryCommand(t *testing.T) {
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

		buf := make([]byte, 256)
		n, _ := conn.Read(buf)
		if string(buf[:n]) != "PLAY_HISTORY side a track 01\n" {
			t.Errorf("unexpected command line: %q", string(buf[:n]))
			return
		}

		_, _ = conn.Write([]byte("OK\tkind=ack\taction=play_history\tstate=quiet_active\tcurrent_track=side a track 01\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	message, err := client.Execute(ctx, "play_history", "side a track 01")
	if err != nil {
		t.Fatalf("execute play_history: %v", err)
	}
	if message != "PLAY_HISTORY -> state=quiet_active current=side a track 01" {
		t.Fatalf("unexpected execute message: %s", message)
	}

	<-done
}

func TestExecuteFormatsQueueReplaceCommand(t *testing.T) {
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

		buf := make([]byte, 256)
		n, _ := conn.Read(buf)
		if string(buf[:n]) != "QUEUE_REPLACE [\"side a 01\",\"side b 02\"]\n" {
			t.Errorf("unexpected command line: %q", string(buf[:n]))
			return
		}

		_, _ = conn.Write([]byte("OK\tkind=ack\taction=queue_replace\tstate=stopped\tcurrent_track=side a 01\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	message, err := client.Execute(ctx, "queue_replace", "[\"side a 01\",\"side b 02\"]")
	if err != nil {
		t.Fatalf("execute queue_replace: %v", err)
	}
	if message != "QUEUE_REPLACE -> state=stopped current=side a 01" {
		t.Fatalf("unexpected execute message: %s", message)
	}

	<-done
}

func TestQueueSnapshotParsesSnapshotPayload(t *testing.T) {
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

		buf := make([]byte, 256)
		n, _ := conn.Read(buf)
		if string(buf[:n]) != "QUEUE_SNAPSHOT\n" {
			t.Errorf("unexpected command line: %q", string(buf[:n]))
			return
		}

		_, _ = conn.Write([]byte("OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":0,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-a\",\"volume_uuid\":\"manual\",\"relative_path\":\"track-a\",\"title\":\"track-a\",\"duration_ms\":null,\"is_current\":true}]}\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	snapshot := client.QueueSnapshot(ctx)
	if !snapshot.Available {
		t.Fatalf("expected queue snapshot to be available, got error: %s", snapshot.Error)
	}
	if snapshot.CurrentOrderIndex == nil || *snapshot.CurrentOrderIndex != 0 {
		t.Fatalf("unexpected current order index: %#v", snapshot.CurrentOrderIndex)
	}
	if len(snapshot.Entries) != 1 {
		t.Fatalf("unexpected entry count: %d", len(snapshot.Entries))
	}
	if snapshot.Entries[0].QueueEntryID != "q1" {
		t.Fatalf("unexpected queue entry id: %s", snapshot.Entries[0].QueueEntryID)
	}
	if !snapshot.Entries[0].IsCurrent {
		t.Fatalf("expected entry to be current")
	}

	<-done
}

func TestExecuteFormatsQueueSnapshotResponse(t *testing.T) {
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

		buf := make([]byte, 256)
		n, _ := conn.Read(buf)
		if string(buf[:n]) != "QUEUE_SNAPSHOT\n" {
			t.Errorf("unexpected command line: %q", string(buf[:n]))
			return
		}

		_, _ = conn.Write([]byte("OK\tkind=queue_snapshot\tpayload={\"order_mode\":\"sequential\",\"repeat_mode\":\"off\",\"current_order_index\":1,\"entries\":[{\"order_index\":0,\"queue_entry_id\":\"q1\",\"track_uid\":\"track-a\",\"volume_uuid\":\"manual\",\"relative_path\":\"track-a\",\"title\":\"track-a\",\"duration_ms\":null,\"is_current\":false},{\"order_index\":1,\"queue_entry_id\":\"q2\",\"track_uid\":\"track-b\",\"volume_uuid\":\"manual\",\"relative_path\":\"track-b\",\"title\":\"track-b\",\"duration_ms\":null,\"is_current\":true}]}\n"))
	}()

	client := New(socketPath, "")
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	message, err := client.Execute(ctx, "queue_snapshot", "")
	if err != nil {
		t.Fatalf("execute queue_snapshot: %v", err)
	}
	if message != "QUEUE_SNAPSHOT -> entries=2 current_index=1" {
		t.Fatalf("unexpected execute message: %s", message)
	}

	<-done
}

func TestSubscribeEventsParsesPlaybackEventStream(t *testing.T) {
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

		_, _ = conn.Write([]byte("EVENT\tname=PLAYBACK_STARTED\ttrack_id=demo-track-001\n"))
		_, _ = conn.Write([]byte("EVENT\tname=TRACK_CHANGED\ttrack_id=demo-track-002\n"))
	}()

	client := New("", socketPath)
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	var events []Event
	err = client.SubscribeEvents(ctx, func(event Event) error {
		events = append(events, event)
		if len(events) == 2 {
			cancel()
		}
		return nil
	})
	if err != nil && err != context.Canceled {
		t.Fatalf("subscribe events: %v", err)
	}

	want := []Event{
		{Name: "PLAYBACK_STARTED", TrackID: "demo-track-001"},
		{Name: "TRACK_CHANGED", TrackID: "demo-track-002"},
	}
	if !reflect.DeepEqual(events, want) {
		t.Fatalf("unexpected events: %#v", events)
	}

	<-done
}

func shortSocketPath(t *testing.T) string {
	t.Helper()

	dir, err := os.MkdirTemp("/tmp", "npct4-uds-")
	if err != nil {
		t.Fatalf("create short temp dir: %v", err)
	}
	t.Cleanup(func() {
		_ = os.RemoveAll(dir)
	})

	return filepath.Join(dir, "playback.sock")
}
~~~

## `services/controld/internal/provisioningclient/client.go`

- bytes: 1935
- segment: 1/1

~~~go
package provisioningclient

import (
	"context"
	"encoding/json"
	"os"
)

type Snapshot struct {
	Available      bool   `json:"available"`
	State          string `json:"state,omitempty"`
	Message        string `json:"message,omitempty"`
	SSID           string `json:"ssid,omitempty"`
	IP             string `json:"ip,omitempty"`
	WiFiIP         string `json:"wifi_ip,omitempty"`
	WiredIP        string `json:"wired_ip,omitempty"`
	AllIPs         []string `json:"all_ips,omitempty"`
	WebURL         string `json:"web_url,omitempty"`
	Hostname       string `json:"hostname,omitempty"`
	WiFiInterface  string `json:"wifi_interface,omitempty"`
	WPAUnit        string `json:"wpa_unit,omitempty"`
	StatusPath     string `json:"status_path,omitempty"`
	UpdatedAt      string `json:"updated_at,omitempty"`
	Error          string `json:"error,omitempty"`
	ErrorCode      string `json:"error_code,omitempty"`
	ApplyOutput    string `json:"apply_output,omitempty"`
	DiagnosticHint string `json:"diagnostic_hint,omitempty"`
	IPWaitSeconds  int    `json:"ip_wait_seconds,omitempty"`
	ReadError      string `json:"read_error,omitempty"`
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
~~~

## `services/controld/internal/provisioningclient/client_test.go`

- bytes: 1968
- segment: 1/1

~~~go
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
~~~

## `services/controld/internal/settings/config.go`

- bytes: 1820
- segment: 1/1

~~~go
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
		DSDPolicy:     "strict_native",
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
~~~

## `services/controld/internal/settings/config_test.go`

- bytes: 1308
- segment: 1/1

~~~go
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
~~~

## `services/controld/internal/sshctl/controller.go`

- bytes: 204
- segment: 1/1

~~~go
package sshctl

type Controller struct {
	enabled bool
}

func NewController(enabled bool) *Controller {
	return &Controller{enabled: enabled}
}

func (c *Controller) Enabled() bool {
	return c.enabled
}
~~~

## `services/controld/web/embed.go`

- bytes: 199
- segment: 1/1

~~~go
package web

import "embed"

// Assets embeds the minimal SSR templates and static files used by the
// Lumelo controld prototype.
//
//go:embed templates/*.html static/css/*.css
var Assets embed.FS
~~~

## `services/controld/web/static/css/app.css`

- bytes: 6610
- segment: 1/1

~~~css
:root {
  color-scheme: light;
  --bg: #f3efe7;
  --ink: #1f1d1a;
  --muted: #6f675e;
  --card: #fffaf2;
  --line: #d8cdbd;
  --accent: #9b5d2e;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-height: 100vh;
  background:
    radial-gradient(circle at top, rgba(155, 93, 46, 0.12), transparent 28rem),
    linear-gradient(180deg, #f7f2ea 0%, var(--bg) 100%);
  color: var(--ink);
  font-family: "IBM Plex Sans", "Segoe UI", sans-serif;
}

.shell {
  width: min(56rem, calc(100% - 2rem));
  margin: 0 auto;
  padding: 2rem 0 4rem;
}

.card {
  padding: 1.25rem;
  margin-bottom: 1rem;
  border: 1px solid var(--line);
  border-radius: 1.25rem;
  background: rgba(255, 250, 242, 0.88);
  box-shadow: 0 0.75rem 2rem rgba(31, 29, 26, 0.08);
}

.eyebrow {
  margin: 0 0 0.5rem;
  color: var(--accent);
  font-size: 0.85rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

h1,
h2,
p,
dt,
dd {
  margin: 0;
}

h1 {
  font-size: clamp(2rem, 5vw, 3rem);
  line-height: 1.05;
}

h2 {
  margin-bottom: 1rem;
  font-size: 1rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.topnav {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1rem;
}

.topnav-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 5.5rem;
  padding: 0.7rem 1rem;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: rgba(255, 250, 242, 0.82);
  color: var(--ink);
  font-weight: 700;
  text-decoration: none;
  transition: transform 140ms ease, border-color 140ms ease;
}

.topnav-link:hover {
  transform: translateY(-1px);
  border-color: rgba(155, 93, 46, 0.38);
}

.topnav-link-current {
  border-color: rgba(155, 93, 46, 0.42);
  background: rgba(155, 93, 46, 0.12);
  color: var(--accent);
}

.summary {
  margin-top: 0.75rem;
  color: var(--muted);
  line-height: 1.6;
}

.status-line {
  margin-bottom: 1rem;
  font-weight: 600;
}

.grid {
  display: grid;
  gap: 0.9rem;
}

.grid div {
  padding-top: 0.85rem;
  border-top: 1px solid var(--line);
}

dt {
  color: var(--muted);
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

dd {
  margin-top: 0.25rem;
  word-break: break-word;
  font-weight: 600;
}

.banner {
  margin-bottom: 1rem;
  padding: 0.8rem 0.95rem;
  border-radius: 0.9rem;
  line-height: 1.45;
}

.banner-ok {
  background: rgba(74, 122, 71, 0.12);
  border: 1px solid rgba(74, 122, 71, 0.26);
}

.banner-error {
  background: rgba(145, 63, 48, 0.12);
  border: 1px solid rgba(145, 63, 48, 0.26);
}

.banner-warn {
  background: rgba(155, 93, 46, 0.14);
  border: 1px solid rgba(155, 93, 46, 0.28);
  color: #6e421f;
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  font-size: 0.78rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.pill-ok {
  background: rgba(74, 122, 71, 0.12);
  color: #335432;
}

.pill-offline {
  background: rgba(145, 63, 48, 0.12);
  color: #7b3328;
}

.stack {
  display: grid;
  gap: 0.75rem;
}

.label {
  font-size: 0.86rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.input {
  width: 100%;
  padding: 0.85rem 0.95rem;
  border: 1px solid var(--line);
  border-radius: 0.9rem;
  background: #fffdf9;
  color: var(--ink);
  font: inherit;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.queue-tools {
  margin-top: 1rem;
}

.queue-list {
  display: grid;
  gap: 0.8rem;
  margin-top: 1rem;
}

.queue-item {
  display: grid;
  gap: 0.85rem;
  align-items: start;
  padding: 0.95rem;
  border: 1px solid var(--line);
  border-radius: 1rem;
  background: rgba(255, 253, 249, 0.88);
}

.queue-item-current {
  border-color: rgba(155, 93, 46, 0.38);
  box-shadow: inset 0 0 0 1px rgba(155, 93, 46, 0.08);
}

.queue-order {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.2rem;
  height: 2.2rem;
  border-radius: 999px;
  background: rgba(155, 93, 46, 0.12);
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.04em;
}

.queue-copy {
  min-width: 0;
}

.queue-title {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  font-weight: 700;
}

.queue-meta {
  margin-top: 0.3rem;
  color: var(--muted);
  font-size: 0.88rem;
  line-height: 1.5;
  word-break: break-word;
}

.queue-row-action {
  align-self: center;
}

.library-list {
  display: grid;
  gap: 0.8rem;
}

.library-item {
  padding: 0.95rem;
  border: 1px solid var(--line);
  border-radius: 1rem;
  background: rgba(255, 253, 249, 0.88);
}

.library-cover-link {
  display: inline-flex;
  margin-bottom: 0.8rem;
  border-radius: 0.9rem;
  overflow: hidden;
  border: 1px solid rgba(31, 29, 26, 0.12);
  background: rgba(31, 29, 26, 0.04);
}

.library-cover-art {
  display: block;
  width: 5.5rem;
  height: 5.5rem;
  object-fit: cover;
}

.library-title {
  font-weight: 700;
  line-height: 1.5;
}

.library-meta {
  margin-top: 0.3rem;
  color: var(--muted);
  font-size: 0.9rem;
  line-height: 1.5;
  word-break: break-word;
}

.pill-current {
  background: rgba(155, 93, 46, 0.14);
  color: var(--accent);
}

button {
  border: 0;
  border-radius: 999px;
  padding: 0.8rem 1rem;
  background: var(--ink);
  color: #fff9f1;
  font: inherit;
  font-weight: 700;
  cursor: pointer;
  transition: transform 140ms ease, opacity 140ms ease;
}

button:hover {
  transform: translateY(-1px);
  opacity: 0.95;
}

button:active {
  transform: translateY(0);
}

.button-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  padding: 0.8rem 1rem;
  background: var(--ink);
  color: #fff9f1;
  font-weight: 700;
  text-decoration: none;
  transition: transform 140ms ease, opacity 140ms ease;
}

.button-link:hover {
  transform: translateY(-1px);
  opacity: 0.95;
}

.button-link-secondary {
  background: rgba(31, 29, 26, 0.08);
  color: var(--ink);
  border: 1px solid var(--line);
}

.log-actions {
  margin-top: 1rem;
}

.log-output {
  overflow: auto;
  max-height: 34rem;
  margin: 1rem 0 0;
  padding: 1rem;
  border: 1px solid rgba(31, 29, 26, 0.16);
  border-radius: 1rem;
  background: #1f1d1a;
  color: #fff7e8;
  font-family: "IBM Plex Mono", "SFMono-Regular", Consolas, monospace;
  font-size: 0.82rem;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
}

@media (min-width: 720px) {
  .shell {
    padding-top: 3rem;
  }

  .grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    column-gap: 1rem;
  }

  .queue-item {
    grid-template-columns: auto minmax(0, 1fr) auto;
  }
}
~~~

