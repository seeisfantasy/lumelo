# AI Review Part 22

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `services/rust/crates/media-indexd/src/main.rs` (2/3)

- bytes: 85235
- segment: 2/3

~~~rust
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
        let parsed_metadata = parse_audio_metadata(&path);
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
        resolve_group_album_context, resolve_track_record, split_genres, table_count, Config,
        FileScanRecord, MediaIndexCommand, ParsedAudioMetadata, VolumeDescriptor,
        DEFAULT_LIBRARY_DB_PATH, UNKNOWN_ARTIST,
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

        touch(&root.join("Album One/01 - Opening.flac"));
        touch(&root.join("Album One/02_Afterglow.mp3"));
        touch(&root.join("Album Two/Disc 2/07 - Finale.wav"));
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

        touch(&root.join("Album/01 - First.flac"));
        touch(&root.join("Album/02 - Second.flac"));

        execute(Config {
            command: MediaIndexCommand::ScanDir {
                scan_root: root.clone(),
            },
            db_path: db_path.clone(),
            artwork_cache_dir: cache_dir.clone(),
        })
        .unwrap();

        fs::remove_file(root.join("Album/02 - Second.flac")).unwrap();

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

        touch(&root.join("Album With Cover/01 - Opening.flac"));
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
~~~

## `services/rust/crates/media-indexd/src/main.rs` (3/3)

- bytes: 85235
- segment: 3/3

~~~rust
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

        touch(&root.join("Priority Album/01 - Opening.flac"));
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
~~~

## `services/rust/crates/media-model/Cargo.toml`

- bytes: 223
- segment: 1/1

~~~toml
[package]
name = "media-model"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[lib]
path = "src/lib.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
~~~

## `services/rust/crates/media-model/src/lib.rs`

- bytes: 1797
- segment: 1/1

~~~rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderMode {
    Sequential,
    Shuffle,
}

impl Default for OrderMode {
    fn default() -> Self {
        Self::Sequential
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    Off,
    One,
    All,
}

impl Default for RepeatMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueueEntry {
    pub queue_entry_id: String,
    pub track_uid: String,
    pub volume_uuid: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub order_mode: OrderMode,
    pub repeat_mode: RepeatMode,
    pub current_order_index: Option<usize>,
    pub play_order: Vec<String>,
    pub tracks: Vec<QueueEntry>,
}

impl QueueSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub played_at: u64,
    pub track_uid: String,
    pub volume_uuid: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct HistoryLog {
    pub entries: Vec<HistoryEntry>,
}

impl HistoryLog {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn push_recent(&mut self, entry: HistoryEntry, limit: usize) {
        self.entries.insert(0, entry);
        self.entries.truncate(limit);
    }
}
~~~

## `services/rust/crates/playbackd/Cargo.toml`

- bytes: 481
- segment: 1/1

~~~toml
[package]
name = "playbackd"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
ipc-proto = { path = "../ipc-proto" }
media-model = { path = "../media-model" }
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
symphonia = { version = "0.5", default-features = false, features = ["aac", "flac", "isomp4", "mp3", "ogg", "pcm", "vorbis", "wav"] }
~~~

