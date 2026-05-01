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

type Query struct {
	AlbumUID            string
	DirectoryVolumeUUID string
	DirectoryPath       string
}

type Snapshot struct {
	Available   bool
	DBPath      string
	Error       string
	Stats       Stats
	Volumes     []VolumeSummary
	Directories []DirectorySummary
	Albums      []AlbumSummary
	Tracks      []TrackSummary
	Query       Query
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

type DirectorySummary struct {
	VolumeUUID         string
	RelativePath       string
	ParentRelativePath string
	DisplayName        string
}

type AlbumSummary struct {
	AlbumUID          string
	VolumeUUID        string
	Title             string
	AlbumArtist       string
	Year              int
	TrackCount        int
	TotalDurationMS   int64
	RootDirHint       string
	CoverThumbRelPath string
	SourceMode        string
}

type TrackSummary struct {
	TrackUID     string
	AlbumUID     string
	AlbumTitle   string
	VolumeUUID   string
	Title        string
	Artist       string
	RelativePath string
	TrackNo      *int64
	DiscNo       *int64
	Format       string
	DurationMS   *int64
	SampleRate   *int64
}

const UncategorizedAlbumUID = "__uncategorized__"

func New(libraryDBPath string) *Client {
	return &Client{LibraryDBPath: libraryDBPath}
}

func (c *Client) Snapshot(ctx context.Context) Snapshot {
	return c.QuerySnapshot(ctx, Query{})
}

func (c *Client) QuerySnapshot(ctx context.Context, query Query) Snapshot {
	snapshot := Snapshot{DBPath: c.LibraryDBPath, Query: query}

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
	directories, err := queryDirectories(ctx, db, query)
	if err != nil {
		snapshot.Error = fmt.Sprintf("query directories: %v", err)
		return snapshot
	}
	albums, err := queryAlbums(ctx, db)
	if err != nil {
		snapshot.Error = fmt.Sprintf("query albums: %v", err)
		return snapshot
	}
	tracks, err := queryTracks(ctx, db, query)
	if err != nil {
		snapshot.Error = fmt.Sprintf("query tracks: %v", err)
		return snapshot
	}

	snapshot.Stats = stats
	snapshot.Volumes = volumes
	snapshot.Directories = directories
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
	volumeCount, err := queryCount(ctx, db, visibleVolumeCountSQL)
	if err != nil {
		return Stats{}, err
	}
	albumCount, err := queryCount(ctx, db, visibleAlbumCountSQL)
	if err != nil {
		return Stats{}, err
	}
	trackCount, err := queryCount(ctx, db, visibleTrackCountSQL)
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
		WHERE `+visibleVolumePredicate("volumes")+`
		  AND EXISTS (
			SELECT 1 FROM tracks
			WHERE tracks.volume_uuid = volumes.volume_uuid
		  )
		ORDER BY last_seen_at DESC, mount_path ASC
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

func queryDirectories(ctx context.Context, db *sql.DB, query Query) ([]DirectorySummary, error) {
	if query.DirectoryVolumeUUID == "" {
		return nil, nil
	}

	if query.DirectoryPath == "" {
		rows, err := db.QueryContext(ctx, `
			SELECT
				directories.volume_uuid,
				directories.relative_path,
				COALESCE(directories.parent_relative_path, ''),
				directories.display_name
			FROM directories
			JOIN volumes ON volumes.volume_uuid = directories.volume_uuid
			WHERE directories.volume_uuid = ?1
			  AND `+visibleVolumePredicate("volumes")+`
			  AND COALESCE(directories.parent_relative_path, '') = ''
			  AND directories.relative_path <> ''
			ORDER BY directories.display_name ASC, directories.relative_path ASC
		`, query.DirectoryVolumeUUID)
		if err != nil {
			return nil, err
		}
		defer rows.Close()
		return scanDirectoryRows(rows)
	}

	rows, err := db.QueryContext(ctx, `
		SELECT
			directories.volume_uuid,
			directories.relative_path,
			COALESCE(directories.parent_relative_path, ''),
			directories.display_name
		FROM directories
		JOIN volumes ON volumes.volume_uuid = directories.volume_uuid
		WHERE directories.volume_uuid = ?1
		  AND `+visibleVolumePredicate("volumes")+`
		  AND COALESCE(directories.parent_relative_path, '') = ?2
		ORDER BY directories.display_name ASC, directories.relative_path ASC
	`, query.DirectoryVolumeUUID, query.DirectoryPath)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanDirectoryRows(rows)
}

func scanDirectoryRows(rows *sql.Rows) ([]DirectorySummary, error) {
	var directories []DirectorySummary
	for rows.Next() {
		var item DirectorySummary
		if err := rows.Scan(
			&item.VolumeUUID,
			&item.RelativePath,
			&item.ParentRelativePath,
			&item.DisplayName,
		); err != nil {
			return nil, err
		}
		directories = append(directories, item)
	}
	return directories, rows.Err()
}

func queryAlbums(ctx context.Context, db *sql.DB) ([]AlbumSummary, error) {
	rows, err := db.QueryContext(ctx, `
		SELECT
			albums.album_uid,
			albums.volume_uuid,
			albums.album_title,
			COALESCE(NULLIF(albums.album_artist, ''), '(unknown artist)'),
			COALESCE(albums.year, 0),
			albums.track_count,
			albums.total_duration_ms,
			COALESCE(albums.album_root_dir_hint, ''),
			COALESCE(artwork_refs.thumb_rel_path, ''),
			albums.source_mode
		FROM albums
		JOIN volumes ON volumes.volume_uuid = albums.volume_uuid
		LEFT JOIN artwork_refs ON artwork_refs.artwork_ref_id = albums.cover_ref_id
		WHERE `+visibleVolumePredicate("volumes")+`
		ORDER BY
			COALESCE(NULLIF(albums.album_artist_norm, ''), NULLIF(albums.album_artist, ''), '(unknown artist)') ASC,
			albums.album_title_norm ASC,
			albums.album_uid ASC
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
			&item.VolumeUUID,
			&item.Title,
			&item.AlbumArtist,
			&item.Year,
			&item.TrackCount,
			&item.TotalDurationMS,
			&item.RootDirHint,
			&item.CoverThumbRelPath,
			&item.SourceMode,
		); err != nil {
			return nil, err
		}
		albums = append(albums, item)
	}

	return albums, rows.Err()
}

func queryTracks(ctx context.Context, db *sql.DB, query Query) ([]TrackSummary, error) {
	var (
		rows *sql.Rows
		err  error
	)
	if query.AlbumUID == UncategorizedAlbumUID {
		rows, err = db.QueryContext(ctx, `
		SELECT
			tracks.track_uid,
			COALESCE(albums.album_uid, ''),
			COALESCE(NULLIF(albums.album_title, ''), '(unknown album)'),
			tracks.volume_uuid,
			COALESCE(NULLIF(tracks.title, ''), tracks.filename),
			COALESCE(NULLIF(tracks.artist, ''), NULLIF(tracks.album_artist, ''), '(unknown artist)'),
			tracks.relative_path,
			tracks.track_no,
			tracks.disc_no,
			COALESCE(tracks.format, ''),
			tracks.duration_ms,
			tracks.sample_rate
		FROM tracks
		JOIN albums ON albums.album_id = tracks.album_id
		JOIN volumes ON volumes.volume_uuid = tracks.volume_uuid
		WHERE `+visibleVolumePredicate("volumes")+`
		  AND albums.source_mode = 'directory_fallback'
		  AND albums.track_count = 1
		ORDER BY
			tracks.relative_path ASC,
			tracks.track_uid ASC
	`)
	} else if query.AlbumUID != "" {
		rows, err = db.QueryContext(ctx, `
		SELECT
			tracks.track_uid,
			COALESCE(albums.album_uid, ''),
			COALESCE(NULLIF(albums.album_title, ''), '(unknown album)'),
			tracks.volume_uuid,
			COALESCE(NULLIF(tracks.title, ''), tracks.filename),
			COALESCE(NULLIF(tracks.artist, ''), NULLIF(tracks.album_artist, ''), '(unknown artist)'),
			tracks.relative_path,
			tracks.track_no,
			tracks.disc_no,
			COALESCE(tracks.format, ''),
			tracks.duration_ms,
			tracks.sample_rate
		FROM tracks
		LEFT JOIN albums ON albums.album_id = tracks.album_id
		JOIN volumes ON volumes.volume_uuid = tracks.volume_uuid
		WHERE albums.album_uid = ?1
		  AND `+visibleVolumePredicate("volumes")+`
		ORDER BY
			COALESCE(tracks.disc_no, 0) ASC,
			COALESCE(tracks.track_no, 0) ASC,
			tracks.relative_path ASC,
			tracks.track_uid ASC
	`, query.AlbumUID)
	} else if query.DirectoryVolumeUUID != "" {
		if query.DirectoryPath == "" {
			rows, err = db.QueryContext(ctx, `
			SELECT
				tracks.track_uid,
				COALESCE(albums.album_uid, ''),
				COALESCE(NULLIF(albums.album_title, ''), '(unknown album)'),
				tracks.volume_uuid,
				COALESCE(NULLIF(tracks.title, ''), tracks.filename),
				COALESCE(NULLIF(tracks.artist, ''), NULLIF(tracks.album_artist, ''), '(unknown artist)'),
				tracks.relative_path,
				tracks.track_no,
				tracks.disc_no,
				COALESCE(tracks.format, ''),
				tracks.duration_ms,
				tracks.sample_rate
			FROM tracks
			LEFT JOIN albums ON albums.album_id = tracks.album_id
			JOIN volumes ON volumes.volume_uuid = tracks.volume_uuid
			WHERE tracks.volume_uuid = ?1
			  AND `+visibleVolumePredicate("volumes")+`
			ORDER BY
				tracks.relative_path ASC,
				tracks.track_uid ASC
		`, query.DirectoryVolumeUUID)
		} else {
			rows, err = db.QueryContext(ctx, `
			SELECT
				tracks.track_uid,
				COALESCE(albums.album_uid, ''),
				COALESCE(NULLIF(albums.album_title, ''), '(unknown album)'),
				tracks.volume_uuid,
				COALESCE(NULLIF(tracks.title, ''), tracks.filename),
				COALESCE(NULLIF(tracks.artist, ''), NULLIF(tracks.album_artist, ''), '(unknown artist)'),
				tracks.relative_path,
				tracks.track_no,
				tracks.disc_no,
				COALESCE(tracks.format, ''),
				tracks.duration_ms,
				tracks.sample_rate
			FROM tracks
			LEFT JOIN albums ON albums.album_id = tracks.album_id
			JOIN volumes ON volumes.volume_uuid = tracks.volume_uuid
			WHERE tracks.volume_uuid = ?1
			  AND `+visibleVolumePredicate("volumes")+`
			  AND (tracks.relative_path = ?2 OR tracks.relative_path LIKE ?3)
			ORDER BY
				tracks.relative_path ASC,
				tracks.track_uid ASC
		`, query.DirectoryVolumeUUID, query.DirectoryPath, query.DirectoryPath+"/%")
		}
	} else {
		rows, err = db.QueryContext(ctx, `
		SELECT
			tracks.track_uid,
			COALESCE(albums.album_uid, ''),
			COALESCE(NULLIF(albums.album_title, ''), '(unknown album)'),
			tracks.volume_uuid,
			COALESCE(NULLIF(tracks.title, ''), tracks.filename),
			COALESCE(NULLIF(tracks.artist, ''), NULLIF(tracks.album_artist, ''), '(unknown artist)'),
			tracks.relative_path,
			tracks.track_no,
			tracks.disc_no,
			COALESCE(tracks.format, ''),
			tracks.duration_ms,
			tracks.sample_rate
		FROM tracks
		LEFT JOIN albums ON albums.album_id = tracks.album_id
		JOIN volumes ON volumes.volume_uuid = tracks.volume_uuid
		WHERE `+visibleVolumePredicate("volumes")+`
		ORDER BY
			COALESCE(NULLIF(albums.album_artist_norm, ''), NULLIF(albums.album_artist, ''), NULLIF(tracks.album_artist, ''), NULLIF(tracks.artist, ''), '(unknown artist)') ASC,
			COALESCE(albums.album_title_norm, '') ASC,
			COALESCE(tracks.disc_no, 0) ASC,
			COALESCE(tracks.track_no, 0) ASC,
			tracks.relative_path ASC,
			tracks.track_uid ASC
	`)
	}
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var tracks []TrackSummary
	for rows.Next() {
		var item TrackSummary
		var duration sql.NullInt64
		var sampleRate sql.NullInt64
		var trackNo sql.NullInt64
		var discNo sql.NullInt64
		if err := rows.Scan(
			&item.TrackUID,
			&item.AlbumUID,
			&item.AlbumTitle,
			&item.VolumeUUID,
			&item.Title,
			&item.Artist,
			&item.RelativePath,
			&trackNo,
			&discNo,
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
		if trackNo.Valid {
			value := trackNo.Int64
			item.TrackNo = &value
		}
		if discNo.Valid {
			value := discNo.Int64
			item.DiscNo = &value
		}
		if sampleRate.Valid {
			value := sampleRate.Int64
			item.SampleRate = &value
		}
		tracks = append(tracks, item)
	}

	return tracks, rows.Err()
}

func visibleVolumePredicate(alias string) string {
	return alias + ".mount_path NOT LIKE '/var/lib/lumelo/test-media%'"
}

const (
	visibleVolumeCountSQL = `
		SELECT COUNT(*)
		FROM volumes
		WHERE ` + "volumes.mount_path NOT LIKE '/var/lib/lumelo/test-media%'" + `
		  AND EXISTS (
			SELECT 1 FROM tracks
			WHERE tracks.volume_uuid = volumes.volume_uuid
		  )
	`
	visibleAlbumCountSQL = `
		SELECT COUNT(*)
		FROM albums
		JOIN volumes ON volumes.volume_uuid = albums.volume_uuid
		WHERE ` + "volumes.mount_path NOT LIKE '/var/lib/lumelo/test-media%'" + `
	`
	visibleTrackCountSQL = `
		SELECT COUNT(*)
		FROM tracks
		JOIN volumes ON volumes.volume_uuid = tracks.volume_uuid
		WHERE ` + "volumes.mount_path NOT LIKE '/var/lib/lumelo/test-media%'" + `
	`
)
