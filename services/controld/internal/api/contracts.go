package api

import (
	"encoding/json"
	"net/http"

	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/playbackclient"
	"github.com/lumelo/controld/internal/provisioningclient"
)

type systemSummaryView struct {
	Mode               string `json:"mode"`
	InterfaceMode      string `json:"interface_mode"`
	DSDPolicy          string `json:"dsd_policy"`
	PasswordConfigured bool   `json:"password_configured"`
	SSHEnabled         bool   `json:"ssh_enabled"`
	CommandSocket      string `json:"command_socket"`
	EventSocket        string `json:"event_socket"`
	LibraryDBPath      string `json:"library_db_path"`
	ConfigPath         string `json:"config_path"`
	AudioOutputPath    string `json:"audio_output_path"`
}

type playbackStatusView struct {
	Available    bool   `json:"available"`
	State        string `json:"state,omitempty"`
	OrderMode    string `json:"order_mode,omitempty"`
	RepeatMode   string `json:"repeat_mode,omitempty"`
	CurrentTrack string `json:"current_track,omitempty"`
	LastCommand  string `json:"last_command,omitempty"`
	QueueEntries int    `json:"queue_entries,omitempty"`
	Raw          string `json:"raw,omitempty"`
	Error        string `json:"error,omitempty"`
}

type playbackQueueEntryView struct {
	OrderIndex   int     `json:"order_index"`
	QueueEntryID string  `json:"queue_entry_id"`
	TrackUID     string  `json:"track_uid"`
	VolumeUUID   string  `json:"volume_uuid"`
	RelativePath string  `json:"relative_path"`
	Title        *string `json:"title,omitempty"`
	DurationMS   *uint64 `json:"duration_ms,omitempty"`
	IsCurrent    bool    `json:"is_current"`
}

type playbackQueueView struct {
	Available         bool                     `json:"available"`
	OrderMode         string                   `json:"order_mode,omitempty"`
	RepeatMode        string                   `json:"repeat_mode,omitempty"`
	CurrentOrderIndex *int                     `json:"current_order_index,omitempty"`
	Entries           []playbackQueueEntryView `json:"entries,omitempty"`
	Raw               string                   `json:"raw,omitempty"`
	Error             string                   `json:"error,omitempty"`
}

type playbackHistoryEntryView struct {
	PlayedAt     uint64  `json:"played_at"`
	TrackUID     string  `json:"track_uid"`
	VolumeUUID   string  `json:"volume_uuid"`
	RelativePath string  `json:"relative_path"`
	Title        *string `json:"title,omitempty"`
	DurationMS   *uint64 `json:"duration_ms,omitempty"`
}

type playbackHistoryView struct {
	Available bool                       `json:"available"`
	Entries   []playbackHistoryEntryView `json:"entries,omitempty"`
	Raw       string                     `json:"raw,omitempty"`
	Error     string                     `json:"error,omitempty"`
}

type libraryQueryView struct {
	AlbumUID            string `json:"album_uid,omitempty"`
	DirectoryVolumeUUID string `json:"directory_volume_uuid,omitempty"`
	DirectoryPath       string `json:"directory_path,omitempty"`
}

type libraryStatsView struct {
	VolumeCount int `json:"volume_count"`
	AlbumCount  int `json:"album_count"`
	TrackCount  int `json:"track_count"`
	ArtistCount int `json:"artist_count"`
	GenreCount  int `json:"genre_count"`
}

type libraryVolumeSnapshotView struct {
	VolumeUUID  string `json:"volume_uuid"`
	Label       string `json:"label"`
	MountPath   string `json:"mount_path"`
	IsAvailable bool   `json:"is_available"`
	LastSeenAt  int64  `json:"last_seen_at"`
}

type libraryDirectorySnapshotView struct {
	VolumeUUID         string `json:"volume_uuid"`
	RelativePath       string `json:"relative_path"`
	ParentRelativePath string `json:"parent_relative_path,omitempty"`
	DisplayName        string `json:"display_name"`
}

type libraryAlbumSnapshotView struct {
	AlbumUID          string `json:"album_uid"`
	VolumeUUID        string `json:"volume_uuid"`
	Title             string `json:"title"`
	AlbumArtist       string `json:"album_artist,omitempty"`
	Year              int    `json:"year,omitempty"`
	TrackCount        int    `json:"track_count"`
	TotalDurationMS   int64  `json:"total_duration_ms"`
	RootDirHint       string `json:"root_dir_hint,omitempty"`
	CoverThumbRelPath string `json:"cover_thumb_rel_path,omitempty"`
	SourceMode        string `json:"source_mode,omitempty"`
}

type libraryTrackSnapshotView struct {
	TrackUID     string `json:"track_uid"`
	AlbumUID     string `json:"album_uid,omitempty"`
	AlbumTitle   string `json:"album_title,omitempty"`
	VolumeUUID   string `json:"volume_uuid,omitempty"`
	Title        string `json:"title,omitempty"`
	Artist       string `json:"artist,omitempty"`
	RelativePath string `json:"relative_path"`
	TrackNo      *int64 `json:"track_no,omitempty"`
	DiscNo       *int64 `json:"disc_no,omitempty"`
	Format       string `json:"format,omitempty"`
	DurationMS   *int64 `json:"duration_ms,omitempty"`
	SampleRate   *int64 `json:"sample_rate,omitempty"`
}

