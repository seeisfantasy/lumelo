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
	if len(snapshot.Tracks) != 2 || snapshot.Tracks[0].Title != "Opening" {
		t.Fatalf("unexpected tracks: %+v", snapshot.Tracks)
	}
	if snapshot.Tracks[0].AlbumUID != "album-001" || snapshot.Tracks[0].VolumeUUID != "vol-001" {
		t.Fatalf("unexpected track context: %+v", snapshot.Tracks[0])
	}
	if snapshot.Tracks[0].TrackNo == nil || *snapshot.Tracks[0].TrackNo != 1 {
		t.Fatalf("expected ordered track number, got %+v", snapshot.Tracks[0])
	}
}

func TestQuerySnapshotFiltersTracksByAlbum(t *testing.T) {
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
		INSERT INTO albums (
			album_id, album_uid, album_title, album_artist, album_artist_norm, year, track_count,
			total_duration_ms, album_root_dir_hint, cover_ref_id, indexed_at, album_title_norm, volume_uuid, source_mode
		) VALUES
			(1, 'album-001', 'Blue Room Sessions', 'Demo Artist', 'demo artist', 2024, 2, 481000, '/Albums/Blue Room Sessions', NULL, 1710000100, 'blue room sessions', 'vol-001', 'tag'),
			(2, 'album-002', 'Second Album', 'Demo Artist', 'demo artist', 2025, 1, 201000, '/Albums/Second Album', NULL, 1710000200, 'second album', 'vol-001', 'tag');
		INSERT INTO tracks (
			track_uid, album_id, volume_uuid, title, filename, artist, album_artist, relative_path,
			track_no, disc_no, format, duration_ms, sample_rate, indexed_at
		) VALUES
			('track-001', 1, 'vol-001', 'Opening', '01-opening.flac', 'Demo Artist', 'Demo Artist', '/Albums/Blue Room Sessions/01-opening.flac', 1, 1, 'flac', 201000, 44100, 1710000101),
			('track-002', 1, 'vol-001', 'Night Signal', '02-night-signal.flac', 'Demo Artist', 'Demo Artist', '/Albums/Blue Room Sessions/02-night-signal.flac', 2, 1, 'flac', 280000, 44100, 1710000102),
			('track-003', 2, 'vol-001', 'Third Song', '01-third.flac', 'Demo Artist', 'Demo Artist', '/Albums/Second Album/01-third.flac', 1, 1, 'flac', 221000, 44100, 1710000201);
		INSERT INTO artists (artist_id, artist_name) VALUES (1, 'Demo Artist');
		INSERT INTO genres (genre_id, genre_name) VALUES (1, 'Ambient');
	`); err != nil {
		t.Fatalf("seed library rows: %v", err)
	}

	snapshot := New(dbPath).QuerySnapshot(context.Background(), Query{AlbumUID: "album-001"})
	if !snapshot.Available {
		t.Fatalf("expected filtered snapshot to be available, got error: %s", snapshot.Error)
	}
	if len(snapshot.Tracks) != 2 {
		t.Fatalf("expected only album tracks, got %+v", snapshot.Tracks)
	}
	if snapshot.Tracks[0].TrackUID != "track-001" || snapshot.Tracks[1].TrackUID != "track-002" {
		t.Fatalf("unexpected filtered ordering: %+v", snapshot.Tracks)
	}
	if snapshot.Query.AlbumUID != "album-001" {
		t.Fatalf("expected snapshot query to round-trip, got %+v", snapshot.Query)
	}
}

func TestSnapshotHidesInternalTestVolumes(t *testing.T) {
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
		CREATE TABLE artists (artist_id INTEGER PRIMARY KEY, artist_name TEXT NOT NULL);
		CREATE TABLE genres (genre_id INTEGER PRIMARY KEY, genre_name TEXT NOT NULL);
	`); err != nil {
		t.Fatalf("create library schema: %v", err)
	}

	if _, err := db.Exec(`
		INSERT INTO volumes (volume_uuid, label, mount_path, is_available, last_seen_at) VALUES
			('vol-user', 'USB', '/media/usb', 1, 10),
			('vol-test', 'test-media-bad', '/var/lib/lumelo/test-media-bad', 1, 9);
		INSERT INTO albums (album_id, album_uid, album_title, album_artist, album_artist_norm, year, track_count, total_duration_ms, album_root_dir_hint, cover_ref_id, indexed_at, album_title_norm, volume_uuid, source_mode) VALUES
			(1, 'album-user', 'User Album', 'Artist', 'artist', 2024, 1, 1000, 'User Album', NULL, 10, 'user album', 'vol-user', 'tag'),
			(2, 'album-test', 'Bad Inputs', 'Unknown Artist', 'unknown artist', 2024, 1, 1000, 'Bad Inputs', NULL, 10, 'bad inputs', 'vol-test', 'directory_fallback');
		INSERT INTO tracks (track_uid, album_id, volume_uuid, title, filename, artist, album_artist, relative_path, track_no, disc_no, format, duration_ms, sample_rate, indexed_at) VALUES
			('track-user', 1, 'vol-user', 'Song', 'song.flac', 'Artist', 'Artist', 'User Album/song.flac', 1, 1, 'flac', 1000, 44100, 10),
			('track-test', 2, 'vol-test', 'Noise', 'noise.flac', 'Unknown Artist', 'Unknown Artist', 'Bad Inputs/noise.flac', 1, 1, 'flac', 1000, 44100, 10);
	`); err != nil {
		t.Fatalf("seed library rows: %v", err)
	}

	snapshot := New(dbPath).Snapshot(context.Background())
	if snapshot.Stats.VolumeCount != 1 || snapshot.Stats.AlbumCount != 1 || snapshot.Stats.TrackCount != 1 {
		t.Fatalf("unexpected visible stats: %+v", snapshot.Stats)
	}
	if len(snapshot.Volumes) != 1 || snapshot.Volumes[0].VolumeUUID != "vol-user" {
		t.Fatalf("unexpected visible volumes: %+v", snapshot.Volumes)
	}
	if len(snapshot.Albums) != 1 || snapshot.Albums[0].AlbumUID != "album-user" {
		t.Fatalf("unexpected visible albums: %+v", snapshot.Albums)
	}
	if len(snapshot.Tracks) != 1 || snapshot.Tracks[0].TrackUID != "track-user" {
		t.Fatalf("unexpected visible tracks: %+v", snapshot.Tracks)
	}
}

