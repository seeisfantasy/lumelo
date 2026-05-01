use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use artwork_cache::{source_path, thumb_path};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::ImageReader;
use ipc_proto::{cache_dir_path, state_dir_path};
use lofty::prelude::{Accessor, AudioFile, ItemKey, TaggedFileExt};
use lofty::probe::Probe;
use rusqlite::{params, Connection, Transaction};

#[cfg(test)]
const DEFAULT_LIBRARY_DB_PATH: &str = "/var/lib/lumelo/library.db";
const LIBRARY_SCHEMA_VERSION: &str = "1";
const UNKNOWN_ARTIST: &str = "Unknown Artist";
const CACHE_VERSION: u32 = 1;
const THUMB_MAX_EDGE_PX: u32 = 320;
const THUMB_JPEG_QUALITY: u8 = 85;
const SUPPORTED_COVER_FILENAMES: &[&str] = &["folder.jpg", "cover.jpg"];
const SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &[
    "aac", "aif", "aiff", "alac", "ape", "dff", "dsf", "flac", "m4a", "mp3", "ogg", "opus", "wav",
    "wma",
];

fn main() {
    if let Err(err) = run() {
        eprintln!("media-indexd failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_env_and_args()?;
    let summary = execute(config)?;

    println!("media-indexd ready");
    println!("  library db:     {}", summary.db_path.display());
    println!("  command:        {}", summary.command_name);
    println!("  volumes:        {}", summary.volume_count);
    println!("  directories:    {}", summary.directory_count);
    println!("  albums:         {}", summary.album_count);
    println!("  tracks:         {}", summary.track_count);
    println!("  artwork refs:   {}", summary.artwork_ref_count);

    Ok(())
}

#[derive(Debug, Clone)]
struct Config {
    command: MediaIndexCommand,
    db_path: PathBuf,
    artwork_cache_dir: PathBuf,
}

impl Config {
    fn from_env_and_args() -> Result<Self, String> {
        let mut args = env::args().skip(1);
        let command = match args.next().as_deref() {
            None => MediaIndexCommand::EnsureSchema,
            Some("ensure-schema") => MediaIndexCommand::EnsureSchema,
            Some("seed-demo") => MediaIndexCommand::SeedDemo,
            Some("scan-dir") => {
                let scan_root = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| "scan-dir requires a directory path".to_string())?;
                MediaIndexCommand::ScanDir { scan_root }
            }
            Some(other) => {
                return Err(format!(
                    "unsupported command: {other} (expected ensure-schema, seed-demo, or scan-dir)"
                ))
            }
        };

        if let Some(extra) = args.next() {
            return Err(format!("unexpected extra argument: {extra}"));
        }

        Ok(Self {
            command,
            db_path: library_db_path_from_env(),
            artwork_cache_dir: artwork_cache_dir_from_env(),
        })
    }
}

#[derive(Debug, Clone)]
enum MediaIndexCommand {
    EnsureSchema,
    SeedDemo,
    ScanDir { scan_root: PathBuf },
}

impl MediaIndexCommand {
    fn as_str(&self) -> &'static str {
        match self {
            Self::EnsureSchema => "ensure-schema",
            Self::SeedDemo => "seed-demo",
            Self::ScanDir { .. } => "scan-dir",
        }
    }
}

#[derive(Debug)]
struct LibrarySummary {
    db_path: PathBuf,
    command_name: &'static str,
    volume_count: i64,
    directory_count: i64,
    album_count: i64,
    track_count: i64,
    artwork_ref_count: i64,
}

#[derive(Debug, Clone)]
struct VolumeDescriptor {
    volume_uuid: String,
    label: String,
    mount_path: String,
    fs_type: Option<String>,
}

#[derive(Debug, Clone)]
struct ScannedDirectory {
    relative_path: String,
    parent_relative_path: Option<String>,
    display_name: String,
}

#[derive(Debug, Clone)]
struct ScannedAlbum {
    album_uid: String,
    volume_uuid: String,
    album_title: String,
    album_title_norm: String,
    album_artist: String,
    album_artist_norm: String,
    album_root_dir_hint: String,
    year: Option<i64>,
    disc_count: i64,
    track_count: i64,
    total_duration_ms: i64,
    cover_ref_id: Option<i64>,
    source_mode: String,
    indexed_at: i64,
}

#[derive(Debug, Clone)]
struct ScannedTrack {
    track_uid: String,
    album_uid: String,
    volume_uuid: String,
    relative_path: String,
    filename: String,
    title: String,
    artist: String,
    album_artist: String,
    track_no: Option<i64>,
    disc_no: i64,
    duration_ms: Option<i64>,
    sample_rate: Option<i64>,
    bit_depth: Option<i64>,
    format: String,
    cover_ref_id: Option<i64>,
    genres: Vec<String>,
    musicbrainz_track_id: Option<String>,
    file_mtime: i64,
    indexed_at: i64,
}

#[derive(Debug)]
struct ScanResult {
    volume: VolumeDescriptor,
    directories: Vec<ScannedDirectory>,
    albums: Vec<ScannedAlbum>,
    tracks: Vec<ScannedTrack>,
    artwork_refs: Vec<ScannedArtworkRef>,
    album_artwork_keys: BTreeMap<String, String>,
    artist_names: Vec<String>,
    genre_names: Vec<String>,
    search_entries: Vec<SearchEntry>,
}

#[derive(Debug, Clone)]
struct SearchEntry {
    doc_type: &'static str,
    doc_id: String,
    content: String,
}

#[derive(Debug, Clone)]
struct FileScanRecord {
    relative_path: String,
    filename: String,
    derived_title: String,
    derived_track_no: Option<i64>,
    directory_disc_no: i64,
    format: String,
    file_mtime: i64,
    indexed_at: i64,
    size: u64,
    album_root_dir_hint: String,
    metadata: ParsedAudioMetadata,
}

#[derive(Debug, Default)]
struct AlbumAccumulator {
    album_title: String,
    album_title_norm: String,
    album_artist: String,
    album_artist_norm: String,
    album_root_dir_hint: String,
    year: Option<i64>,
    disc_count: i64,
    track_count: i64,
    total_duration_ms: i64,
    source_mode: String,
}

#[derive(Debug, Clone, Default)]
struct ParsedAudioMetadata {
    title: Option<String>,
    artist: Option<String>,
    album_artist: Option<String>,
    album_title: Option<String>,
    track_no: Option<i64>,
    disc_no: Option<i64>,
    year: Option<i64>,
    genres: Vec<String>,
    duration_ms: Option<i64>,
    sample_rate: Option<i64>,
    bit_depth: Option<i64>,
}

#[derive(Debug, Clone)]
struct GroupAlbumContext {
    album_title: String,
    album_artist: String,
    source_mode: String,
    year: Option<i64>,
}

#[derive(Debug, Clone)]
struct ResolvedTrackRecord {
    album_uid: String,
    album_title: String,
    album_artist: String,
    album_root_dir_hint: String,
    source_mode: String,
    year: Option<i64>,
    title: String,
    artist: String,
    track_no: Option<i64>,
    disc_no: i64,
    duration_ms: Option<i64>,
    sample_rate: Option<i64>,
    bit_depth: Option<i64>,
    genres: Vec<String>,
}

#[derive(Debug, Clone)]
struct ScannedArtworkRef {
    content_hash: String,
    mime_type: String,
    extension: String,
    source_file_relative_path: String,
    source_file_abs_path: PathBuf,
    indexed_at: i64,
}

#[derive(Debug, Clone)]
struct GeneratedThumbRecord {
    width: i64,
    height: i64,
    thumb_rel_path: String,
}

fn execute(config: Config) -> Result<LibrarySummary, String> {
    ensure_parent_dir(&config.db_path, "library db")?;
    fs::create_dir_all(&config.artwork_cache_dir).map_err(|err| {
        format!(
            "create artwork cache dir {}: {err}",
            config.artwork_cache_dir.display()
        )
    })?;

    let mut connection = Connection::open(&config.db_path)
        .map_err(|err| format!("open library db {}: {err}", config.db_path.display()))?;
    configure_connection(&connection)?;
    ensure_schema(&connection)?;

    match &config.command {
        MediaIndexCommand::EnsureSchema => {}
        MediaIndexCommand::SeedDemo => {
            seed_demo_library(&mut connection, &config.artwork_cache_dir)?
        }
        MediaIndexCommand::ScanDir { scan_root } => {
            scan_directory_library(&mut connection, scan_root, &config.artwork_cache_dir)?
        }
    }

    summarize_library(&connection, &config.db_path, config.command.as_str())
}

fn configure_connection(connection: &Connection) -> Result<(), String> {
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|err| format!("enable foreign_keys: {err}"))?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|err| format!("set journal_mode: {err}"))?;

    Ok(())
}