type librarySnapshotView struct {
	Available   bool                           `json:"available"`
	DBPath      string                         `json:"db_path"`
	Error       string                         `json:"error,omitempty"`
	Query       libraryQueryView               `json:"query"`
	Stats       libraryStatsView               `json:"stats"`
	Volumes     []libraryVolumeSnapshotView    `json:"volumes,omitempty"`
	Directories []libraryDirectorySnapshotView `json:"directories,omitempty"`
	Albums      []libraryAlbumSnapshotView     `json:"albums,omitempty"`
	Tracks      []libraryTrackSnapshotView     `json:"tracks,omitempty"`
}

type playbackCommandRequest struct {
	Action  string `json:"action"`
	TrackID string `json:"track_id,omitempty"`
}

type libraryCommandRequest struct {
	Action  string           `json:"action"`
	TrackID string           `json:"track_id,omitempty"`
	Query   libraryQueryView `json:"query,omitempty"`
}

type playbackCommandResponse struct {
	OK             bool               `json:"ok"`
	Action         string             `json:"action,omitempty"`
	TrackID        string             `json:"track_id,omitempty"`
	Message        string             `json:"message,omitempty"`
	Error          string             `json:"error,omitempty"`
	PlaybackStatus playbackStatusView `json:"playback_status"`
	Queue          playbackQueueView  `json:"queue"`
}

type libraryCommandResponse struct {
	OK             bool               `json:"ok"`
	Action         string             `json:"action,omitempty"`
	TrackID        string             `json:"track_id,omitempty"`
	Query          libraryQueryView   `json:"query"`
	Message        string             `json:"message,omitempty"`
	Error          string             `json:"error,omitempty"`
	PlaybackStatus playbackStatusView `json:"playback_status"`
	Queue          playbackQueueView  `json:"queue"`
}

func buildSystemSummaryView(deps Dependencies, provisioning provisioningclient.Snapshot) systemSummaryView {
	return systemSummaryView{
		Mode:               deps.Settings.Mode,
		InterfaceMode:      effectiveInterfaceMode(deps.Settings.InterfaceMode, provisioning),
		DSDPolicy:          deps.Settings.DSDPolicy,
		PasswordConfigured: deps.Auth.PasswordConfigured(),
		SSHEnabled:         deps.SSH.Enabled(),
		CommandSocket:      deps.Playback.CommandSocket,
		EventSocket:        deps.Playback.EventSocket,
		LibraryDBPath:      deps.Library.LibraryDBPath,
		ConfigPath:         deps.Settings.ConfigPath,
		AudioOutputPath:    "/api/v1/system/audio-output",
	}
}

func buildHealthPayload(
	deps Dependencies,
	playbackStatus playbackclient.Status,
	librarySnapshot libraryclient.Snapshot,
	provisioning provisioningclient.Snapshot,
) healthView {
	return healthView{
		Status:                "ok",
		Mode:                  deps.Settings.Mode,
		InterfaceMode:         effectiveInterfaceMode(deps.Settings.InterfaceMode, provisioning),
		SSHEnabled:            deps.SSH.Enabled(),
		PlaybackAvailable:     playbackStatus.Available,
		PlaybackState:         playbackStatus.State,
		PlaybackError:         playbackStatus.Error,
		LibraryAvailable:      librarySnapshot.Available,
		LibraryDBPath:         deps.Library.LibraryDBPath,
		LibraryError:          librarySnapshot.Error,
		ProvisioningAvailable: provisioning.Available,
		ProvisioningState:     provisioning.State,
		ProvisioningMessage:   provisioning.Message,
		ProvisioningErrorCode: provisioning.ErrorCode,
		ProvisioningBTAddress: provisioning.BluetoothAddress,
		ProvisioningRFCOMM:    provisioning.RFCOMMChannel,
		ProvisioningSDPCount:  len(provisioning.SDPRecordHandles),
		ProvisioningReadError: provisioning.ReadError,
	}
}

func buildPlaybackStatusView(status playbackclient.Status) playbackStatusView {
	return playbackStatusView{
		Available:    status.Available,
		State:        status.State,
		OrderMode:    status.OrderMode,
		RepeatMode:   status.RepeatMode,
		CurrentTrack: status.CurrentTrack,
		LastCommand:  status.LastCommand,
		QueueEntries: status.QueueEntries,
		Raw:          status.Raw,
		Error:        status.Error,
	}
}

