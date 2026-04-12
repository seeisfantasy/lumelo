package libraryclient

import (
	"context"
	"database/sql"
	"path/filepath"
	"testing"

	_ "github.com/mattn/go-sqlite3"
)

func TestSnapshotReadsLibraryOverview(t *testing.T) {
	dbPath := filepath.Join(t.TempDir(), "library.db")
	db, err := sql.Open("sqlite3", dbPath)
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