fn ensure_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS volumes (
                volume_uuid TEXT PRIMARY KEY,
                label TEXT,
                mount_path TEXT NOT NULL,
                fs_type TEXT,
                is_available INTEGER NOT NULL DEFAULT 1,
                last_seen_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS directories (
                directory_id INTEGER PRIMARY KEY AUTOINCREMENT,
                volume_uuid TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                parent_relative_path TEXT,
                display_name TEXT NOT NULL,
                indexed_at INTEGER NOT NULL,
                UNIQUE(volume_uuid, relative_path)
            );

            CREATE TABLE IF NOT EXISTS artwork_refs (
                artwork_ref_id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_hash TEXT NOT NULL UNIQUE,
                mime_type TEXT,
                width INTEGER,
                height INTEGER,
                source_rel_path TEXT,
                thumb_rel_path TEXT,
                indexed_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS albums (
                album_id INTEGER PRIMARY KEY AUTOINCREMENT,
                album_uid TEXT NOT NULL UNIQUE,
                volume_uuid TEXT NOT NULL,
                album_title TEXT NOT NULL,
                album_title_norm TEXT NOT NULL,
                album_artist TEXT,
                album_artist_norm TEXT,
                album_root_dir_hint TEXT,
                year INTEGER,
                disc_count INTEGER NOT NULL DEFAULT 1,
                track_count INTEGER NOT NULL DEFAULT 0,
                total_duration_ms INTEGER NOT NULL DEFAULT 0,
                cover_ref_id INTEGER,
                musicbrainz_release_id TEXT,
                source_mode TEXT NOT NULL DEFAULT 'folder',
                indexed_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tracks (
                track_uid TEXT PRIMARY KEY,
                album_id INTEGER,
                volume_uuid TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                filename TEXT NOT NULL,
                title TEXT,
                artist TEXT,
                album_artist TEXT,
                track_no INTEGER,
                disc_no INTEGER,
                duration_ms INTEGER,
                sample_rate INTEGER,
                bit_depth INTEGER,
                format TEXT,
                cover_ref_id INTEGER,
                musicbrainz_track_id TEXT,
                file_mtime INTEGER,
                indexed_at INTEGER NOT NULL,
                UNIQUE(volume_uuid, relative_path)
            );

            CREATE TABLE IF NOT EXISTS artists (
                artist_id INTEGER PRIMARY KEY AUTOINCREMENT,
                artist_name TEXT NOT NULL,
                artist_name_norm TEXT NOT NULL,
                UNIQUE(artist_name_norm)
            );

            CREATE TABLE IF NOT EXISTS genres (
                genre_id INTEGER PRIMARY KEY AUTOINCREMENT,
                genre_name TEXT NOT NULL,
                genre_name_norm TEXT NOT NULL,
                UNIQUE(genre_name_norm)
            );

            CREATE TABLE IF NOT EXISTS album_artists (
                album_id INTEGER NOT NULL,
                artist_id INTEGER NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY(album_id, artist_id)
            );

            CREATE TABLE IF NOT EXISTS track_artists (
                track_uid TEXT NOT NULL,
                artist_id INTEGER NOT NULL,
                role TEXT NOT NULL DEFAULT 'performer',
                sort_order INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY(track_uid, artist_id, role)
            );

            CREATE TABLE IF NOT EXISTS track_genres (
                track_uid TEXT NOT NULL,
                genre_id INTEGER NOT NULL,
                PRIMARY KEY(track_uid, genre_id)
            );

            CREATE TABLE IF NOT EXISTS search_fts (
                doc_type TEXT NOT NULL,
                doc_id TEXT NOT NULL,
                content TEXT NOT NULL,
                PRIMARY KEY(doc_type, doc_id)
            );

            CREATE INDEX IF NOT EXISTS idx_directories_volume_path
            ON directories (volume_uuid, relative_path);

            CREATE INDEX IF NOT EXISTS idx_albums_volume_uuid
            ON albums (volume_uuid, album_title_norm);

            CREATE INDEX IF NOT EXISTS idx_tracks_album_id
            ON tracks (album_id, disc_no, track_no);

            CREATE INDEX IF NOT EXISTS idx_tracks_volume_path
            ON tracks (volume_uuid, relative_path);
            ",
        )
        .map_err(|err| format!("ensure library schema: {err}"))?;

    connection
        .execute(
            "INSERT INTO schema_meta (key, value) VALUES ('library_schema_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![LIBRARY_SCHEMA_VERSION],
        )
        .map_err(|err| format!("store schema version: {err}"))?;

    Ok(())
}

fn seed_demo_library(connection: &mut Connection, artwork_cache_dir: &Path) -> Result<(), String> {
    let tx = connection
        .transaction()
        .map_err(|err| format!("start seed transaction: {err}"))?;
    let indexed_at = unix_timestamp_secs() as i64;

    tx.execute(
        "INSERT INTO volumes (volume_uuid, label, mount_path, fs_type, is_available, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5)
         ON CONFLICT(volume_uuid) DO UPDATE SET
           label = excluded.label,
           mount_path = excluded.mount_path,
           fs_type = excluded.fs_type,
           is_available = excluded.is_available,
           last_seen_at = excluded.last_seen_at",
        params![
            "demo-volume",
            "Demo TF Card",
            "/media/demo-volume",
            "exfat",
            indexed_at
        ],
    )
    .map_err(|err| format!("seed volume: {err}"))?;

    seed_directory(&tx, "demo-volume", "", None, "Demo TF Card", indexed_at)?;
    seed_directory(
        &tx,
        "demo-volume",
        "Demo Album",
        Some(""),
        "Demo Album",
        indexed_at,
    )?;

    let cover_ref_id = seed_artwork_ref(&tx, artwork_cache_dir, indexed_at)?;
    let album_id = seed_album(&tx, cover_ref_id, indexed_at)?;
    seed_track(
        &tx,
        "demo-track-001",
        album_id,
        "demo-volume",
        "Demo Album/01 - Opening.flac",
        "01 - Opening.flac",
        "Opening",
        "Demo Artist",
        1,
        1,
        182_000,
        44_100,
        16,
        "flac",
        cover_ref_id,
        indexed_at,
    )?;
    seed_track(
        &tx,
        "demo-track-002",
        album_id,
        "demo-volume",
        "Demo Album/02 - Afterglow.flac",
        "02 - Afterglow.flac",
        "Afterglow",
        "Demo Artist",
        2,
        1,
        205_000,
        44_100,
        16,
        "flac",
        cover_ref_id,
        indexed_at,
    )?;

    seed_artist(&tx, "Demo Artist")?;
    seed_genre(&tx, "Ambient")?;
    seed_search_entry(
        &tx,
        "album",
        "demo-album",
        "Demo Album Demo Artist Demo Album",
    )?;
    seed_search_entry(
        &tx,
        "track",
        "demo-track-001",
        "Opening Demo Artist Demo Album 01 - Opening.flac",
    )?;
    seed_search_entry(
        &tx,
        "track",
        "demo-track-002",
        "Afterglow Demo Artist Demo Album 02 - Afterglow.flac",
    )?;

    tx.commit()
        .map_err(|err| format!("commit seed transaction: {err}"))?;

    Ok(())
}

fn scan_directory_library(
    connection: &mut Connection,
    scan_root: &Path,
    artwork_cache_dir: &Path,
) -> Result<(), String> {
    let indexed_at = unix_timestamp_secs() as i64;
    let mut scan_result = collect_scan_result(scan_root, indexed_at)?;

    let tx = connection
        .transaction()
        .map_err(|err| format!("start scan transaction: {err}"))?;

    clear_volume_rows(&tx, &scan_result.volume.volume_uuid)?;
    upsert_volume(&tx, &scan_result.volume, indexed_at)?;

    for directory in &scan_result.directories {
        seed_directory(
            &tx,
            &scan_result.volume.volume_uuid,
            &directory.relative_path,
            directory.parent_relative_path.as_deref(),
            &directory.display_name,
            indexed_at,
        )?;
    }

    let mut artwork_ids = BTreeMap::new();
    for artwork_ref in &scan_result.artwork_refs {
        let artwork_ref_id = upsert_artwork_ref(&tx, artwork_ref, artwork_cache_dir)?;
        artwork_ids.insert(artwork_ref.content_hash.clone(), artwork_ref_id);
    }

    let album_artwork_keys = scan_result.album_artwork_keys.clone();
    for album in &mut scan_result.albums {
        album.cover_ref_id = album_artwork_keys
            .get(&album.album_uid)
            .and_then(|artwork_key| artwork_ids.get(artwork_key))
            .copied();
    }
    for track in &mut scan_result.tracks {
        track.cover_ref_id = album_artwork_keys
            .get(&track.album_uid)
            .and_then(|artwork_key| artwork_ids.get(artwork_key))
            .copied();
    }

    let mut artist_ids = BTreeMap::new();
    for artist_name in &scan_result.artist_names {
        let artist_id = seed_artist_and_load_id(&tx, artist_name)?;
        artist_ids.insert(normalize_text(artist_name), artist_id);
    }

    let mut genre_ids = BTreeMap::new();
    for genre_name in &scan_result.genre_names {
        let genre_id = seed_genre_and_load_id(&tx, genre_name)?;
        genre_ids.insert(normalize_text(genre_name), genre_id);
    }

    let mut album_ids = BTreeMap::new();
    for album in &scan_result.albums {
        let album_id = upsert_album(&tx, album)?;
        album_ids.insert(album.album_uid.clone(), album_id);

        if let Some(artist_id) = artist_ids.get(&album.album_artist_norm) {
            upsert_album_artist(&tx, album_id, *artist_id)?;
        }
    }

    for track in &scan_result.tracks {
        let album_id = album_ids
            .get(&track.album_uid)
            .copied()
            .ok_or_else(|| format!("missing album id for {}", track.album_uid))?;
        upsert_track(&tx, track, album_id)?;

        let artist_key = normalize_text(&track.artist);
        if let Some(artist_id) = artist_ids.get(&artist_key) {
            upsert_track_artist(&tx, &track.track_uid, *artist_id)?;
        }

        for genre_name in &track.genres {
            let genre_key = normalize_text(genre_name);
            if let Some(genre_id) = genre_ids.get(&genre_key) {
                upsert_track_genre(&tx, &track.track_uid, *genre_id)?;
            }
        }
    }

    for entry in &scan_result.search_entries {
        seed_search_entry(&tx, entry.doc_type, &entry.doc_id, &entry.content)?;
    }

    delete_orphan_artists(&tx)?;
    delete_orphan_genres(&tx)?;
    delete_orphan_artwork_refs(&tx)?;

    tx.commit()
        .map_err(|err| format!("commit scan transaction: {err}"))?;

    Ok(())
}

fn collect_scan_result(scan_root: &Path, indexed_at: i64) -> Result<ScanResult, String> {
    let canonical_root = fs::canonicalize(scan_root)
        .map_err(|err| format!("canonicalize scan root {}: {err}", scan_root.display()))?;
    if !canonical_root.is_dir() {
        return Err(format!(
            "scan root is not a directory: {}",
            canonical_root.display()
        ));
    }

    let volume = build_volume_descriptor(&canonical_root)?;
    let mut directories = BTreeMap::new();
    directories.insert(
        String::new(),
        ScannedDirectory {
            relative_path: String::new(),
            parent_relative_path: None,
            display_name: volume.label.clone(),
        },
    );

    let mut files = Vec::new();
    collect_entries(
        &canonical_root,
        &canonical_root,
        &mut directories,
        &mut files,
        indexed_at,
    )?;
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let mut grouped_files = BTreeMap::<(String, String), Vec<FileScanRecord>>::new();
    for file in files {
        grouped_files
            .entry((file.album_root_dir_hint.clone(), file_grouping_key(&file)))
            .or_default()
            .push(file);
    }

    let mut albums = BTreeMap::new();
    let mut tracks = Vec::new();
    let mut artwork_refs = BTreeMap::new();
    let mut album_artwork_keys = BTreeMap::new();
    let mut artist_names = BTreeSet::new();
    let mut genre_names = BTreeSet::new();
    let mut search_entries = Vec::new();

    for ((album_root_dir_hint, _), mut group_files) in grouped_files {
        group_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        let group_context =
            resolve_group_album_context(&group_files, &volume.label, &album_root_dir_hint);
        let artwork_ref =
            discover_directory_artwork(&canonical_root, &volume, &album_root_dir_hint, indexed_at)?;
        let artwork_key = artwork_ref
            .as_ref()
            .map(|artwork| artwork.content_hash.clone());
        if let Some(artwork_ref) = artwork_ref {
            artwork_refs.insert(artwork_ref.content_hash.clone(), artwork_ref);
        }

        for file in group_files {
            let resolved = resolve_track_record(&volume, &file, &group_context);
            let album_artist_norm = normalize_text(&resolved.album_artist);
            let album_uid = resolved.album_uid.clone();
            let track_uid = stable_id(&format!(
                "file\n{}\n{}\n{}\n{}\n0\n{}",
                volume.volume_uuid, file.relative_path, file.file_mtime, file.size, CACHE_VERSION
            ));

            let album = albums
                .entry(album_uid.clone())
                .or_insert_with(|| AlbumAccumulator {
                    album_title: resolved.album_title.clone(),
                    album_title_norm: normalize_text(&resolved.album_title),
                    album_artist: resolved.album_artist.clone(),
                    album_artist_norm: album_artist_norm.clone(),
                    album_root_dir_hint: resolved.album_root_dir_hint.clone(),
                    year: resolved.year,
                    disc_count: resolved.disc_no,
                    track_count: 0,
                    total_duration_ms: 0,
                    source_mode: resolved.source_mode.clone(),
                });
            if let Some(artwork_key) = &artwork_key {
                album_artwork_keys
                    .entry(album_uid.clone())
                    .or_insert_with(|| artwork_key.clone());
            }
            album.track_count += 1;
            album.disc_count = album.disc_count.max(resolved.disc_no);
            album.total_duration_ms += resolved.duration_ms.unwrap_or(0);
            if album.year.is_none() {
                album.year = resolved.year;
            }

            artist_names.insert(resolved.album_artist.clone());
            if !resolved.artist.is_empty() {
                artist_names.insert(resolved.artist.clone());
            }
            for genre_name in &resolved.genres {
                genre_names.insert(genre_name.clone());
            }

            search_entries.push(SearchEntry {
                doc_type: "track",
                doc_id: track_uid.clone(),
                content: build_track_search_content(&file, &resolved),
            });

            tracks.push(ScannedTrack {
                track_uid,
                album_uid,
                volume_uuid: volume.volume_uuid.clone(),
                relative_path: file.relative_path,
                filename: file.filename,
                title: resolved.title,
                artist: resolved.artist,
                album_artist: resolved.album_artist,
                track_no: resolved.track_no,
                disc_no: resolved.disc_no,
                duration_ms: resolved.duration_ms,
                sample_rate: resolved.sample_rate,
                bit_depth: resolved.bit_depth,
                format: file.format,
                cover_ref_id: None,
                genres: resolved.genres,
                musicbrainz_track_id: None,
                file_mtime: file.file_mtime,
                indexed_at: file.indexed_at,
            });
        }
    }

    let mut scanned_albums = Vec::new();
    for (album_uid, album) in albums {
        search_entries.push(SearchEntry {
            doc_type: "album",
            doc_id: album_uid.clone(),
            content: format!(
                "{} {} {}",
                album.album_title, album.album_artist, album.album_root_dir_hint
            ),
        });
        scanned_albums.push(ScannedAlbum {
            album_uid,
            volume_uuid: volume.volume_uuid.clone(),
            album_title: album.album_title,
            album_title_norm: album.album_title_norm,
            album_artist: album.album_artist,
            album_artist_norm: album.album_artist_norm,
            album_root_dir_hint: album.album_root_dir_hint,
            year: album.year,
            disc_count: album.disc_count.max(1),
            track_count: album.track_count,
            total_duration_ms: album.total_duration_ms,
            cover_ref_id: None,
            source_mode: album.source_mode,
            indexed_at,
        });
    }

    Ok(ScanResult {
        volume,
        directories: directories.into_values().collect(),
        albums: scanned_albums,
        tracks,
        artwork_refs: artwork_refs.into_values().collect(),
        album_artwork_keys,
        artist_names: artist_names.into_iter().collect(),
        genre_names: genre_names.into_iter().collect(),
        search_entries,
    })
}

fn build_volume_descriptor(scan_root: &Path) -> Result<VolumeDescriptor, String> {
    let mount_path = scan_root.to_string_lossy().to_string();
    let derived_label = scan_root
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("Scanned Library")
        .to_string();
    let label = env::var("MEDIA_INDEX_VOLUME_LABEL").unwrap_or(derived_label);
    let volume_uuid = env::var("MEDIA_INDEX_VOLUME_UUID")
        .unwrap_or_else(|_| format!("scan-{}", stable_id(&mount_path)));
    let fs_type = env::var("MEDIA_INDEX_FS_TYPE").ok();

    Ok(VolumeDescriptor {
        volume_uuid,
        label,
        mount_path,
        fs_type,
    })
}

fn resolve_group_album_context(
    files: &[FileScanRecord],
    volume_label: &str,
    album_root_dir_hint: &str,
) -> GroupAlbumContext {
    let mut tagged_albums =
        BTreeMap::<(String, String), (usize, String, String, Option<i64>)>::new();

    for file in files {
        let album_title = clean_optional_text(file.metadata.album_title.as_deref());
        let album_artist = file
            .metadata
            .album_artist
            .as_deref()
            .and_then(|value| clean_optional_text(Some(value)))
            .or_else(|| {
                file.metadata
                    .artist
                    .as_deref()
                    .and_then(|value| clean_optional_text(Some(value)))
            });

        if let (Some(album_title), Some(album_artist)) = (album_title, album_artist) {
            let key = (normalize_text(&album_artist), normalize_text(&album_title));
            let entry = tagged_albums.entry(key).or_insert_with(|| {
                (
                    0,
                    album_title.clone(),
                    album_artist.clone(),
                    file.metadata.year,
                )
            });
            entry.0 += 1;
            if entry.3.is_none() {
                entry.3 = file.metadata.year;
            }
        }
    }

    if let Some((_, (_, album_title, album_artist, year))) =
        tagged_albums.into_iter().max_by(|left, right| {
            left.1
                 .0
                .cmp(&right.1 .0)
                .then_with(|| left.0.cmp(&right.0))
        })
    {
        return GroupAlbumContext {
            album_title,
            album_artist,
            source_mode: "tag".to_string(),
            year,
        };
    }

    GroupAlbumContext {
        album_title: album_title_for_root(album_root_dir_hint, volume_label),
        album_artist: UNKNOWN_ARTIST.to_string(),
        source_mode: "directory_fallback".to_string(),
        year: None,
    }
}

fn resolve_track_record(
    volume: &VolumeDescriptor,
    file: &FileScanRecord,
    group_context: &GroupAlbumContext,
) -> ResolvedTrackRecord {
    let track_artist = clean_optional_text(file.metadata.artist.as_deref())
        .or_else(|| {
            if group_context.source_mode == "tag" {
                Some(group_context.album_artist.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| UNKNOWN_ARTIST.to_string());
    let title = clean_optional_text(file.metadata.title.as_deref())
        .unwrap_or_else(|| file.derived_title.clone());
    let disc_no = file
        .metadata
        .disc_no
        .unwrap_or(file.directory_disc_no)
        .max(1);
    let track_no = file.metadata.track_no.or(file.derived_track_no);
    let album_uid = stable_id(&format!(
        "{}\n{}\n{}\n{}",
        volume.volume_uuid,
        normalize_text(&group_context.album_artist),
        normalize_text(&group_context.album_title),
        file.album_root_dir_hint
    ));

    ResolvedTrackRecord {
        album_uid,
        album_title: group_context.album_title.clone(),
        album_artist: group_context.album_artist.clone(),
        album_root_dir_hint: file.album_root_dir_hint.clone(),
        source_mode: group_context.source_mode.clone(),
        year: file.metadata.year.or(group_context.year),
        title,
        artist: track_artist,
        track_no,
        disc_no,
        duration_ms: file.metadata.duration_ms,
        sample_rate: file.metadata.sample_rate,
        bit_depth: file.metadata.bit_depth,
        genres: file.metadata.genres.clone(),
    }
}

fn file_grouping_key(file: &FileScanRecord) -> String {
    if let Some(album_title) = clean_optional_text(file.metadata.album_title.as_deref()) {
        let album_artist =
            clean_optional_text(file.metadata.album_artist.as_deref()).unwrap_or_default();
        return format!(
            "tag:{}:{}",
            normalize_text(&album_artist),
            normalize_text(&album_title)
        );
    }

    format!("fallback:{}", file.album_root_dir_hint)
}

fn build_track_search_content(file: &FileScanRecord, resolved: &ResolvedTrackRecord) -> String {
    let mut parts = vec![
        resolved.title.clone(),
        resolved.artist.clone(),
        resolved.album_artist.clone(),
        resolved.album_title.clone(),
        file.filename.clone(),
        file.relative_path.clone(),
        file.album_root_dir_hint.clone(),
    ];
    parts.extend(resolved.genres.clone());
    parts.retain(|value| !value.trim().is_empty());
    parts.join(" ")
}

fn parse_audio_metadata(path: &Path) -> Option<ParsedAudioMetadata> {
    let probe = match Probe::open(path) {
        Ok(probe) => probe,
        Err(_) => return fallback_probe_metadata(path),
    };
    let tagged_file = match probe.read() {
        Ok(tagged_file) => tagged_file,
        Err(_) => return fallback_probe_metadata(path),
    };

    let primary_tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let properties = tagged_file.properties();

    let mut metadata = ParsedAudioMetadata {
        duration_ms: duration_to_millis(properties.duration()),
        sample_rate: properties
            .sample_rate()
            .map(i64::from)
            .filter(|value| *value > 0),
        bit_depth: properties.bit_depth().map(i64::from),
        ..ParsedAudioMetadata::default()
    };

    if let Some(tag) = primary_tag {
        metadata.title = clean_optional_text(tag.title().as_deref());
        metadata.artist = clean_optional_text(tag.artist().as_deref());
        metadata.album_artist = clean_optional_text(tag.get_string(&ItemKey::AlbumArtist));
        metadata.album_title = clean_optional_text(tag.album().as_deref());
        metadata.track_no = tag.track().map(i64::from);
        metadata.disc_no = tag.disk().map(i64::from);
        metadata.year = tag.year().map(i64::from);
        if let Some(genre) = clean_optional_text(tag.genre().as_deref()) {
            metadata.genres = split_genres(&genre);
        }
    }

    Some(metadata)
}

fn fallback_probe_metadata(path: &Path) -> Option<ParsedAudioMetadata> {
    if !has_known_audio_header(path) {
        return None;
    }

    Some(ParsedAudioMetadata::default())
}

fn has_known_audio_header(path: &Path) -> bool {
    let extension = file_extension(path);
    match extension.as_str() {
        "wav" => file_header_matches(path, 0, b"RIFF") && file_header_matches(path, 8, b"WAVE"),
        "dff" => file_header_matches(path, 0, b"FRM8"),
        "dsf" | "dsd" => file_header_matches(path, 0, b"DSD "),
        _ => false,
    }
}

fn file_header_matches(path: &Path, offset: u64, expected: &[u8]) -> bool {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut prefix = vec![0u8; expected.len()];
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return false;
    }
    if file.read_exact(&mut prefix).is_err() {
        return false;
    }
    prefix == expected
}

fn discover_directory_artwork(
    scan_root: &Path,
    volume: &VolumeDescriptor,
    album_root_dir_hint: &str,
    indexed_at: i64,
) -> Result<Option<ScannedArtworkRef>, String> {
    let artwork_dir = if album_root_dir_hint.is_empty() {
        scan_root.to_path_buf()
    } else {
        scan_root.join(album_root_dir_hint)
    };
    let Some(source_file_abs_path) = find_preferred_directory_artwork(&artwork_dir)? else {
        return Ok(None);
    };

    let metadata = fs::metadata(&source_file_abs_path).map_err(|err| {
        format!(
            "read artwork metadata {}: {err}",
            source_file_abs_path.display()
        )
    })?;
    let file_mtime = file_mtime_secs(&metadata)?;
    let source_file_relative_path = relative_path_string(scan_root, &source_file_abs_path)?;
    let extension = file_extension(&source_file_abs_path);
    let mime_type = artwork_mime_type(&extension).ok_or_else(|| {
        format!(
            "unsupported artwork file type: {}",
            source_file_abs_path.display()
        )
    })?;
    let content_hash = stable_artwork_ref_id(
        volume,
        &source_file_relative_path,
        file_mtime,
        metadata.len(),
    );

    Ok(Some(ScannedArtworkRef {
        content_hash,
        mime_type: mime_type.to_string(),
        extension,
        source_file_relative_path,
        source_file_abs_path,
        indexed_at,
    }))
}

fn find_preferred_directory_artwork(directory: &Path) -> Result<Option<PathBuf>, String> {
    if !directory.is_dir() {
        return Ok(None);
    }

    let mut matches = BTreeMap::<String, PathBuf>::new();
    let entries = fs::read_dir(directory)
        .map_err(|err| format!("read artwork directory {}: {err}", directory.display()))?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            format!(
                "read artwork directory entry {}: {err}",
                directory.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("read artwork file type {}: {err}", path.display()))?;
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if SUPPORTED_COVER_FILENAMES.contains(&file_name.as_str()) {
            matches.entry(file_name).or_insert(path);
        }
    }

    for candidate in SUPPORTED_COVER_FILENAMES {
        if let Some(path) = matches.get(*candidate) {
            return Ok(Some(path.clone()));
        }
    }

    Ok(None)
}

fn artwork_mime_type(extension: &str) -> Option<&'static str> {
    match extension {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        _ => None,
    }
}

fn stable_artwork_ref_id(
    volume: &VolumeDescriptor,
    source_file_relative_path: &str,
    file_mtime: i64,
    file_size: u64,
) -> String {
    stable_id(&format!(
        "directory_file\n{}\n{}\n{}\n{}\n0\n{}",
        volume.volume_uuid, source_file_relative_path, file_mtime, file_size, CACHE_VERSION
    ))
}

fn generate_thumb_320(
    source_path_abs: &Path,
    artwork_cache_dir: &Path,
    cover_ref_id: &str,
) -> Result<GeneratedThumbRecord, String> {
    let source_image = ImageReader::open(source_path_abs)
        .map_err(|err| format!("open artwork image {}: {err}", source_path_abs.display()))?
        .decode()
        .map_err(|err| format!("decode artwork image {}: {err}", source_path_abs.display()))?;
    let width = i64::from(source_image.width());
    let height = i64::from(source_image.height());
    let thumb_image = if source_image.width() <= THUMB_MAX_EDGE_PX
        && source_image.height() <= THUMB_MAX_EDGE_PX
    {
        source_image
    } else {
        source_image.resize(THUMB_MAX_EDGE_PX, THUMB_MAX_EDGE_PX, FilterType::Lanczos3)
    };

    let thumb_abs = thumb_path(
        artwork_cache_dir
            .to_str()
            .ok_or_else(|| "artwork cache dir is not valid UTF-8".to_string())?,
        cover_ref_id,
    );
    let thumb_parent = thumb_abs
        .parent()
        .ok_or_else(|| format!("artwork thumb path has no parent: {}", thumb_abs.display()))?;
    fs::create_dir_all(thumb_parent)
        .map_err(|err| format!("create artwork thumb dir {}: {err}", thumb_parent.display()))?;

    let thumb_file = fs::File::create(&thumb_abs)
        .map_err(|err| format!("create artwork thumb {}: {err}", thumb_abs.display()))?;
    let mut writer = BufWriter::new(thumb_file);
    let mut encoder = JpegEncoder::new_with_quality(&mut writer, THUMB_JPEG_QUALITY);
    encoder
        .encode_image(&thumb_image)
        .map_err(|err| format!("encode artwork thumb {}: {err}", thumb_abs.display()))?;
    writer
        .flush()
        .map_err(|err| format!("flush artwork thumb {}: {err}", thumb_abs.display()))?;

    let thumb_rel_path = thumb_abs
        .strip_prefix(artwork_cache_dir)
        .map_err(|err| format!("strip artwork thumb prefix: {err}"))?
        .to_string_lossy()
        .trim_start_matches('/')
        .to_string();

    Ok(GeneratedThumbRecord {
        width,
        height,
        thumb_rel_path,
    })
}

fn duration_to_millis(duration: std::time::Duration) -> Option<i64> {
    let millis = duration.as_millis();
    if millis == 0 {
        None
    } else {
        Some(millis.min(i64::MAX as u128) as i64)
    }
}

fn collect_entries(
    scan_root: &Path,
    current_dir: &Path,
    directories: &mut BTreeMap<String, ScannedDirectory>,
    files: &mut Vec<FileScanRecord>,
    indexed_at: i64,
) -> Result<(), String> {
    let mut entries = fs::read_dir(current_dir)
        .map_err(|err| format!("read directory {}: {err}", current_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("read directory entry {}: {err}", current_dir.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if should_skip_name(&file_name) {
            continue;
        }

        let file_type = entry
            .file_type()
            .map_err(|err| format!("read file type {}: {err}", path.display()))?;

        if file_type.is_dir() {
            let relative_path = relative_path_string(scan_root, &path)?;
            directories.insert(
                relative_path.clone(),
                ScannedDirectory {
                    parent_relative_path: parent_relative_path(&relative_path),
                    display_name: display_name_from_relative_path(&relative_path),
                    relative_path,
                },
            );
            collect_entries(scan_root, &path, directories, files, indexed_at)?;
            continue;
        }

        if !file_type.is_file() || !is_audio_file(&path) {
            continue;
        }

        let metadata = entry
            .metadata()
            .map_err(|err| format!("read metadata {}: {err}", path.display()))?;
        let relative_path = relative_path_string(scan_root, &path)?;
        let filename = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| format!("non-utf8 filename: {}", path.display()))?
            .to_string();
        let stem = path
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or(&filename)
            .to_string();
        let parent_relative = parent_relative_path(&relative_path).unwrap_or_default();
        let (album_root_dir_hint, directory_disc_no) = album_root_and_disc_no(&parent_relative);
        let Some(parsed_metadata) = parse_audio_metadata(&path) else {
            continue;
        };
        files.push(FileScanRecord {
            relative_path,
            filename,
            derived_title: derive_track_title(&stem),
            derived_track_no: parse_track_number(&stem),
            directory_disc_no,
            format: file_extension(&path),
            file_mtime: file_mtime_secs(&metadata)?,
            indexed_at,
            size: metadata.len(),
            album_root_dir_hint,
            metadata: parsed_metadata,
        });
    }

    Ok(())
}

fn clear_volume_rows(tx: &Transaction<'_>, volume_uuid: &str) -> Result<(), String> {
    let album_uids = query_string_column(
        tx,
        "SELECT album_uid FROM albums WHERE volume_uuid = ?1",
        params![volume_uuid],
    )?;
    let track_uids = query_string_column(
        tx,
        "SELECT track_uid FROM tracks WHERE volume_uuid = ?1",
        params![volume_uuid],
    )?;

    tx.execute(
        "DELETE FROM album_artists
         WHERE album_id IN (SELECT album_id FROM albums WHERE volume_uuid = ?1)",
        params![volume_uuid],
    )
    .map_err(|err| format!("clear album_artists for {volume_uuid}: {err}"))?;
    tx.execute(
        "DELETE FROM track_artists
         WHERE track_uid IN (SELECT track_uid FROM tracks WHERE volume_uuid = ?1)",
        params![volume_uuid],
    )
    .map_err(|err| format!("clear track_artists for {volume_uuid}: {err}"))?;
    tx.execute(
        "DELETE FROM track_genres
         WHERE track_uid IN (SELECT track_uid FROM tracks WHERE volume_uuid = ?1)",
        params![volume_uuid],
    )
    .map_err(|err| format!("clear track_genres for {volume_uuid}: {err}"))?;

    for album_uid in album_uids {
        tx.execute(
            "DELETE FROM search_fts WHERE doc_type = 'album' AND doc_id = ?1",
            params![album_uid],
        )
        .map_err(|err| format!("clear album search entry for {volume_uuid}: {err}"))?;
    }
    for track_uid in track_uids {
        tx.execute(
            "DELETE FROM search_fts WHERE doc_type = 'track' AND doc_id = ?1",
            params![track_uid],
        )
        .map_err(|err| format!("clear track search entry for {volume_uuid}: {err}"))?;
    }

    tx.execute(
        "DELETE FROM tracks WHERE volume_uuid = ?1",
        params![volume_uuid],
    )
    .map_err(|err| format!("clear tracks for {volume_uuid}: {err}"))?;
    tx.execute(
        "DELETE FROM albums WHERE volume_uuid = ?1",
        params![volume_uuid],
    )
    .map_err(|err| format!("clear albums for {volume_uuid}: {err}"))?;
    tx.execute(
        "DELETE FROM directories WHERE volume_uuid = ?1",
        params![volume_uuid],
    )
    .map_err(|err| format!("clear directories for {volume_uuid}: {err}"))?;

    Ok(())
}

fn query_string_column<P>(tx: &Transaction<'_>, sql: &str, params: P) -> Result<Vec<String>, String>
where
    P: rusqlite::Params,
{
    let mut statement = tx
        .prepare(sql)
        .map_err(|err| format!("prepare query '{sql}': {err}"))?;
    let rows = statement
        .query_map(params, |row| row.get(0))
        .map_err(|err| format!("query '{sql}': {err}"))?;

    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|err| format!("read query row '{sql}': {err}"))?);
    }
    Ok(values)
}

