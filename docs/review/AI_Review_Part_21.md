# AI Review Part 21

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `services/rust/crates/media-indexd/src/main.rs` (1/3)

- bytes: 85235
- segment: 1/3

~~~rust
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufWriter, Write};
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

    let mut grouped_files = BTreeMap::<String, Vec<FileScanRecord>>::new();
    for file in files {
        grouped_files
            .entry(file.album_root_dir_hint.clone())
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

    for (album_root_dir_hint, mut group_files) in grouped_files {
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

fn parse_audio_metadata(path: &Path) -> ParsedAudioMetadata {
    let probe = match Probe::open(path) {
        Ok(probe) => probe,
        Err(_) => return ParsedAudioMetadata::default(),
    };
    let tagged_file = match probe.read() {
        Ok(tagged_file) => tagged_file,
        Err(_) => return ParsedAudioMetadata::default(),
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

    metadata
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
~~~