func buildPlaybackQueueView(snapshot playbackclient.QueueSnapshot) playbackQueueView {
	entries := make([]playbackQueueEntryView, 0, len(snapshot.Entries))
	for _, entry := range snapshot.Entries {
		entries = append(entries, playbackQueueEntryView{
			OrderIndex:   entry.OrderIndex,
			QueueEntryID: entry.QueueEntryID,
			TrackUID:     entry.TrackUID,
			VolumeUUID:   entry.VolumeUUID,
			RelativePath: entry.RelativePath,
			Title:        entry.Title,
			DurationMS:   entry.DurationMS,
			IsCurrent:    entry.IsCurrent,
		})
	}

	return playbackQueueView{
		Available:         snapshot.Available,
		OrderMode:         snapshot.OrderMode,
		RepeatMode:        snapshot.RepeatMode,
		CurrentOrderIndex: snapshot.CurrentOrderIndex,
		Entries:           entries,
		Raw:               snapshot.Raw,
		Error:             snapshot.Error,
	}
}

func buildPlaybackHistoryView(snapshot playbackclient.HistorySnapshot) playbackHistoryView {
	entries := make([]playbackHistoryEntryView, 0, len(snapshot.Entries))
	for _, entry := range snapshot.Entries {
		entries = append(entries, playbackHistoryEntryView{
			PlayedAt:     entry.PlayedAt,
			TrackUID:     entry.TrackUID,
			VolumeUUID:   entry.VolumeUUID,
			RelativePath: entry.RelativePath,
			Title:        entry.Title,
			DurationMS:   entry.DurationMS,
		})
	}

	return playbackHistoryView{
		Available: snapshot.Available,
		Entries:   entries,
		Raw:       snapshot.Raw,
		Error:     snapshot.Error,
	}
}

func buildLibraryQueryView(query libraryclient.Query) libraryQueryView {
	return libraryQueryView{
		AlbumUID:            query.AlbumUID,
		DirectoryVolumeUUID: query.DirectoryVolumeUUID,
		DirectoryPath:       query.DirectoryPath,
	}
}

func buildLibrarySnapshotView(snapshot libraryclient.Snapshot) librarySnapshotView {
	volumes := make([]libraryVolumeSnapshotView, 0, len(snapshot.Volumes))
	for _, volume := range snapshot.Volumes {
		volumes = append(volumes, libraryVolumeSnapshotView{
			VolumeUUID:  volume.VolumeUUID,
			Label:       volume.Label,
			MountPath:   volume.MountPath,
			IsAvailable: volume.IsAvailable,
			LastSeenAt:  volume.LastSeenAt,
		})
	}

	directories := make([]libraryDirectorySnapshotView, 0, len(snapshot.Directories))
	for _, directory := range snapshot.Directories {
		directories = append(directories, libraryDirectorySnapshotView{
			VolumeUUID:         directory.VolumeUUID,
			RelativePath:       directory.RelativePath,
			ParentRelativePath: directory.ParentRelativePath,
			DisplayName:        directory.DisplayName,
		})
	}

	albums := make([]libraryAlbumSnapshotView, 0, len(snapshot.Albums))
	for _, album := range snapshot.Albums {
		albums = append(albums, libraryAlbumSnapshotView{
			AlbumUID:          album.AlbumUID,
			VolumeUUID:        album.VolumeUUID,
			Title:             album.Title,
			AlbumArtist:       album.AlbumArtist,
			Year:              album.Year,
			TrackCount:        album.TrackCount,
			TotalDurationMS:   album.TotalDurationMS,
			RootDirHint:       album.RootDirHint,
			CoverThumbRelPath: album.CoverThumbRelPath,
			SourceMode:        album.SourceMode,
		})
	}

	tracks := make([]libraryTrackSnapshotView, 0, len(snapshot.Tracks))
	for _, track := range snapshot.Tracks {
		tracks = append(tracks, libraryTrackSnapshotView{
			TrackUID:     track.TrackUID,
			AlbumUID:     track.AlbumUID,
			AlbumTitle:   track.AlbumTitle,
			VolumeUUID:   track.VolumeUUID,
			Title:        track.Title,
			Artist:       track.Artist,
			RelativePath: track.RelativePath,
			TrackNo:      track.TrackNo,
			DiscNo:       track.DiscNo,
			Format:       track.Format,
			DurationMS:   track.DurationMS,
			SampleRate:   track.SampleRate,
		})
	}

	return librarySnapshotView{
		Available: snapshot.Available,
		DBPath:    snapshot.DBPath,
		Error:     snapshot.Error,
		Query:     buildLibraryQueryView(snapshot.Query),
		Stats: libraryStatsView{
			VolumeCount: snapshot.Stats.VolumeCount,
			AlbumCount:  snapshot.Stats.AlbumCount,
			TrackCount:  snapshot.Stats.TrackCount,
			ArtistCount: snapshot.Stats.ArtistCount,
			GenreCount:  snapshot.Stats.GenreCount,
		},
		Volumes:     volumes,
		Directories: directories,
		Albums:      albums,
		Tracks:      tracks,
	}
}

func writeJSON(w http.ResponseWriter, payload any) error {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	return json.NewEncoder(w).Encode(payload)
}

func writeJSONStatus(w http.ResponseWriter, status int, payload any) error {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	return json.NewEncoder(w).Encode(payload)
}