fn upsert_volume(
    tx: &Transaction<'_>,
    volume: &VolumeDescriptor,
    indexed_at: i64,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO volumes (volume_uuid, label, mount_path, fs_type, is_available, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5)
         ON CONFLICT(volume_uuid) DO UPDATE SET
           label = excluded.label,
           mount_path = excluded.mount_path,
           fs_type = excluded.fs_type,
           is_available = excluded.is_available,
           last_seen_at = excluded.last_seen_at",
        params![
            volume.volume_uuid,
            volume.label,
            volume.mount_path,
            volume.fs_type,
            indexed_at
        ],
    )
    .map_err(|err| format!("upsert volume {}: {err}", volume.volume_uuid))?;

    Ok(())
}

fn seed_directory(
    tx: &rusqlite::Transaction<'_>,
    volume_uuid: &str,
    relative_path: &str,
    parent_relative_path: Option<&str>,
    display_name: &str,
    indexed_at: i64,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO directories (
            volume_uuid, relative_path, parent_relative_path, display_name, indexed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(volume_uuid, relative_path) DO UPDATE SET
           parent_relative_path = excluded.parent_relative_path,
           display_name = excluded.display_name,
           indexed_at = excluded.indexed_at",
        params![
            volume_uuid,
            relative_path,
            parent_relative_path,
            display_name,
            indexed_at
        ],
    )
    .map_err(|err| format!("seed directory {relative_path}: {err}"))?;

    Ok(())
}

