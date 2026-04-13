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