func TestQuerySnapshotSupportsDirectoryFilter(t *testing.T) {
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
		CREATE TABLE artists (artist_id INTEGER PRIMARY KEY, artist_name TEXT NOT NULL);
		CREATE TABLE genres (genre_id INTEGER PRIMARY KEY, genre_name TEXT NOT NULL);
	`); err != nil {
		t.Fatalf("create library schema: %v", err)
	}

	if _, err := db.Exec(`
		INSERT INTO volumes (volume_uuid, label, mount_path, is_available, last_seen_at)
		VALUES ('vol-001', 'Demo TF', '/media/demo', 1, 1710000000);
		INSERT INTO directories (directory_id, volume_uuid, relative_path, parent_relative_path, display_name, indexed_at) VALUES
			(1, 'vol-001', 'OST', '', 'OST', 1),
			(2, 'vol-001', 'OST/Disc 01', 'OST', 'Disc 01', 1),
			(3, 'vol-001', 'Classical', '', 'Classical', 1);
		INSERT INTO albums (album_id, album_uid, album_title, album_artist, album_artist_norm, year, track_count, total_duration_ms, album_root_dir_hint, cover_ref_id, indexed_at, album_title_norm, volume_uuid, source_mode) VALUES
			(1, 'album-001', 'Blue Room Sessions', 'Demo Artist', 'demo artist', 2024, 2, 481000, 'OST/Disc 01', NULL, 1710000100, 'blue room sessions', 'vol-001', 'tag');
		INSERT INTO tracks (track_uid, album_id, volume_uuid, title, filename, artist, album_artist, relative_path, track_no, disc_no, format, duration_ms, sample_rate, indexed_at) VALUES
			('track-001', 1, 'vol-001', 'Opening', '01-opening.flac', 'Demo Artist', 'Demo Artist', 'OST/Disc 01/01-opening.flac', 1, 1, 'flac', 201000, 44100, 1710000101),
			('track-002', 1, 'vol-001', 'Night Signal', '02-night-signal.flac', 'Demo Artist', 'Demo Artist', 'OST/Disc 01/02-night-signal.flac', 2, 1, 'flac', 280000, 44100, 1710000102),
			('track-003', 1, 'vol-001', 'Other', '03-other.flac', 'Demo Artist', 'Demo Artist', 'Classical/03-other.flac', 3, 1, 'flac', 111000, 44100, 1710000103);
	`); err != nil {
		t.Fatalf("seed library rows: %v", err)
	}

	snapshot := New(dbPath).QuerySnapshot(context.Background(), Query{
		DirectoryVolumeUUID: "vol-001",
		DirectoryPath:       "OST",
	})
	if len(snapshot.Directories) != 1 || snapshot.Directories[0].RelativePath != "OST/Disc 01" {
		t.Fatalf("unexpected directories: %+v", snapshot.Directories)
	}
	if len(snapshot.Tracks) != 2 || snapshot.Tracks[0].TrackUID != "track-001" || snapshot.Tracks[1].TrackUID != "track-002" {
		t.Fatalf("unexpected filtered tracks: %+v", snapshot.Tracks)
	}
	if snapshot.Query.DirectoryVolumeUUID != "vol-001" || snapshot.Query.DirectoryPath != "OST" {
		t.Fatalf("unexpected query round-trip: %+v", snapshot.Query)
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