fn seed_artwork_ref(
    tx: &rusqlite::Transaction<'_>,
    artwork_cache_dir: &Path,
    indexed_at: i64,
) -> Result<i64, String> {
    let cover_ref_id = "demo-cover-a1b2";
    let thumb_abs = thumb_path(
        artwork_cache_dir
            .to_str()
            .ok_or_else(|| "artwork cache dir is not valid UTF-8".to_string())?,
        cover_ref_id,
    );
    let thumb_rel = thumb_abs
        .strip_prefix(artwork_cache_dir)
        .map_err(|err| format!("strip artwork cache prefix: {err}"))?
        .to_string_lossy()
        .trim_start_matches('/')
        .to_string();

    tx.execute(
        "INSERT INTO artwork_refs (
            content_hash, mime_type, width, height, source_rel_path, thumb_rel_path, indexed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(content_hash) DO UPDATE SET
           mime_type = excluded.mime_type,
           width = excluded.width,
           height = excluded.height,
           source_rel_path = excluded.source_rel_path,
           thumb_rel_path = excluded.thumb_rel_path,
           indexed_at = excluded.indexed_at",
        params![
            cover_ref_id,
            "image/jpeg",
            320,
            320,
            "source/de/mo/demo-cover-a1b2.jpg",
            thumb_rel,
            indexed_at
        ],
    )
    .map_err(|err| format!("seed artwork ref: {err}"))?;

    tx.query_row(
        "SELECT artwork_ref_id FROM artwork_refs WHERE content_hash = ?1",
        params![cover_ref_id],
        |row| row.get(0),
    )
    .map_err(|err| format!("load artwork ref id: {err}"))
}

fn upsert_artwork_ref(
    tx: &Transaction<'_>,
    artwork_ref: &ScannedArtworkRef,
    artwork_cache_dir: &Path,
) -> Result<i64, String> {
    let cache_source_abs = source_path(
        artwork_cache_dir
            .to_str()
            .ok_or_else(|| "artwork cache dir is not valid UTF-8".to_string())?,
        &artwork_ref.content_hash,
        &artwork_ref.extension,
    );
    let cache_source_parent = cache_source_abs.parent().ok_or_else(|| {
        format!(
            "artwork source path has no parent: {}",
            cache_source_abs.display()
        )
    })?;
    fs::create_dir_all(cache_source_parent).map_err(|err| {
        format!(
            "create artwork source dir {}: {err}",
            cache_source_parent.display()
        )
    })?;
    fs::copy(&artwork_ref.source_file_abs_path, &cache_source_abs).map_err(|err| {
        format!(
            "copy artwork source {} -> {}: {err}",
            artwork_ref.source_file_abs_path.display(),
            cache_source_abs.display()
        )
    })?;

    let source_rel_path = cache_source_abs
        .strip_prefix(artwork_cache_dir)
        .map_err(|err| format!("strip artwork source prefix: {err}"))?
        .to_string_lossy()
        .trim_start_matches('/')
        .to_string();
    let generated_thumb = generate_thumb_320(
        &cache_source_abs,
        artwork_cache_dir,
        &artwork_ref.content_hash,
    );
    if let Err(err) = &generated_thumb {
        eprintln!(
            "media-indexd artwork thumbnail skipped for {}: {err}",
            artwork_ref.source_file_relative_path
        );
    }
    let generated_thumb = generated_thumb.ok();

    tx.execute(
        "INSERT INTO artwork_refs (
            content_hash, mime_type, width, height, source_rel_path, thumb_rel_path, indexed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(content_hash) DO UPDATE SET
           mime_type = excluded.mime_type,
           width = excluded.width,
           height = excluded.height,
           source_rel_path = excluded.source_rel_path,
           thumb_rel_path = excluded.thumb_rel_path,
           indexed_at = excluded.indexed_at",
        params![
            artwork_ref.content_hash,
            artwork_ref.mime_type,
            generated_thumb.as_ref().map(|thumb| thumb.width),
            generated_thumb.as_ref().map(|thumb| thumb.height),
            source_rel_path,
            generated_thumb
                .as_ref()
                .map(|thumb| thumb.thumb_rel_path.as_str()),
            artwork_ref.indexed_at
        ],
    )
    .map_err(|err| {
        format!(
            "upsert artwork ref for {}: {err}",
            artwork_ref.source_file_relative_path
        )
    })?;

    tx.query_row(
        "SELECT artwork_ref_id FROM artwork_refs WHERE content_hash = ?1",
        params![artwork_ref.content_hash],
        |row| row.get(0),
    )
    .map_err(|err| format!("load artwork ref id {}: {err}", artwork_ref.content_hash))
}

fn seed_album(
    tx: &rusqlite::Transaction<'_>,
    cover_ref_id: i64,
    indexed_at: i64,
) -> Result<i64, String> {
    tx.execute(
        "INSERT INTO albums (
            album_uid,
            volume_uuid,
            album_title,
            album_title_norm,
            album_artist,
            album_artist_norm,
            album_root_dir_hint,
            year,
            disc_count,
            track_count,
            total_duration_ms,
            cover_ref_id,
            source_mode,
            indexed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, 2, ?9, ?10, 'folder', ?11)
         ON CONFLICT(album_uid) DO UPDATE SET
           volume_uuid = excluded.volume_uuid,
           album_title = excluded.album_title,
           album_title_norm = excluded.album_title_norm,
           album_artist = excluded.album_artist,
           album_artist_norm = excluded.album_artist_norm,
           album_root_dir_hint = excluded.album_root_dir_hint,
           year = excluded.year,
           disc_count = excluded.disc_count,
           track_count = excluded.track_count,
           total_duration_ms = excluded.total_duration_ms,
           cover_ref_id = excluded.cover_ref_id,
           source_mode = excluded.source_mode,
           indexed_at = excluded.indexed_at",
        params![
            "demo-album",
            "demo-volume",
            "Demo Album",
            "demo album",
            "Demo Artist",
            "demo artist",
            "Demo Album",
            2026,
            387_000_i64,
            cover_ref_id,
            indexed_at
        ],
    )
    .map_err(|err| format!("seed album: {err}"))?;

    tx.query_row(
        "SELECT album_id FROM albums WHERE album_uid = ?1",
        params!["demo-album"],
        |row| row.get(0),
    )
    .map_err(|err| format!("load album id: {err}"))
}

fn upsert_album(tx: &Transaction<'_>, album: &ScannedAlbum) -> Result<i64, String> {
    tx.execute(
        "INSERT INTO albums (
            album_uid,
            volume_uuid,
            album_title,
            album_title_norm,
            album_artist,
            album_artist_norm,
            album_root_dir_hint,
            year,
            disc_count,
            track_count,
            total_duration_ms,
            cover_ref_id,
            musicbrainz_release_id,
            source_mode,
            indexed_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13, ?14
         )
         ON CONFLICT(album_uid) DO UPDATE SET
           volume_uuid = excluded.volume_uuid,
           album_title = excluded.album_title,
           album_title_norm = excluded.album_title_norm,
           album_artist = excluded.album_artist,
           album_artist_norm = excluded.album_artist_norm,
           album_root_dir_hint = excluded.album_root_dir_hint,
           year = excluded.year,
           disc_count = excluded.disc_count,
           track_count = excluded.track_count,
           total_duration_ms = excluded.total_duration_ms,
           cover_ref_id = excluded.cover_ref_id,
           musicbrainz_release_id = excluded.musicbrainz_release_id,
           source_mode = excluded.source_mode,
           indexed_at = excluded.indexed_at",
        params![
            album.album_uid,
            album.volume_uuid,
            album.album_title,
            album.album_title_norm,
            album.album_artist,
            album.album_artist_norm,
            album.album_root_dir_hint,
            album.year,
            album.disc_count,
            album.track_count,
            album.total_duration_ms,
            album.cover_ref_id,
            album.source_mode,
            album.indexed_at
        ],
    )
    .map_err(|err| format!("upsert album {}: {err}", album.album_uid))?;

    tx.query_row(
        "SELECT album_id FROM albums WHERE album_uid = ?1",
        params![album.album_uid],
        |row| row.get(0),
    )
    .map_err(|err| format!("load album id {}: {err}", album.album_uid))
}

#[allow(clippy::too_many_arguments)]
fn seed_track(
    tx: &rusqlite::Transaction<'_>,
    track_uid: &str,
    album_id: i64,
    volume_uuid: &str,
    relative_path: &str,
    filename: &str,
    title: &str,
    artist: &str,
    track_no: i64,
    disc_no: i64,
    duration_ms: i64,
    sample_rate: i64,
    bit_depth: i64,
    format: &str,
    cover_ref_id: i64,
    indexed_at: i64,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO tracks (
            track_uid,
            album_id,
            volume_uuid,
            relative_path,
            filename,
            title,
            artist,
            album_artist,
            track_no,
            disc_no,
            duration_ms,
            sample_rate,
            bit_depth,
            format,
            cover_ref_id,
            file_mtime,
            indexed_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
         )
         ON CONFLICT(track_uid) DO UPDATE SET
           album_id = excluded.album_id,
           volume_uuid = excluded.volume_uuid,
           relative_path = excluded.relative_path,
           filename = excluded.filename,
           title = excluded.title,
           artist = excluded.artist,
           album_artist = excluded.album_artist,
           track_no = excluded.track_no,
           disc_no = excluded.disc_no,
           duration_ms = excluded.duration_ms,
           sample_rate = excluded.sample_rate,
           bit_depth = excluded.bit_depth,
           format = excluded.format,
           cover_ref_id = excluded.cover_ref_id,
           file_mtime = excluded.file_mtime,
           indexed_at = excluded.indexed_at",
        params![
            track_uid,
            album_id,
            volume_uuid,
            relative_path,
            filename,
            title,
            artist,
            "Demo Artist",
            track_no,
            disc_no,
            duration_ms,
            sample_rate,
            bit_depth,
            format,
            cover_ref_id,
            indexed_at,
            indexed_at
        ],
    )
    .map_err(|err| format!("seed track {track_uid}: {err}"))?;

    Ok(())
}

fn upsert_track(tx: &Transaction<'_>, track: &ScannedTrack, album_id: i64) -> Result<(), String> {
    tx.execute(
        "INSERT INTO tracks (
            track_uid,
            album_id,
            volume_uuid,
            relative_path,
            filename,
            title,
            artist,
            album_artist,
            track_no,
            disc_no,
            duration_ms,
            sample_rate,
            bit_depth,
            format,
            cover_ref_id,
            musicbrainz_track_id,
            file_mtime,
            indexed_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
         )
         ON CONFLICT(track_uid) DO UPDATE SET
           album_id = excluded.album_id,
           volume_uuid = excluded.volume_uuid,
           relative_path = excluded.relative_path,
           filename = excluded.filename,
           title = excluded.title,
           artist = excluded.artist,
           album_artist = excluded.album_artist,
           track_no = excluded.track_no,
           disc_no = excluded.disc_no,
           duration_ms = excluded.duration_ms,
           sample_rate = excluded.sample_rate,
           bit_depth = excluded.bit_depth,
           format = excluded.format,
           cover_ref_id = excluded.cover_ref_id,
           musicbrainz_track_id = excluded.musicbrainz_track_id,
           file_mtime = excluded.file_mtime,
           indexed_at = excluded.indexed_at",
        params![
            track.track_uid,
            album_id,
            track.volume_uuid,
            track.relative_path,
            track.filename,
            track.title,
            track.artist,
            track.album_artist,
            track.track_no,
            track.disc_no,
            track.duration_ms,
            track.sample_rate,
            track.bit_depth,
            track.format,
            track.cover_ref_id,
            track.musicbrainz_track_id,
            track.file_mtime,
            track.indexed_at
        ],
    )
    .map_err(|err| format!("upsert track {}: {err}", track.track_uid))?;

    Ok(())
}

fn seed_artist(tx: &rusqlite::Transaction<'_>, artist_name: &str) -> Result<(), String> {
    tx.execute(
        "INSERT INTO artists (artist_name, artist_name_norm) VALUES (?1, ?2)
         ON CONFLICT(artist_name_norm) DO UPDATE SET artist_name = excluded.artist_name",
        params![artist_name, normalize_text(artist_name)],
    )
    .map_err(|err| format!("seed artist: {err}"))?;

    Ok(())
}

fn seed_artist_and_load_id(tx: &Transaction<'_>, artist_name: &str) -> Result<i64, String> {
    seed_artist(tx, artist_name)?;
    tx.query_row(
        "SELECT artist_id FROM artists WHERE artist_name_norm = ?1",
        params![normalize_text(artist_name)],
        |row| row.get(0),
    )
    .map_err(|err| format!("load artist id for {artist_name}: {err}"))
}

fn upsert_album_artist(tx: &Transaction<'_>, album_id: i64, artist_id: i64) -> Result<(), String> {
    tx.execute(
        "INSERT INTO album_artists (album_id, artist_id, sort_order)
         VALUES (?1, ?2, 0)
         ON CONFLICT(album_id, artist_id) DO UPDATE SET sort_order = excluded.sort_order",
        params![album_id, artist_id],
    )
    .map_err(|err| format!("upsert album_artists {album_id}/{artist_id}: {err}"))?;

    Ok(())
}

fn upsert_track_artist(
    tx: &Transaction<'_>,
    track_uid: &str,
    artist_id: i64,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO track_artists (track_uid, artist_id, role, sort_order)
         VALUES (?1, ?2, 'performer', 0)
         ON CONFLICT(track_uid, artist_id, role) DO UPDATE SET sort_order = excluded.sort_order",
        params![track_uid, artist_id],
    )
    .map_err(|err| format!("upsert track_artists {track_uid}/{artist_id}: {err}"))?;

    Ok(())
}

fn seed_genre(tx: &rusqlite::Transaction<'_>, genre_name: &str) -> Result<(), String> {
    tx.execute(
        "INSERT INTO genres (genre_name, genre_name_norm) VALUES (?1, ?2)
         ON CONFLICT(genre_name_norm) DO UPDATE SET genre_name = excluded.genre_name",
        params![genre_name, normalize_text(genre_name)],
    )
    .map_err(|err| format!("seed genre: {err}"))?;

    Ok(())
}

fn seed_genre_and_load_id(tx: &Transaction<'_>, genre_name: &str) -> Result<i64, String> {
    seed_genre(tx, genre_name)?;
    tx.query_row(
        "SELECT genre_id FROM genres WHERE genre_name_norm = ?1",
        params![normalize_text(genre_name)],
        |row| row.get(0),
    )
    .map_err(|err| format!("load genre id for {genre_name}: {err}"))
}

fn upsert_track_genre(tx: &Transaction<'_>, track_uid: &str, genre_id: i64) -> Result<(), String> {
    tx.execute(
        "INSERT INTO track_genres (track_uid, genre_id)
         VALUES (?1, ?2)
         ON CONFLICT(track_uid, genre_id) DO NOTHING",
        params![track_uid, genre_id],
    )
    .map_err(|err| format!("upsert track_genres {track_uid}/{genre_id}: {err}"))?;

    Ok(())
}

fn delete_orphan_artists(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute(
        "DELETE FROM artists
         WHERE artist_id NOT IN (
            SELECT artist_id FROM album_artists
            UNION
            SELECT artist_id FROM track_artists
         )",
        [],
    )
    .map_err(|err| format!("delete orphan artists: {err}"))?;

    Ok(())
}

fn delete_orphan_genres(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute(
        "DELETE FROM genres
         WHERE genre_id NOT IN (
            SELECT genre_id FROM track_genres
         )",
        [],
    )
    .map_err(|err| format!("delete orphan genres: {err}"))?;

    Ok(())
}

fn delete_orphan_artwork_refs(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute(
        "DELETE FROM artwork_refs
         WHERE artwork_ref_id NOT IN (
            SELECT cover_ref_id FROM albums WHERE cover_ref_id IS NOT NULL
            UNION
            SELECT cover_ref_id FROM tracks WHERE cover_ref_id IS NOT NULL
         )",
        [],
    )
    .map_err(|err| format!("delete orphan artwork refs: {err}"))?;

    Ok(())
}

fn seed_search_entry(
    tx: &rusqlite::Transaction<'_>,
    doc_type: &str,
    doc_id: &str,
    content: &str,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO search_fts (doc_type, doc_id, content) VALUES (?1, ?2, ?3)
         ON CONFLICT(doc_type, doc_id) DO UPDATE SET content = excluded.content",
        params![doc_type, doc_id, content],
    )
    .map_err(|err| format!("seed search entry {doc_type}:{doc_id}: {err}"))?;

    Ok(())
}

fn summarize_library(
    connection: &Connection,
    db_path: &Path,
    command_name: &'static str,
) -> Result<LibrarySummary, String> {
    Ok(LibrarySummary {
        db_path: db_path.to_path_buf(),
        command_name,
        volume_count: table_count(connection, "volumes")?,
        directory_count: table_count(connection, "directories")?,
        album_count: table_count(connection, "albums")?,
        track_count: table_count(connection, "tracks")?,
        artwork_ref_count: table_count(connection, "artwork_refs")?,
    })
}

fn table_count(connection: &Connection, table: &str) -> Result<i64, String> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    connection
        .query_row(&sql, [], |row| row.get(0))
        .map_err(|err| format!("count {table}: {err}"))
}

fn ensure_parent_dir(path: &Path, label: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label} path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("create {label} dir {}: {err}", parent.display()))
}

fn library_db_path_from_env() -> PathBuf {
    env::var("LIBRARY_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| state_dir_path().join("library.db"))
}

fn artwork_cache_dir_from_env() -> PathBuf {
    env::var("ARTWORK_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| cache_dir_path().join("artwork"))
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn relative_path_string(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|err| {
        format!(
            "strip prefix {} from {}: {err}",
            root.display(),
            path.display()
        )
    })?;
    let joined = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/");
    Ok(joined)
}

fn parent_relative_path(relative_path: &str) -> Option<String> {
    let path = Path::new(relative_path);
    path.parent()
        .map(|parent| parent.to_string_lossy().replace('\\', "/"))
        .filter(|value| !value.is_empty() && value != ".")
}

fn display_name_from_relative_path(relative_path: &str) -> String {
    Path::new(relative_path)
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(relative_path)
        .to_string()
}

fn should_skip_name(name: &str) -> bool {
    name.starts_with('.') || name.starts_with("._")
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|ext| SUPPORTED_AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn file_extension(path: &Path) -> String {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

fn file_mtime_secs(metadata: &fs::Metadata) -> Result<i64, String> {
    let modified = metadata
        .modified()
        .map_err(|err| format!("read file modified time: {err}"))?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64)
}

fn album_root_and_disc_no(parent_relative_path: &str) -> (String, i64) {
    if parent_relative_path.is_empty() {
        return (String::new(), 1);
    }

    let path = Path::new(parent_relative_path);
    let last = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(parent_relative_path);
    if let Some(disc_no) = parse_disc_number(last) {
        let album_root = path
            .parent()
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .filter(|value| !value.is_empty() && value != ".")
            .unwrap_or_else(|| parent_relative_path.to_string());
        return (album_root, disc_no);
    }

    (parent_relative_path.to_string(), 1)
}

fn album_title_for_root(album_root_dir_hint: &str, volume_label: &str) -> String {
    if album_root_dir_hint.is_empty() {
        return volume_label.to_string();
    }

    Path::new(album_root_dir_hint)
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(volume_label)
        .to_string()
}

fn derive_track_title(stem: &str) -> String {
    let trimmed = stem.trim();
    let mut start = 0;
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() {
            start += ch.len_utf8();
            continue;
        }
        break;
    }

    let mut candidate = &trimmed[start..];
    candidate = candidate.trim_start_matches(|ch: char| matches!(ch, ' ' | '-' | '_' | '.'));
    let candidate = candidate.replace('_', " ");
    let candidate = collapse_whitespace(&candidate);
    if candidate.is_empty() {
        collapse_whitespace(trimmed)
    } else {
        candidate
    }
}

fn parse_track_number(stem: &str) -> Option<i64> {
    let digits: String = stem.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
}

fn parse_disc_number(name: &str) -> Option<i64> {
    let lower = name.to_ascii_lowercase();
    for prefix in ["disc", "disk", "cd"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let digits: String = rest
                .chars()
                .skip_while(|ch| !ch.is_ascii_digit())
                .take_while(|ch| ch.is_ascii_digit())
                .collect();
            if let Ok(value) = digits.parse() {
                return Some(value);
            }
        }
    }

    None
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty())
}

fn split_genres(raw: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    for piece in raw.split([';', '/']) {
        let normalized = collapse_whitespace(piece);
        if !normalized.is_empty() {
            seen.insert(normalized);
        }
    }

    if seen.is_empty() {
        let fallback = collapse_whitespace(raw);
        if fallback.is_empty() {
            Vec::new()
        } else {
            vec![fallback]
        }
    } else {
        seen.into_iter().collect()
    }
}

fn normalize_text(value: &str) -> String {
    collapse_whitespace(value).to_ascii_lowercase()
}

fn stable_id(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::{
        build_volume_descriptor, discover_directory_artwork, ensure_schema, execute,
        fallback_probe_metadata, file_grouping_key, resolve_group_album_context,
        resolve_track_record, split_genres, table_count, Config, FileScanRecord, MediaIndexCommand,
        ParsedAudioMetadata, VolumeDescriptor, DEFAULT_LIBRARY_DB_PATH, UNKNOWN_ARTIST,
    };
    use image::{ImageReader, Rgb, RgbImage};
    use rusqlite::{Connection, OptionalExtension};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ensure_schema_creates_core_tables() {
        let db_path = temp_path("library-schema", "db");
        let connection = Connection::open(&db_path).unwrap();

        ensure_schema(&connection).unwrap();

        assert_eq!(table_count(&connection, "volumes").unwrap(), 0);
        assert_eq!(table_count(&connection, "albums").unwrap(), 0);
        assert_eq!(table_count(&connection, "tracks").unwrap(), 0);
        assert!(schema_version(&connection).is_some());

        cleanup_db_files(&db_path);
    }

    #[test]
    fn seed_demo_populates_minimum_library_entities() {
        let db_path = temp_path("library-seed", "db");
        let cache_dir = temp_dir("artwork-cache");
        let summary = execute(Config {
            command: MediaIndexCommand::SeedDemo,
            db_path: db_path.clone(),
            artwork_cache_dir: cache_dir.clone(),
        })
        .unwrap();

        assert_eq!(summary.command_name, "seed-demo");
        assert_eq!(summary.volume_count, 1);
        assert_eq!(summary.album_count, 1);
        assert_eq!(summary.track_count, 2);
        assert_eq!(summary.artwork_ref_count, 1);

        cleanup_db_files(&db_path);
        cleanup_dir(&cache_dir);
    }

    #[test]
    fn scan_dir_populates_real_directory_snapshot() {
        let db_path = temp_path("library-scan", "db");
        let cache_dir = temp_dir("scan-artwork");
        let root = temp_dir("scan-root");

        write_test_wav(&root.join("Album One/01 - Opening.wav"), 44_100, 250);
        write_test_wav(&root.join("Album One/02_Afterglow.wav"), 44_100, 250);
        write_test_wav(&root.join("Album Two/Disc 2/07 - Finale.wav"), 48_000, 250);
        touch(&root.join("notes.txt"));

        let summary = execute(Config {
            command: MediaIndexCommand::ScanDir {
                scan_root: root.clone(),
            },
            db_path: db_path.clone(),
            artwork_cache_dir: cache_dir.clone(),
        })
        .unwrap();

        let connection = Connection::open(&db_path).unwrap();
        let artist_count = table_count(&connection, "artists").unwrap();
        let first_album: (String, i64) = connection
            .query_row(
                "SELECT album_title, track_count FROM albums ORDER BY album_title ASC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let finale_row: (String, i64) = connection
            .query_row(
                "SELECT title, disc_no FROM tracks WHERE relative_path = 'Album Two/Disc 2/07 - Finale.wav'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(summary.command_name, "scan-dir");
        assert_eq!(summary.volume_count, 1);
        assert_eq!(summary.directory_count, 4);
        assert_eq!(summary.album_count, 2);
        assert_eq!(summary.track_count, 3);
        assert_eq!(artist_count, 1);
        assert_eq!(first_album, ("Album One".to_string(), 2));
        assert_eq!(finale_row, ("Finale".to_string(), 2));

        cleanup_db_files(&db_path);
        cleanup_dir(&cache_dir);
        cleanup_dir(&root);
    }

    #[test]
    fn rescan_replaces_previous_rows_for_same_volume() {
        let db_path = temp_path("library-rescan", "db");
        let cache_dir = temp_dir("rescan-artwork");
        let root = temp_dir("rescan-root");

        write_test_wav(&root.join("Album/01 - First.wav"), 44_100, 250);
        write_test_wav(&root.join("Album/02 - Second.wav"), 44_100, 250);

        execute(Config {
            command: MediaIndexCommand::ScanDir {
                scan_root: root.clone(),
            },
            db_path: db_path.clone(),
            artwork_cache_dir: cache_dir.clone(),
        })
        .unwrap();

        fs::remove_file(root.join("Album/02 - Second.wav")).unwrap();

        let summary = execute(Config {
            command: MediaIndexCommand::ScanDir { scan_root: root },
            db_path: db_path.clone(),
            artwork_cache_dir: cache_dir.clone(),
        })
        .unwrap();

        let connection = Connection::open(&db_path).unwrap();
        let track_titles = string_rows(&connection, "SELECT title FROM tracks ORDER BY title ASC");

        assert_eq!(summary.track_count, 1);
        assert_eq!(track_titles, vec!["First".to_string()]);

        cleanup_db_files(&db_path);
        cleanup_dir(&cache_dir);
    }

    #[test]
    fn scan_dir_discovers_folder_cover_and_links_album_artwork() {
        let db_path = temp_path("library-cover", "db");
        let cache_dir = temp_dir("cover-artwork");
        let root = temp_dir("cover-root");

        write_test_wav(&root.join("Album With Cover/01 - Opening.wav"), 44_100, 250);
        write_test_jpeg(
            &root.join("Album With Cover/folder.jpg"),
            640,
            480,
            [32, 64, 128],
        );

        execute(Config {
            command: MediaIndexCommand::ScanDir {
                scan_root: root.clone(),
            },
            db_path: db_path.clone(),
            artwork_cache_dir: cache_dir.clone(),
        })
        .unwrap();

        let connection = Connection::open(&db_path).unwrap();
        let artwork_row: (String, i64, i64, String, String) = connection
            .query_row(
                "SELECT artwork_refs.mime_type, COALESCE(artwork_refs.width, 0), COALESCE(artwork_refs.height, 0),
                        artwork_refs.source_rel_path, COALESCE(artwork_refs.thumb_rel_path, '')
                 FROM albums
                 JOIN artwork_refs ON artwork_refs.artwork_ref_id = albums.cover_ref_id
                 WHERE albums.album_title = 'Album With Cover'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        let covered_tracks: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM tracks WHERE cover_ref_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(artwork_row.0, "image/jpeg".to_string());
        assert_eq!(artwork_row.1, 640);
        assert_eq!(artwork_row.2, 480);
        assert!(artwork_row.3.starts_with("source/"));
        assert!(artwork_row.3.ends_with(".jpg"));
        assert!(artwork_row.4.starts_with("thumb/320/"));
        assert!(artwork_row.4.ends_with(".jpg"));
        assert_eq!(covered_tracks, 1);
        assert!(cache_dir.join(&artwork_row.3).is_file());
        assert!(cache_dir.join(&artwork_row.4).is_file());

        let thumb_image = ImageReader::open(cache_dir.join(&artwork_row.4))
            .unwrap()
            .decode()
            .unwrap();
        assert_eq!(thumb_image.width(), 320);
        assert_eq!(thumb_image.height(), 240);

        cleanup_db_files(&db_path);
        cleanup_dir(&cache_dir);
        cleanup_dir(&root);
    }

    #[test]
    fn scan_dir_prefers_folder_jpg_over_cover_jpg() {
        let db_path = temp_path("library-cover-priority", "db");
        let cache_dir = temp_dir("cover-priority-artwork");
        let root = temp_dir("cover-priority-root");

        write_test_wav(&root.join("Priority Album/01 - Opening.wav"), 44_100, 250);
        write_test_jpeg(
            &root.join("Priority Album/folder.jpg"),
            500,
            500,
            [12, 34, 56],
        );
        write_test_jpeg(
            &root.join("Priority Album/cover.jpg"),
            700,
            400,
            [78, 90, 123],
        );

        execute(Config {
            command: MediaIndexCommand::ScanDir {
                scan_root: root.clone(),
            },
            db_path: db_path.clone(),
            artwork_cache_dir: cache_dir.clone(),
        })
        .unwrap();

        let canonical_root = fs::canonicalize(&root).unwrap();
        let volume = build_volume_descriptor(&canonical_root).unwrap();
        let expected_artwork =
            discover_directory_artwork(&canonical_root, &volume, "Priority Album", 1)
                .unwrap()
                .unwrap();

        let connection = Connection::open(&db_path).unwrap();
        let chosen_ref: String = connection
            .query_row(
                "SELECT artwork_refs.content_hash
                 FROM albums
                 JOIN artwork_refs ON artwork_refs.artwork_ref_id = albums.cover_ref_id
                 WHERE albums.album_title = 'Priority Album'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            expected_artwork.source_file_relative_path,
            "Priority Album/folder.jpg"
        );
        assert_eq!(chosen_ref, expected_artwork.content_hash);

        cleanup_db_files(&db_path);
        cleanup_dir(&cache_dir);
        cleanup_dir(&root);
    }

    #[test]
    fn scan_dir_skips_unreadable_audio_files() {
        let db_path = temp_path("library-scan-bad-media", "db");
        let cache_dir = temp_dir("scan-bad-media-artwork");
        let root = temp_dir("scan-bad-media-root");

        touch(&root.join("Broken Album/01 - Broken.mp3"));
        touch(&root.join("Broken Album/02 - Broken.flac"));
        touch(&root.join("Broken Album/03 - Broken.ogg"));
        write_test_wav(&root.join("Broken Album/04 - Valid.wav"), 44_100, 250);

        let summary = execute(Config {
            command: MediaIndexCommand::ScanDir {
                scan_root: root.clone(),
            },
            db_path: db_path.clone(),
            artwork_cache_dir: cache_dir.clone(),
        })
        .unwrap();

        let connection = Connection::open(&db_path).unwrap();
        let relative_paths = string_rows(
            &connection,
            "SELECT relative_path FROM tracks ORDER BY relative_path ASC",
        );

        assert_eq!(summary.album_count, 1);
        assert_eq!(summary.track_count, 1);
        assert_eq!(
            relative_paths,
            vec!["Broken Album/04 - Valid.wav".to_string()]
        );

        cleanup_db_files(&db_path);
        cleanup_dir(&cache_dir);
        cleanup_dir(&root);
    }

    #[test]
    fn ensure_schema_uses_default_path_constant_shape() {
        assert!(DEFAULT_LIBRARY_DB_PATH.ends_with("/library.db"));
    }

    #[test]
    fn tag_context_promotes_group_level_album_identity() {
        let volume = demo_volume();
        let tagged = file_record(
            "Pink Floyd - Wish You Were Here/01 - Shine On.flac",
            "Pink Floyd - Wish You Were Here",
            ParsedAudioMetadata {
                title: Some("Shine On You Crazy Diamond".to_string()),
                artist: Some("Pink Floyd".to_string()),
                album_artist: Some("Pink Floyd".to_string()),
                album_title: Some("Wish You Were Here".to_string()),
                track_no: Some(1),
                disc_no: Some(1),
                year: Some(1975),
                genres: vec!["Progressive Rock".to_string(), "Art Rock".to_string()],
                duration_ms: Some(81000),
                sample_rate: Some(96000),
                bit_depth: Some(24),
            },
        );
        let sparse = file_record(
            "Pink Floyd - Wish You Were Here/02 - Welcome.flac",
            "Pink Floyd - Wish You Were Here",
            ParsedAudioMetadata {
                title: Some("Welcome to the Machine".to_string()),
                ..ParsedAudioMetadata::default()
            },
        );

        let context = resolve_group_album_context(
            &[tagged.clone(), sparse.clone()],
            &volume.label,
            &tagged.album_root_dir_hint,
        );
        let resolved = resolve_track_record(&volume, &sparse, &context);

        assert_eq!(context.source_mode, "tag");
        assert_eq!(resolved.album_title, "Wish You Were Here");
        assert_eq!(resolved.album_artist, "Pink Floyd");
        assert_eq!(resolved.artist, "Pink Floyd");
        assert_eq!(resolved.source_mode, "tag");
    }

    #[test]
    fn directory_fallback_remains_when_album_tags_are_missing() {
        let volume = demo_volume();
        let file = file_record(
            "Loose Tracks/01 - Unknown.flac",
            "Loose Tracks",
            ParsedAudioMetadata::default(),
        );

        let context = resolve_group_album_context(&[file.clone()], &volume.label, "Loose Tracks");
        let resolved = resolve_track_record(&volume, &file, &context);

        assert_eq!(context.source_mode, "directory_fallback");
        assert_eq!(resolved.album_artist, UNKNOWN_ARTIST);
        assert_eq!(resolved.album_title, "Loose Tracks");
        assert_eq!(resolved.title, "Unknown");
    }

    #[test]
    fn file_grouping_key_splits_mixed_folder_by_tagged_album_identity() {
        let first = file_record(
            "Mixed Folder/01 - Song A.flac",
            "Mixed Folder",
            ParsedAudioMetadata {
                album_title: Some("Album A".to_string()),
                artist: Some("Artist A".to_string()),
                ..ParsedAudioMetadata::default()
            },
        );
        let second = file_record(
            "Mixed Folder/02 - Song B.flac",
            "Mixed Folder",
            ParsedAudioMetadata {
                album_title: Some("Album B".to_string()),
                artist: Some("Artist B".to_string()),
                ..ParsedAudioMetadata::default()
            },
        );
        let fallback = file_record(
            "Mixed Folder/03 - Untagged.flac",
            "Mixed Folder",
            ParsedAudioMetadata::default(),
        );

        assert_ne!(file_grouping_key(&first), file_grouping_key(&second));
        assert_ne!(file_grouping_key(&first), file_grouping_key(&fallback));
        assert!(file_grouping_key(&fallback).starts_with("fallback:"));
    }

    #[test]
    fn fallback_probe_metadata_accepts_known_dsd_and_wav_headers() {
        let wav = temp_path("header-fallback-wav", "wav");
        write_test_wav(&wav, 44_100, 250);

        let dff = temp_path("header-fallback-dff", "dff");
        fs::write(&dff, b"FRM8\x00\x00\x00\x00DSD ").unwrap();

        let bogus = temp_path("header-fallback-bogus", "flac");
        fs::write(&bogus, b"lumelo").unwrap();

        assert!(fallback_probe_metadata(&wav).is_some());
        assert!(fallback_probe_metadata(&dff).is_some());
        assert!(fallback_probe_metadata(&bogus).is_none());

        let _ = fs::remove_file(wav);
        let _ = fs::remove_file(dff);
        let _ = fs::remove_file(bogus);
    }

    #[test]
    fn split_genres_breaks_simple_multi_value_strings() {
        assert_eq!(
            split_genres("Ambient; Electronic / Drone"),
            vec![
                "Ambient".to_string(),
                "Drone".to_string(),
                "Electronic".to_string()
            ]
        );
    }

    fn schema_version(connection: &Connection) -> Option<String> {
        connection
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'library_schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
    }

    fn string_rows(connection: &Connection, sql: &str) -> Vec<String> {
        let mut statement = connection.prepare(sql).unwrap();
        let rows = statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap();
        rows
    }

    fn temp_path(label: &str, extension: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("lumelo-{label}-{suffix}.{extension}"))
    }

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lumelo-{label}-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"lumelo").unwrap();
    }

    fn write_test_jpeg(path: &Path, width: u32, height: u32, rgb: [u8; 3]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        let image = RgbImage::from_pixel(width, height, Rgb(rgb));
        image.save(path).unwrap();
    }

    fn write_test_wav(path: &Path, sample_rate: u32, duration_ms: u32) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        let channel_count = 1u16;
        let bits_per_sample = 16u16;
        let bytes_per_sample = u32::from(bits_per_sample / 8);
        let frame_count = ((u64::from(sample_rate) * u64::from(duration_ms)) / 1000).max(1) as u32;
        let data_len = frame_count * u32::from(channel_count) * bytes_per_sample;
        let byte_rate = sample_rate * u32::from(channel_count) * bytes_per_sample;
        let block_align = channel_count * (bits_per_sample / 8);
        let riff_chunk_size = 36 + data_len;

        let mut bytes = Vec::with_capacity((44 + data_len) as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_chunk_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&channel_count.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.resize((44 + data_len) as usize, 0);

        fs::write(path, bytes).unwrap();
    }

    fn demo_volume() -> VolumeDescriptor {
        VolumeDescriptor {
            volume_uuid: "scan-demo".to_string(),
            label: "Demo Volume".to_string(),
            mount_path: "/media/demo".to_string(),
            fs_type: None,
        }
    }

    fn file_record(
        relative_path: &str,
        album_root_dir_hint: &str,
        metadata: ParsedAudioMetadata,
    ) -> FileScanRecord {
        let filename = Path::new(relative_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap()
            .to_string();
        FileScanRecord {
            relative_path: relative_path.to_string(),
            filename,
            derived_title: "Unknown".to_string(),
            derived_track_no: Some(1),
            directory_disc_no: 1,
            format: "flac".to_string(),
            file_mtime: 1,
            indexed_at: 1,
            size: 10,
            album_root_dir_hint: album_root_dir_hint.to_string(),
            metadata,
        }
    }

    fn cleanup_db_files(path: &PathBuf) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{}-wal", path.display()));
        let _ = fs::remove_file(format!("{}-shm", path.display()));
    }

    fn cleanup_dir(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }
}
