# AI Review Part 06

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `base/rootfs/overlay/usr/bin/lumelo-media-smoke` (1/2)

- bytes: 41956
- segment: 1/2

~~~text
#!/usr/bin/env python3
import argparse
import base64
import json
import math
import os
import shutil
import sqlite3
import struct
import subprocess
import time
import sys
import wave
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple


DEFAULT_DB = Path("/var/lib/lumelo/library.db")
DEFAULT_ROOT = Path("/var/lib/lumelo/test-media")
DEFAULT_DEVICE = os.environ.get("LUMELO_AUDIO_DEVICE", "default")
DEFAULT_PLAYBACK_SOCKET = Path("/run/lumelo/playback_cmd.sock")
DEFAULT_ARTWORK_CACHE_ROOT = Path("/var/cache/lumelo/artwork")
DECODED_FORMAT_PRIORITY = ("m4a", "flac", "ogg", "opus", "mp3", "aac")
EMBEDDED_JPEG_VARIANTS = {
    "folder": (
        "/9j/4AAQSkZJRgABAQAASABIAAD/4QBMRXhpZgAATU0AKgAAAAgAAYdpAAQAAAABAAAAGgAAAAAA"
        "A6ABAAMAAAABAAEAAKACAAQAAAABAAAAAqADAAQAAAABAAAAAgAAAAD/7QA4UGhvdG9zaG9wIDMu"
        "MAA4QklNBAQAAAAAAAA4QklNBCUAAAAAABDUHYzZjwCyBOmACZjs+EJ+/8AAEQgAAgACAwEiAAIR"
        "AQMRAf/EAB8AAAEFAQEBAQEBAAAAAAAAAAABAgMEBQYHCAkKC//EALUQAAIBAwMCBAMFBQQEAAAB"
        "fQECAwAEEQUSITFBBhNRYQcicRQygZGhCCNCscEVUtHwJDNicoIJChYXGBkaJSYnKCkqNDU2Nzg5"
        "OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6g4SFhoeIiYqSk5SVlpeYmZqio6Slpqeoqa"
        "qys7S1tre4ubrCw8TFxsfIycrS09TV1tfY2drh4uPk5ebn6Onq8fLz9PX29/j5+v/EAB8BAAMBAQEB"
        "AQEBAQEAAAAAAAABAgMEBQYHCAkKC//EALURAAIBAgQEAwQHBQQEAAECdwABAgMRBAUhMQYSQVEH"
        "YXETIjKBCBRCkaGxwQkjM1LwFWJy0QoWJDThJfEXGBkaJicoKSo1Njc4OTpDREVGR0hJSlNUVVZX"
        "WFlaY2RlZmdoaWpzdHV2d3h5eoKDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLD"
        "xMXGx8jJytLT1NXW19jZ2uLj5OXm5+jp6vLz9PX29/j5+v/bAEMAAgICAgICAwICAwUDAwMFBgUF"
        "BQUGCAYGBgYGCAoICAgICAgKCgoKCgoKCgwMDAwMDA4ODg4ODw8PDw8PDw8PD//bAEMBAgICBAQE"
        "BwQEBxALCQsQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB"
        "AQEP/dAAQAAf/aAAwDAQACEQMRAD8A+L6KKK/lM/38P//Z"
    ),
    "cover": (
        "/9j/4AAQSkZJRgABAQAASABIAAD/4QBMRXhpZgAATU0AKgAAAAgAAYdpAAQAAAABAAAAGgAAAAAA"
        "A6ABAAMAAAABAAEAAKACAAQAAAABAAAAAqADAAQAAAABAAAAAgAAAAD/7QA4UGhvdG9zaG9wIDMu"
        "MAA4QklNBAQAAAAAAAA4QklNBCUAAAAAABDUHYzZjwCyBOmACZjs+EJ+/8AAEQgAAgACAwEiAAIR"
        "AQMRAf/EAB8AAAEFAQEBAQEBAAAAAAAAAAABAgMEBQYHCAkKC//EALUQAAIBAwMCBAMFBQQEAAAB"
        "fQECAwAEEQUSITFBBhNRYQcicRQygZGhCCNCscEVUtHwJDNicoIJChYXGBkaJSYnKCkqNDU2Nzg5"
        "OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6g4SFhoeIiYqSk5SVlpeYmZqio6Slpqeoqa"
        "qys7S1tre4ubrCw8TFxsfIycrS09TV1tfY2drh4uPk5ebn6Onq8fLz9PX29/j5+v/EAB8BAAMBAQEB"
        "AQEBAQEAAAAAAAABAgMEBQYHCAkKC//EALURAAIBAgQEAwQHBQQEAAECdwABAgMRBAUhMQYSQVEH"
        "YXETIjKBCBRCkaGxwQkjM1LwFWJy0QoWJDThJfEXGBkaJicoKSo1Njc4OTpDREVGR0hJSlNUVVZX"
        "WFlaY2RlZmdoaWpzdHV2d3h5eoKDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLD"
        "xMXGx8jJytLT1NXW19jZ2uLj5OXm5+jp6vLz9PX29/j5+v/bAEMAAgICAgICAwICAwUDAwMFBgUF"
        "BQUGCAYGBgYGCAoICAgICAgKCgoKCgoKCgwMDAwMDA4ODg4ODw8PDw8PDw8PD//bAEMBAgICBAQE"
        "BwQEBxALCQsQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEB"
        "AQEP/dAAQAAf/aAAwDAQACEQMRAD8A/HOiiiv9/D8rP//Z"
    ),
}


def run(argv: List[str], check: bool = True) -> subprocess.CompletedProcess:
    print("$", " ".join(argv))
    return subprocess.run(argv, check=check, text=True)


def generate_demo_wav(root: Path, seconds: int, frequency: float) -> Path:
    album_dir = root / "Blue Room Sessions"
    track_path = album_dir / "01 - Warmup Tone.wav"
    album_dir.mkdir(parents=True, exist_ok=True)

    sample_rate = 44100
    amplitude = 0.30

    with wave.open(str(track_path), "wb") as wav_file:
        wav_file.setnchannels(2)
        wav_file.setsampwidth(2)
        wav_file.setframerate(sample_rate)
        for i in range(sample_rate * seconds):
            sample = int(32767 * amplitude * math.sin(2 * math.pi * frequency * i / sample_rate))
            frame = struct.pack("<hh", sample, sample)
            wav_file.writeframesraw(frame)

    return track_path


def ensure_db(path: Path) -> sqlite3.Connection:
    if not path.exists():
        raise SystemExit(f"library db does not exist: {path}")
    return sqlite3.connect(str(path))


def scan_dir(root: Path) -> None:
    run(["media-indexd", "scan-dir", str(root)])


def query_tracks(
    connection: sqlite3.Connection,
    *,
    track_uid: Optional[str] = None,
    relative_path: Optional[str] = None,
    first_wav: bool = False,
    limit: int = 10,
) -> List[sqlite3.Row]:
    connection.row_factory = sqlite3.Row
    sql = """
        SELECT
            t.track_uid,
            COALESCE(NULLIF(t.title, ''), t.filename) AS title,
            t.relative_path,
            COALESCE(LOWER(t.format), '') AS format,
            COALESCE(t.duration_ms, 0) AS duration_ms,
            t.volume_uuid,
            v.mount_path
        FROM tracks t
        JOIN volumes v ON v.volume_uuid = t.volume_uuid
    """
    clauses = []
    params = []

    if track_uid:
        clauses.append("t.track_uid = ?")
        params.append(track_uid)
    if relative_path:
        clauses.append("t.relative_path = ?")
        params.append(relative_path)
    if first_wav:
        clauses.append("LOWER(COALESCE(t.format, '')) = 'wav'")

    if clauses:
        sql += " WHERE " + " AND ".join(clauses)
    sql += " ORDER BY t.indexed_at DESC, t.track_uid ASC LIMIT ?"
    params.append(limit)

    return list(connection.execute(sql, params))


def query_tracks_from_mount(
    connection: sqlite3.Connection,
    mount_path: Path,
    *,
    limit: int = 50,
) -> List[sqlite3.Row]:
    connection.row_factory = sqlite3.Row
    sql = """
        SELECT
            t.track_uid,
            COALESCE(NULLIF(t.title, ''), t.filename) AS title,
            t.relative_path,
            COALESCE(LOWER(t.format), '') AS format,
            COALESCE(t.duration_ms, 0) AS duration_ms,
            t.volume_uuid,
            v.mount_path
        FROM tracks t
        JOIN volumes v ON v.volume_uuid = t.volume_uuid
        WHERE v.mount_path = ?
        ORDER BY t.relative_path ASC, t.track_uid ASC
        LIMIT ?
    """
    return list(connection.execute(sql, (str(mount_path), limit)))


def absolute_track_path(row: sqlite3.Row) -> Path:
    return Path(row["mount_path"]) / row["relative_path"]


def print_tracks(rows: List[sqlite3.Row]) -> None:
    if not rows:
        print("No tracks found.")
        return

    for row in rows:
        print(
            f"- {row['track_uid']} | {row['title']} | {row['format'] or '-'} | "
            f"{row['relative_path']} | {absolute_track_path(row)}"
        )


def request_playback(command: str, socket_path: Path) -> str:
    import socket

    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        sock.connect(str(socket_path))
        sock.sendall((command + "\n").encode())
        chunks = []
        while True:
            chunk = sock.recv(65536)
            if not chunk:
                break
            chunks.append(chunk)
            if chunk.endswith(b"\n"):
                break
    finally:
        sock.close()

    return b"".join(chunks).decode().strip()


def parse_fields(line: str) -> Tuple[str, Dict[str, str]]:
    parts = line.split("\t")
    fields: Dict[str, str] = {}
    for token in parts[1:]:
        if "=" in token:
            key, value = token.split("=", 1)
            fields[key] = value
    return parts[0], fields


def find_first_track_by_format(
    connection: sqlite3.Connection, formats: Sequence[str]
) -> Optional[sqlite3.Row]:
    format_set = [value.lower() for value in formats if value]
    if not format_set:
        return None

    connection.row_factory = sqlite3.Row
    placeholders = ",".join("?" for _ in format_set)
    sql = f"""
        SELECT
            t.track_uid,
            COALESCE(NULLIF(t.title, ''), t.filename) AS title,
            t.relative_path,
            COALESCE(LOWER(t.format), '') AS format,
            COALESCE(t.duration_ms, 0) AS duration_ms,
            t.volume_uuid,
            v.mount_path
        FROM tracks t
        JOIN volumes v ON v.volume_uuid = t.volume_uuid
        WHERE LOWER(COALESCE(t.format, '')) IN ({placeholders})
        ORDER BY t.indexed_at DESC, t.relative_path ASC
        LIMIT 1
    """
    return connection.execute(sql, format_set).fetchone()


def find_first_track_by_format_from_mount(
    connection: sqlite3.Connection, mount_path: Path, formats: Sequence[str]
) -> Optional[sqlite3.Row]:
    format_set = [value.lower() for value in formats if value]
    if not format_set:
        return None

    connection.row_factory = sqlite3.Row
    placeholders = ",".join("?" for _ in format_set)
    sql = f"""
        SELECT
            t.track_uid,
            COALESCE(NULLIF(t.title, ''), t.filename) AS title,
            t.relative_path,
            COALESCE(LOWER(t.format), '') AS format,
            COALESCE(t.duration_ms, 0) AS duration_ms,
            t.volume_uuid,
            v.mount_path
        FROM tracks t
        JOIN volumes v ON v.volume_uuid = t.volume_uuid
        WHERE v.mount_path = ?
          AND LOWER(COALESCE(t.format, '')) IN ({placeholders})
        ORDER BY t.indexed_at DESC, t.relative_path ASC
        LIMIT 1
    """
    params: List[str] = [str(mount_path)] + format_set
    return connection.execute(sql, params).fetchone()


def current_aplay() -> str:
    completed = subprocess.run(["pgrep", "-a", "aplay"], capture_output=True, text=True)
    return completed.stdout.strip()


def playback_ready(socket_path: Path) -> bool:
    try:
        request_playback("STATUS", socket_path)
    except OSError:
        return False
    return True


def wait_for_playback_ready(socket_path: Path, timeout: float = 10.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if socket_path.exists() and playback_ready(socket_path):
            return
        time.sleep(0.2)
    raise SystemExit(f"timed out waiting for playback socket readiness: {socket_path}")


def wait_for_status(
    socket_path: Path,
    *,
    expected_state: Optional[str] = None,
    expected_track: Optional[str] = None,
    timeout: float = 5.0,
) -> Dict[str, str]:
    deadline = time.monotonic() + timeout
    last_fields: Dict[str, str] = {}

    while time.monotonic() < deadline:
        line = request_playback("STATUS", socket_path)
        _, fields = parse_fields(line)
        last_fields = fields
        state_ok = expected_state is None or fields.get("state") == expected_state
        track_ok = expected_track is None or fields.get("current_track") == expected_track
        if state_ok and track_ok:
            return fields
        time.sleep(0.2)

    raise SystemExit(
        "timed out waiting for playback status "
        + json.dumps(last_fields, ensure_ascii=False, sort_keys=True)
    )


def require_ack(line: str, action: str) -> Dict[str, str]:
    prefix, fields = parse_fields(line)
    if prefix != "OK":
        raise SystemExit(f"{action} failed: {line}")
    return fields


def require_service_active(name: str) -> None:
    completed = subprocess.run(
        ["systemctl", "is-active", name],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit(f"service {name} is not active: {completed.stdout.strip()}")


def track_label(row: sqlite3.Row) -> str:
    return f"{row['track_uid']} | {row['title']} | {row['format'] or '-'}"


def copy_track_fixture(source_row: sqlite3.Row, destination: Path) -> None:
    source_path = absolute_track_path(source_row)
    if not source_path.exists():
        raise SystemExit(f"source fixture is missing: {source_path}")

    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source_path, destination)


def write_embedded_jpeg(destination: Path, variant: str) -> None:
    payload = EMBEDDED_JPEG_VARIANTS.get(variant)
    if payload is None:
        raise SystemExit(f"unknown embedded jpeg variant: {variant}")

    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(base64.b64decode(payload))


def write_broken_media_fixture(destination: Path, payload: bytes) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(payload)


def command_regress_library_scan(args: argparse.Namespace) -> int:
    source_root = Path(args.source_root)
    suite_root = Path(args.root)
    artwork_cache_root = Path(args.artwork_cache_root)
    if suite_root == source_root:
        print("regression root must differ from source root", file=sys.stderr)
        return 64

    if suite_root.exists():
        shutil.rmtree(suite_root)
    suite_root.mkdir(parents=True, exist_ok=True)

    plan = [
        ("Resident Testers/Album Alpha/01 - Warmup Tone", ("wav",)),
        ("Resident Testers/Album Alpha/02 - Decoder Check", ("m4a", "aac")),
        ("Guest Stack/Album Beta/01 - Decoder Check", ("flac",)),
        ("Guest Stack/Album Beta/02 - Remote Sample", ("mp3",)),
        ("Guest Stack/Album Gamma/01 - Remote Sample", ("ogg",)),
    ]
    cover_fixtures = [
        ("Resident Testers/Album Alpha/folder.jpg", "folder"),
        ("Resident Testers/Album Alpha/cover.jpg", "cover"),
        ("Guest Stack/Album Beta/cover.jpg", "cover"),
    ]

    copied_files: List[Tuple[str, str, Path]] = []
    missing_formats: List[str] = []
    with ensure_db(Path(args.db)) as connection:
        for stem, formats in plan:
            row = find_first_track_by_format_from_mount(connection, source_root, formats)
            if row is None:
                missing_formats.append("/".join(formats))
                continue

            suffix = absolute_track_path(row).suffix.lower() or f".{row['format']}"
            destination = suite_root / f"{stem}{suffix}"
            copy_track_fixture(row, destination)
            copied_files.append((row["track_uid"], row["format"], destination))

    if len(copied_files) < args.min_tracks:
        print(
            "not enough source fixtures to run library scan regression; "
            f"copied={len(copied_files)} missing={missing_formats}",
            file=sys.stderr,
        )
        return 65

    (suite_root / "README.txt").write_text(
        "Lumelo batch scan regression fixture. This file should be ignored by media-indexd.\n",
        encoding="utf-8",
    )
    for relative_path, variant in cover_fixtures:
        write_embedded_jpeg(suite_root / relative_path, variant)

    print("Created library scan fixture set:")
    for track_uid, fmt, destination in copied_files:
        print(f"- {fmt or '-'} | {track_uid} | {destination}")
    print("Created artwork fixtures:")
    for relative_path, variant in cover_fixtures:
        print(f"- {variant} | {suite_root / relative_path}")

    scan_dir(suite_root)

    with ensure_db(Path(args.db)) as connection:
        connection.row_factory = sqlite3.Row
        rows = list(
            connection.execute(
                """
                SELECT
                    t.track_uid,
                    t.relative_path,
                    COALESCE(NULLIF(t.title, ''), t.filename) AS title,
                    LOWER(COALESCE(t.format, '')) AS format,
                    v.mount_path
                FROM tracks t
                JOIN volumes v ON v.volume_uuid = t.volume_uuid
                WHERE v.mount_path = ?
                ORDER BY t.relative_path ASC
                """,
                (str(suite_root),),
            )
        )
        album_rows = list(
            connection.execute(
                """
                SELECT
                    albums.album_title,
                    albums.track_count,
                    COALESCE(artwork_refs.source_rel_path, '') AS source_rel_path,
                    COALESCE(artwork_refs.thumb_rel_path, '') AS thumb_rel_path
                FROM albums
                JOIN volumes v ON v.volume_uuid = albums.volume_uuid
                LEFT JOIN artwork_refs ON artwork_refs.artwork_ref_id = albums.cover_ref_id
                WHERE v.mount_path = ?
                ORDER BY albums.album_title ASC
                """,
                (str(suite_root),),
            )
        )
        covered_tracks = int(
            connection.execute(
                """
                SELECT COUNT(*)
                FROM tracks t
                JOIN volumes v ON v.volume_uuid = t.volume_uuid
                WHERE v.mount_path = ?
                  AND t.cover_ref_id IS NOT NULL
                """,
                (str(suite_root),),
            ).fetchone()[0]
        )

    print("\nIndexed regression tracks:")
    for row in rows:
        print(
            f"- {row['format'] or '-'} | {row['track_uid']} | "
            f"{row['relative_path']} | {row['title']}"
        )
    print("\nIndexed regression albums:")
    for row in album_rows:
        coverage = "cover" if row["source_rel_path"] else "no-cover"
        print(
            f"- {row['album_title']} | tracks={row['track_count']} | {coverage}"
        )

    expected_count = len(copied_files)
    if len(rows) != expected_count:
        print(
            f"expected {expected_count} indexed tracks under {suite_root}, got {len(rows)}",
            file=sys.stderr,
        )
        return 65

    indexed_formats = {row["format"] for row in rows}
    expected_formats = {fmt for _, fmt, _ in copied_files}
    if indexed_formats != expected_formats:
        print(
            "indexed format set mismatch: "
            + json.dumps(
                {
                    "expected": sorted(expected_formats),
                    "got": sorted(indexed_formats),
                },
                ensure_ascii=False,
            ),
            file=sys.stderr,
        )
        return 65

    parent_dirs = {str(Path(row["relative_path"]).parent) for row in rows}
    if len(parent_dirs) < args.min_directories:
        print(
            f"expected at least {args.min_directories} distinct directories, got {len(parent_dirs)}",
            file=sys.stderr,
        )
        return 65

    expected_album_track_counts: Dict[str, int] = {}
    for _, _, destination in copied_files:
        album_title = destination.parent.name
        expected_album_track_counts[album_title] = (
            expected_album_track_counts.get(album_title, 0) + 1
        )
    if len(album_rows) < args.min_albums:
        print(
            f"expected at least {args.min_albums} albums, got {len(album_rows)}",
            file=sys.stderr,
        )
        return 65

    actual_album_rows = {str(row["album_title"]): row for row in album_rows}
    if set(actual_album_rows) != set(expected_album_track_counts):
        print(
            "indexed album title set mismatch: "
            + json.dumps(
                {
                    "expected": sorted(expected_album_track_counts),
                    "got": sorted(actual_album_rows),
                },
                ensure_ascii=False,
            ),
            file=sys.stderr,
        )
        return 65

    for album_title, expected_tracks in sorted(expected_album_track_counts.items()):
        actual_tracks = int(actual_album_rows[album_title]["track_count"])
        if actual_tracks != expected_tracks:
            print(
                f"album {album_title} expected {expected_tracks} tracks, got {actual_tracks}",
                file=sys.stderr,
            )
            return 65

    expected_cover_sources = {
        "Album Alpha": suite_root / "Resident Testers/Album Alpha/folder.jpg",
        "Album Beta": suite_root / "Guest Stack/Album Beta/cover.jpg",
    }
    expected_covered_tracks = sum(
        expected_album_track_counts.get(album_title, 0)
        for album_title in expected_cover_sources
    )
    if covered_tracks != expected_covered_tracks:
        print(
            f"expected {expected_covered_tracks} cover-linked tracks, got {covered_tracks}",
            file=sys.stderr,
        )
        return 65

    for album_title, expected_fixture in sorted(expected_cover_sources.items()):
        row = actual_album_rows.get(album_title)
        if row is None:
            print(f"missing expected covered album: {album_title}", file=sys.stderr)
            return 65
        source_rel_path = str(row["source_rel_path"])
        thumb_rel_path = str(row["thumb_rel_path"])
        if not source_rel_path or not thumb_rel_path:
            print(
                f"album {album_title} is missing indexed artwork paths",
                file=sys.stderr,
            )
            return 65
        cached_source = artwork_cache_root / source_rel_path
        cached_thumb = artwork_cache_root / thumb_rel_path
        if not cached_source.is_file():
            print(f"indexed artwork source is missing: {cached_source}", file=sys.stderr)
            return 65
        if not cached_thumb.is_file():
            print(f"indexed artwork thumb is missing: {cached_thumb}", file=sys.stderr)
            return 65
        if cached_source.read_bytes() != expected_fixture.read_bytes():
            print(
                f"album {album_title} artwork content mismatch; expected {expected_fixture.name}",
                file=sys.stderr,
            )
            return 65

    uncovered_album = actual_album_rows.get("Album Gamma")
    if uncovered_album is not None and str(uncovered_album["source_rel_path"]):
        print(
            "Album Gamma unexpectedly received artwork coverage",
            file=sys.stderr,
        )
        return 65

    print(
        "\nLibrary scan regression passed: "
        + json.dumps(
            {
                "albums": len(album_rows),
                "covered_tracks": covered_tracks,
                "tracks": len(rows),
                "formats": sorted(indexed_formats),
                "directories": len(parent_dirs),
                "mount_path": str(suite_root),
            },
            ensure_ascii=False,
            sort_keys=True,
        )
    )
    return 0


def command_regress_bad_media(args: argparse.Namespace) -> int:
    source_root = Path(args.source_root)
    suite_root = Path(args.root)
    socket_path = Path(args.socket)

    if suite_root == source_root:
        print("bad-media regression root must differ from source root", file=sys.stderr)
        return 64
    if not socket_path.exists():
        print(f"playback socket does not exist: {socket_path}", file=sys.stderr)
        return 66

    if suite_root.exists():
        shutil.rmtree(suite_root)
    suite_root.mkdir(parents=True, exist_ok=True)

    with ensure_db(Path(args.db)) as connection:
        source_row = find_first_track_by_format_from_mount(
            connection,
            source_root,
            ("mp3", "m4a", "flac", "ogg", "wav"),
        )
    if source_row is None:
        print(
            f"no indexed source track found under {source_root} for bad-media regression",
            file=sys.stderr,
        )
        return 65

    good_name = f"01 - Good {source_row['title']}{absolute_track_path(source_row).suffix.lower()}"
    good_relative = Path("Bad Inputs") / good_name
    copy_track_fixture(source_row, suite_root / good_relative)
    write_broken_media_fixture(
        suite_root / "Bad Inputs/02 - Broken.mp3",
        b"not really an mp3\n",
    )
    write_broken_media_fixture(
        suite_root / "Bad Inputs/03 - Broken.flac",
        b"fLaCbroken",
    )
    write_broken_media_fixture(
        suite_root / "Bad Inputs/04 - Broken.ogg",
        b"OggSbad",
    )

    scan_dir(suite_root)

    with ensure_db(Path(args.db)) as connection:
        rows = query_tracks_from_mount(connection, suite_root, limit=20)

    print("Indexed bad-media regression tracks:")
    for row in rows:
        print(f"- {track_label(row)} | {row['relative_path']}")

    good_row = next((row for row in rows if row["relative_path"] == str(good_relative)), None)
    if good_row is None:
        print("valid regression track was not indexed", file=sys.stderr)
        return 65

    broken_rows = [row for row in rows if "Broken" in row["relative_path"]]
    require_ack(request_playback("STOP", socket_path), "STOP bad-media")
    require_ack(request_playback("QUEUE_CLEAR", socket_path), "QUEUE_CLEAR bad-media")

    broken_result = "not-indexed"
    if broken_rows:
        broken_line = request_playback(f"PLAY {broken_rows[0]['track_uid']}", socket_path)
        broken_prefix, _ = parse_fields(broken_line)
        broken_result = broken_prefix
        time.sleep(min(max(args.timeout, 1.0), 2.0))
        broken_status = request_playback("STATUS", socket_path)
        _, broken_status_fields = parse_fields(broken_status)
        print(f"Broken status: {broken_status}")
        if broken_prefix not in {"ERR", "OK"}:
            raise SystemExit(f"unexpected broken playback response: {broken_line}")
        if broken_prefix == "OK" and broken_status_fields.get("state") == "quiet_active":
            raise SystemExit("broken media unexpectedly entered active playback")
        require_service_active("playbackd.service")

    require_ack(
        request_playback(f"PLAY {good_row['track_uid']}", socket_path),
        "PLAY good track after broken media",
    )
    wait_for_status(
        socket_path,
        expected_state="quiet_active",
        expected_track=good_row["track_uid"],
        timeout=args.timeout,
    )
    require_ack(request_playback("STOP", socket_path), "STOP recovery track")
    wait_for_status(
        socket_path,
        expected_state="stopped",
        expected_track=good_row["track_uid"],
        timeout=args.timeout,
    )

    print(
        "\nBad-media regression passed: "
        + json.dumps(
            {
                "good_track": good_row["track_uid"],
                "indexed_rows": len(rows),
                "broken_rows": len(broken_rows),
                "broken_result": broken_result,
            },
            ensure_ascii=False,
            sort_keys=True,
        )
    )
    return 0


def command_regress_playback(args: argparse.Namespace) -> int:
    socket_path = Path(args.socket)
    if not socket_path.exists():
        print(f"playback socket does not exist: {socket_path}", file=sys.stderr)
        return 66

    with ensure_db(Path(args.db)) as connection:
        decoded_formats = (
            (args.decoded_format.lower(),)
            if args.decoded_format
            else DECODED_FORMAT_PRIORITY
        )
        if args.mount_root:
            mount_root = Path(args.mount_root)
            wav_row = find_first_track_by_format_from_mount(connection, mount_root, ("wav",))
            decoded_row = find_first_track_by_format_from_mount(
                connection, mount_root, decoded_formats
            )
        else:
            wav_row = find_first_track_by_format(connection, ("wav",))
            decoded_row = find_first_track_by_format(connection, decoded_formats)

    if wav_row is None:
        print("no indexed wav track found; cannot run playback regression", file=sys.stderr)
        return 65

    print("Resolved playback regression tracks:")
    print(f"- wav:     {track_label(wav_row)}")
    if decoded_row is not None:
        print(f"- decoded: {track_label(decoded_row)}")
    else:
        print("- decoded: (none found, decoded-path checks will be skipped)")

    require_ack(request_playback("STOP", socket_path), "STOP")
    require_ack(request_playback("QUEUE_CLEAR", socket_path), "QUEUE_CLEAR")

    print("\n[wav] play / pause / resume / stop")
    require_ack(request_playback(f"PLAY {wav_row['track_uid']}", socket_path), "PLAY wav")
    wait_for_status(
        socket_path,
        expected_state="quiet_active",
        expected_track=wav_row["track_uid"],
        timeout=args.timeout,
    )
    print(f"  aplay: {current_aplay() or '(none)'}")

    require_ack(request_playback("PAUSE", socket_path), "PAUSE wav")
    wait_for_status(
        socket_path,
        expected_state="paused",
        expected_track=wav_row["track_uid"],
        timeout=args.timeout,
    )
    print(f"  paused aplay: {current_aplay() or '(none)'}")

    require_ack(request_playback(f"PLAY {wav_row['track_uid']}", socket_path), "RESUME wav")
    wait_for_status(
        socket_path,
        expected_state="quiet_active",
        expected_track=wav_row["track_uid"],
        timeout=args.timeout,
    )

    require_ack(request_playback("STOP", socket_path), "STOP wav")
    wait_for_status(
        socket_path,
        expected_state="stopped",
        expected_track=wav_row["track_uid"],
        timeout=args.timeout,
    )

    if decoded_row is not None:
        print("\n[decoded] play / pause / resume / stop")
        require_ack(
            request_playback(f"PLAY {decoded_row['track_uid']}", socket_path),
            "PLAY decoded",
        )
        wait_for_status(
            socket_path,
            expected_state="quiet_active",
            expected_track=decoded_row["track_uid"],
            timeout=args.timeout,
        )
        print(f"  aplay: {current_aplay() or '(none)'}")

        require_ack(request_playback("PAUSE", socket_path), "PAUSE decoded")
        wait_for_status(
            socket_path,
            expected_state="paused",
            expected_track=decoded_row["track_uid"],
            timeout=args.timeout,
        )

        require_ack(
            request_playback(f"PLAY {decoded_row['track_uid']}", socket_path),
            "RESUME decoded",
        )
        wait_for_status(
            socket_path,
            expected_state="quiet_active",
            expected_track=decoded_row["track_uid"],
            timeout=args.timeout,
        )

        require_ack(request_playback("STOP", socket_path), "STOP decoded")
        wait_for_status(
            socket_path,
            expected_state="stopped",
            expected_track=decoded_row["track_uid"],
            timeout=args.timeout,
        )

        print("\n[mixed queue] decoded -> wav auto-next")
        require_ack(request_playback("QUEUE_CLEAR", socket_path), "QUEUE_CLEAR mixed")
        require_ack(
            request_playback(f"PLAY {decoded_row['track_uid']}", socket_path),
            "PLAY mixed decoded",
        )
        require_ack(
            request_playback(f"QUEUE_APPEND {wav_row['track_uid']}", socket_path),
            "QUEUE_APPEND mixed wav",
        )
        wait_for_status(
            socket_path,
            expected_state="quiet_active",
            expected_track=decoded_row["track_uid"],
            timeout=args.timeout,
        )
        print(f"  start aplay: {current_aplay() or '(none)'}")

        auto_next_timeout = max(args.timeout, 6.0)
        if args.skip_mixed:
            print("  mixed queue auto-next skipped by request")
        else:
            wait_for_status(
                socket_path,
                expected_state="quiet_active",
                expected_track=wav_row["track_uid"],
                timeout=auto_next_timeout,
            )
            print(f"  auto-next aplay: {current_aplay() or '(none)'}")

        require_ack(request_playback("STOP", socket_path), "STOP mixed")
        expected_stop_track = decoded_row["track_uid"] if args.skip_mixed else wav_row["track_uid"]
        wait_for_status(
            socket_path,
            expected_state="stopped",
            expected_track=expected_stop_track,
            timeout=args.timeout,
        )

    print("\nPlayback regression passed.")
    return 0


def command_regress_playbackd_restart(args: argparse.Namespace) -> int:
    socket_path = Path(args.socket)
    if not socket_path.exists():
        print(f"playback socket does not exist: {socket_path}", file=sys.stderr)
        return 66

    with ensure_db(Path(args.db)) as connection:
        if args.mount_root:
            mount_root = Path(args.mount_root)
            rows = query_tracks_from_mount(connection, mount_root, limit=4)
        else:
            rows = query_tracks(connection, limit=4)

    if len(rows) < 2:
        print("need at least two indexed tracks to run playbackd restart regression", file=sys.stderr)
        return 65

    current_row = rows[0]
    queued_row = next((row for row in rows[1:] if row["track_uid"] != current_row["track_uid"]), None)
    if queued_row is None:
        print("could not resolve a second distinct track for playbackd restart regression", file=sys.stderr)
        return 65

    print("Resolved playbackd restart tracks:")
    print(f"- current: {track_label(current_row)}")
    print(f"- queued:  {track_label(queued_row)}")

    require_ack(request_playback("STOP", socket_path), "STOP restart")
    require_ack(request_playback("QUEUE_CLEAR", socket_path), "QUEUE_CLEAR restart")
    require_ack(
        request_playback(f"PLAY {current_row['track_uid']}", socket_path),
        "PLAY restart current",
    )
    require_ack(
        request_playback(f"QUEUE_APPEND {queued_row['track_uid']}", socket_path),
        "QUEUE_APPEND restart queued",
    )
    wait_for_status(
        socket_path,
        expected_state="quiet_active",
        expected_track=current_row["track_uid"],
        timeout=args.timeout,
    )

    before_status_line = request_playback("STATUS", socket_path)
    before_queue_line = request_playback("QUEUE_SNAPSHOT", socket_path)
    _, before_status_fields = parse_fields(before_status_line)
    _, before_queue_fields = parse_fields(before_queue_line)
    before_entries = json.loads(before_queue_fields["payload"])["entries"]

    run(["systemctl", "restart", "playbackd.service"])
    wait_for_playback_ready(socket_path, timeout=max(args.timeout, 8.0))
    require_service_active("playbackd.service")

    after_status_line = request_playback("STATUS", socket_path)
    after_queue_line = request_playback("QUEUE_SNAPSHOT", socket_path)
    _, after_status_fields = parse_fields(after_status_line)
    _, after_queue_fields = parse_fields(after_queue_line)
    after_entries = json.loads(after_queue_fields["payload"])["entries"]

    before_uids = [entry["track_uid"] for entry in before_entries]
    after_uids = [entry["track_uid"] for entry in after_entries]
    if before_uids != after_uids:
        raise SystemExit(
            "playback queue changed across restart: "
            + json.dumps({"before": before_uids, "after": after_uids}, ensure_ascii=False)
        )
    if after_status_fields.get("current_track") != current_row["track_uid"]:
        raise SystemExit(
            "playback current_track changed across restart: "
            + json.dumps(
                {
                    "before": before_status_fields.get("current_track"),
                    "after": after_status_fields.get("current_track"),
                },
                ensure_ascii=False,
            )
        )

    print(
        "\nPlaybackd restart regression passed: "
        + json.dumps(
            {
                "current_track": after_status_fields.get("current_track"),
                "state_after_restart": after_status_fields.get("state"),
                "queue_entries": len(after_entries),
            },
            ensure_ascii=False,
            sort_keys=True,
        )
    )
    return 0


def play_row(row: sqlite3.Row, device: str) -> int:
    abs_path = absolute_track_path(row)
    if not abs_path.exists():
        print(f"resolved media path is missing: {abs_path}", file=sys.stderr)
        return 66

    print(f"track_uid: {row['track_uid']}")
    print(f"title:     {row['title']}")
    print(f"format:    {row['format'] or '-'}")
    print(f"path:      {abs_path}")
    completed = run(["aplay", "-D", device, str(abs_path)], check=False)
    return completed.returncode


def command_smoke(args: argparse.Namespace) -> int:
    root = Path(args.root)
    track_path = generate_demo_wav(root, args.seconds, args.frequency)
    print(f"Generated demo WAV: {track_path}")
    scan_dir(root)

    with ensure_db(Path(args.db)) as connection:
        rows = query_tracks(connection, relative_path=str(track_path.relative_to(root)))
        print_tracks(rows)
        if not rows:
            print("demo track was not indexed", file=sys.stderr)
            return 65
        if args.skip_play:
            return 0
        return play_row(rows[0], args.device)


def command_list(args: argparse.Namespace) -> int:
    with ensure_db(Path(args.db)) as connection:
        rows = query_tracks(connection, first_wav=args.first_wav, limit=args.limit)
        print_tracks(rows)
    return 0


def command_play(args: argparse.Namespace) -> int:
    with ensure_db(Path(args.db)) as connection:
        rows = query_tracks(
            connection,
            track_uid=args.track_uid,
            relative_path=args.relative_path,
            first_wav=args.first_wav,
            limit=1,
        )
        if not rows:
            print("no matching track found", file=sys.stderr)
            return 65
        return play_row(rows[0], args.device)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Lumelo real-media smoke helper for index + ALSA playback validation."
    )
    subparsers = parser.add_subparsers(dest="command")

    smoke = subparsers.add_parser("smoke", help="Generate a demo WAV, scan it, then play it.")
    smoke.add_argument("--db", default=str(DEFAULT_DB))
    smoke.add_argument("--root", default=str(DEFAULT_ROOT))
    smoke.add_argument("--device", default=DEFAULT_DEVICE)
    smoke.add_argument("--seconds", type=int, default=3)
    smoke.add_argument("--frequency", type=float, default=440.0)
    smoke.add_argument("--skip-play", action="store_true")
    smoke.set_defaults(func=command_smoke)

    list_cmd = subparsers.add_parser("list", help="List recently indexed tracks.")
    list_cmd.add_argument("--db", default=str(DEFAULT_DB))
    list_cmd.add_argument("--limit", type=int, default=10)
    list_cmd.add_argument("--first-wav", action="store_true")
~~~

## `base/rootfs/overlay/usr/bin/lumelo-media-smoke` (2/2)

- bytes: 41956
- segment: 2/2

~~~text
    list_cmd.set_defaults(func=command_list)

    play = subparsers.add_parser("play", help="Resolve one indexed track and play it via aplay.")
    play.add_argument("--db", default=str(DEFAULT_DB))
    play.add_argument("--device", default=DEFAULT_DEVICE)
    play.add_argument("--track-uid")
    play.add_argument("--relative-path")
    play.add_argument("--first-wav", action="store_true")
    play.set_defaults(func=command_play)

    regress = subparsers.add_parser(
        "regress-playback",
        help="Run board-local playbackd regression against indexed real tracks.",
    )
    regress.add_argument("--db", default=str(DEFAULT_DB))
    regress.add_argument("--socket", default=str(DEFAULT_PLAYBACK_SOCKET))
    regress.add_argument(
        "--mount-root",
        help="Restrict regression track selection to one indexed mount path.",
    )
    regress.add_argument("--timeout", type=float, default=5.0)
    regress.add_argument(
        "--decoded-format",
        help="Prefer one decoded format explicitly, such as m4a or flac.",
    )
    regress.add_argument(
        "--skip-mixed",
        action="store_true",
        help="Skip the decoded->wav auto-next step for long-form decoded tracks.",
    )
    regress.set_defaults(func=command_regress_playback)

    scan_regress = subparsers.add_parser(
        "regress-library-scan",
        help="Build a multi-directory fixture tree and verify media-indexd batch scan output.",
    )
    scan_regress.add_argument("--db", default=str(DEFAULT_DB))
    scan_regress.add_argument("--source-root", default=str(DEFAULT_ROOT))
    scan_regress.add_argument(
        "--artwork-cache-root",
        default=str(DEFAULT_ARTWORK_CACHE_ROOT),
        help="Artwork cache root used by media-indexd for copied source/thumb files.",
    )
    scan_regress.add_argument(
        "--root",
        default=str(Path("/var/lib/lumelo/test-media-batch")),
        help="Dedicated root for generated batch-scan fixtures.",
    )
    scan_regress.add_argument("--min-tracks", type=int, default=3)
    scan_regress.add_argument("--min-directories", type=int, default=3)
    scan_regress.add_argument("--min-albums", type=int, default=3)
    scan_regress.set_defaults(func=command_regress_library_scan)

    bad_media = subparsers.add_parser(
        "regress-bad-media",
        help="Index malformed audio-like files and verify playbackd recovers cleanly.",
    )
    bad_media.add_argument("--db", default=str(DEFAULT_DB))
    bad_media.add_argument("--socket", default=str(DEFAULT_PLAYBACK_SOCKET))
    bad_media.add_argument("--source-root", default=str(Path("/var/lib/lumelo/test-media-tagged")))
    bad_media.add_argument(
        "--root",
        default=str(Path("/var/lib/lumelo/test-media-bad")),
        help="Dedicated root for generated malformed-media fixtures.",
    )
    bad_media.add_argument("--timeout", type=float, default=5.0)
    bad_media.set_defaults(func=command_regress_bad_media)

    restart = subparsers.add_parser(
        "regress-playbackd-restart",
        help="Verify queue and current track survive a playbackd service restart.",
    )
    restart.add_argument("--db", default=str(DEFAULT_DB))
    restart.add_argument("--socket", default=str(DEFAULT_PLAYBACK_SOCKET))
    restart.add_argument(
        "--mount-root",
        help="Restrict restart regression track selection to one indexed mount path.",
    )
    restart.add_argument("--timeout", type=float, default=5.0)
    restart.set_defaults(func=command_regress_playbackd_restart)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    if not getattr(args, "command", None):
        parser.print_help(sys.stderr)
        return 64
    if args.command == "play" and not (args.track_uid or args.relative_path or args.first_wav):
        parser.error("play requires --track-uid, --relative-path, or --first-wav")
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
~~~

## `base/rootfs/overlay/usr/bin/lumelo-t4-report`

- bytes: 6411
- segment: 1/1

~~~text
#!/bin/sh
set -u

OUT=${1:-/tmp/lumelo-t4-report.txt}

detect_wifi_iface() {
  if [ -n "${LUMELO_WIFI_IFACE:-}" ]; then
    printf '%s\n' "${LUMELO_WIFI_IFACE}"
    return 0
  fi

  if [ -n "${WIFI_INTERFACE:-}" ]; then
    printf '%s\n' "${WIFI_INTERFACE}"
    return 0
  fi

  if command -v iw >/dev/null 2>&1; then
    iface=$(iw dev 2>/dev/null | awk '$1 == "Interface" && $2 !~ /^p2p-dev/ { print $2; exit }')
    if [ -n "${iface:-}" ]; then
      printf '%s\n' "${iface}"
      return 0
    fi
  fi

  for candidate in /sys/class/net/*; do
    iface=$(basename "${candidate}")
    case "${iface}" in
      p2p-dev*|lo)
        continue
        ;;
    esac
    if [ -d "${candidate}/wireless" ]; then
      printf '%s\n' "${iface}"
      return 0
    fi
  done

  return 1
}

section() {
  printf '\n## %s\n' "$1"
}

run() {
  printf '\n$'
  for arg in "$@"; do
    printf ' %s' "$arg"
  done
  printf '\n'

  "$@"
  status=$?
  if [ "${status}" -ne 0 ]; then
    printf '[exit %s]\n' "${status}"
  fi
}

cat_if_readable() {
  path=$1
  printf '\n# %s\n' "${path}"
  if [ -r "${path}" ]; then
    cat "${path}"
  else
    printf 'not readable\n'
  fi
}

command_if_present() {
  if command -v "$1" >/dev/null 2>&1; then
    run "$@"
  else
    printf '\n$ %s\nnot installed\n' "$1"
  fi
}

emit_dmesg_tail() {
  if command -v dmesg >/dev/null 2>&1 && command -v tail >/dev/null 2>&1; then
    printf '\n$ dmesg -T | tail -n 200\n'
    dmesg -T 2>&1 | tail -n 200
  else
    command_if_present dmesg -T
  fi
}

emit_keyword_dmesg_tail() {
  if ! command -v dmesg >/dev/null 2>&1; then
    return
  fi

  printf '\n$ dmesg -T | grep -Ei %s | tail -n 200\n' "$1"
  dmesg -T 2>&1 | grep -Ei "$1" | tail -n 200 || true
}

emit_report() {
  wifi_iface=$(detect_wifi_iface || true)

  section "Lumelo T4 Bring-Up Report"
  run date -u
  run uname -a
  cat_if_readable /etc/os-release
  cat_if_readable /etc/lumelo/image-build.txt
  cat_if_readable /proc/cmdline

  section "Storage"
  command_if_present lsblk -f
  run df -h
  run mount

  section "Network"
  run ip addr
  run ip route
  command_if_present networkctl status --no-pager
  command_if_present nmcli general status
  command_if_present nmcli device status
  command_if_present ss -lntup
  command_if_present rfkill list
  command_if_present iw dev
  cat_if_readable /etc/NetworkManager/NetworkManager.conf
  cat_if_readable /etc/NetworkManager/conf.d/12-managed-wifi.conf
  cat_if_readable /etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf
  cat_if_readable /etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf
  cat_if_readable /etc/network/interfaces
  command_if_present bluetoothctl show
  if [ -n "${wifi_iface}" ]; then
    run ip addr show dev "${wifi_iface}"
    command_if_present networkctl status "${wifi_iface}" --no-pager
  fi

  section "Bluetooth / Firmware"
  command_if_present systemctl cat bluetooth.service
  if [ -d /etc/systemd/system/bluetooth.service.d ]; then
    run ls -l /etc/systemd/system/bluetooth.service.d
  fi
  cat_if_readable /etc/systemd/system/bluetooth.service.d/10-lumelo-rfkill-unblock.conf
  cat_if_readable /etc/systemd/system/bluetooth.service.d/20-lumelo-uart-attach.conf
  cat_if_readable /etc/systemd/system/lumelo-bluetooth-uart-attach.service
  if [ -d /lib/firmware/brcm ]; then
    run ls -l /lib/firmware/brcm
  fi
  if [ -d /lib/firmware/rtl_bt ]; then
    run ls -l /lib/firmware/rtl_bt
  fi
  emit_keyword_dmesg_tail 'bluetooth|brcm|hci|firmware|rfkill'

  section "Lumelo Files"
  run ls -l /usr/bin/playbackd /usr/bin/sessiond /usr/bin/media-indexd /usr/bin/controld
  command_if_present ls -l \
    /usr/bin/lumelo-t4-report \
    /usr/bin/lumelo-audio-smoke \
    /usr/bin/hciattach.rk \
    /usr/bin/lumelo-bluetooth-provisioning-mode \
    /usr/bin/lumelo-wifi-apply \
    /usr/libexec/lumelo/bluetooth-uart-attach \
    /usr/libexec/lumelo/classic-bluetooth-wifi-provisiond \
    /usr/libexec/lumelo/bluetooth-wifi-provisiond
  run ls -l /etc/lumelo
  run ls -l /run/lumelo
  run ls -l /var/lib/lumelo
  run ls -l /var/cache/lumelo
  cat_if_readable /run/lumelo/provisioning-status.json

  section "Systemd"
  command_if_present systemctl --failed --no-pager
  if [ -d /etc/systemd/system/ssh.service.d ]; then
    run ls -l /etc/systemd/system/ssh.service.d
  fi
  cat_if_readable /etc/systemd/system/ssh.service.d/10-lumelo-hostkeys.conf
  for unit in \
    local-mode.target \
    playbackd.service \
    sessiond.service \
    media-indexd.service \
    controld.service \
    lumelo-bluetooth-uart-attach.service \
    bluetooth.service \
    lumelo-bluetooth-provisioning.service \
    lumelo-wifi-provisiond.service \
    systemd-networkd.service \
    systemd-resolved.service \
    ssh.service
  do
    command_if_present systemctl status "${unit}" --no-pager --lines=40
  done
  if [ -n "${wifi_iface}" ]; then
    command_if_present systemctl status "wpa_supplicant@${wifi_iface}.service" --no-pager --lines=40
  fi
  if [ "${wifi_iface}" != "wlan0" ]; then
    command_if_present systemctl status wpa_supplicant@wlan0.service --no-pager --lines=40
  fi

  section "Lumelo Journals"
  for unit in playbackd.service sessiond.service media-indexd.service controld.service lumelo-bluetooth-uart-attach.service bluetooth.service lumelo-bluetooth-provisioning.service lumelo-wifi-provisiond.service; do
    command_if_present journalctl -u "${unit}" -b --no-pager -n 100
  done
  if [ -n "${wifi_iface}" ]; then
    command_if_present journalctl -u "wpa_supplicant@${wifi_iface}.service" -b --no-pager -n 100
  fi
  if [ "${wifi_iface}" != "wlan0" ]; then
    command_if_present journalctl -u wpa_supplicant@wlan0.service -b --no-pager -n 100
  fi

  section "Audio"
  cat_if_readable /proc/asound/cards
  cat_if_readable /proc/asound/devices
  command_if_present aplay -l
  command_if_present amixer
  if [ -d /sys/class/sound ]; then
    run ls -l /sys/class/sound
  fi

  section "Kernel Tail"
  emit_dmesg_tail

  section "WebUI Loopback"
  command_if_present curl -sS --max-time 2 http://127.0.0.1:18080/healthz
  command_if_present curl -sS -I --max-time 2 http://127.0.0.1:18080/
  command_if_present curl -sS --max-time 2 http://127.0.0.1:18080/
}

if [ "${OUT}" = "-" ]; then
  emit_report
else
  out_dir=$(dirname "${OUT}")
  mkdir -p "${out_dir}" 2>/dev/null || true
  emit_report > "${OUT}" 2>&1
  printf 'Lumelo T4 report written to %s\n' "${OUT}"
fi
~~~

## `base/rootfs/overlay/usr/bin/lumelo-wifi-apply`

- bytes: 5995
- segment: 1/1

~~~text
#!/bin/sh
set -eu

usage() {
  echo "usage: lumelo-wifi-apply <ssid> <password>" >&2
  echo "   or: lumelo-wifi-apply --psk-hex <64-hex-psk> <ssid>" >&2
  exit 64
}

escape_wpa_string() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

is_networkmanager_active() {
  if ! command -v nmcli >/dev/null 2>&1; then
    return 1
  fi

  if command -v systemctl >/dev/null 2>&1; then
    systemctl is-active --quiet NetworkManager.service && return 0
    systemctl is-active --quiet NetworkManager && return 0
  fi

  nmcli general status >/dev/null 2>&1
}

find_nmcli_wifi_iface() {
  if ! command -v nmcli >/dev/null 2>&1; then
    return 1
  fi

  nmcli -t -f DEVICE,TYPE device status 2>/dev/null | awk -F: '
    $2 == "wifi" && $1 !~ /^p2p-dev/ {
      print $1
      exit
    }
  '
}

find_iw_wifi_iface() {
  if ! command -v iw >/dev/null 2>&1; then
    return 1
  fi

  iw dev 2>/dev/null | awk '
    $1 == "Interface" && $2 !~ /^p2p-dev/ {
      print $2
      exit
    }
  '
}

find_sysfs_wifi_iface() {
  for pattern in /sys/class/net/wlan* /sys/class/net/wl*; do
    [ -e "${pattern}" ] || continue
    if [ -d "${pattern}/wireless" ]; then
      basename "${pattern}"
      return 0
    fi
  done

  for candidate in /sys/class/net/*; do
    [ -e "${candidate}" ] || continue
    iface=$(basename "${candidate}")
    case "${iface}" in
      p2p-dev*|lo)
        continue
        ;;
    esac

    if [ -d "${candidate}/wireless" ]; then
      printf '%s\n' "${iface}"
      return 0
    fi
  done

  return 1
}

detect_wifi_iface() {
  if [ -n "${LUMELO_WIFI_IFACE:-}" ]; then
    printf '%s\n' "${LUMELO_WIFI_IFACE}"
    return 0
  fi

  if [ -n "${WIFI_INTERFACE:-}" ]; then
    printf '%s\n' "${WIFI_INTERFACE}"
    return 0
  fi

  iface=$(find_nmcli_wifi_iface || true)
  if [ -n "${iface:-}" ]; then
    printf '%s\n' "${iface}"
    return 0
  fi

  iface=$(find_iw_wifi_iface || true)
  if [ -n "${iface:-}" ]; then
    printf '%s\n' "${iface}"
    return 0
  fi

  find_sysfs_wifi_iface
}

selected_backend() {
  backend=${LUMELO_WIFI_BACKEND:-auto}
  case "${backend}" in
    auto)
      if is_networkmanager_active; then
        printf 'networkmanager\n'
      else
        printf 'wpa_supplicant\n'
      fi
      ;;
    networkmanager|wpa_supplicant)
      printf '%s\n' "${backend}"
      ;;
    *)
      echo "Unsupported Wi-Fi backend: ${backend}" >&2
      exit 64
      ;;
  esac
}

apply_with_networkmanager() {
  connection_name=${LUMELO_WIFI_CONNECTION_NAME:-lumelo-${WIFI_IFACE}}

  if [ -n "${PSK_HEX:-}" ]; then
    echo "NetworkManager backend does not support pre-derived WPA-PSK input" >&2
    exit 64
  fi

  if ! command -v nmcli >/dev/null 2>&1; then
    echo "nmcli is unavailable" >&2
    exit 69
  fi

  nmcli radio wifi on >/dev/null 2>&1 || true
  nmcli device set "${WIFI_IFACE}" managed yes >/dev/null 2>&1 || true
  nmcli connection delete id "${connection_name}" >/dev/null 2>&1 || true

  if ! nmcli --wait 45 device wifi connect "${SSID}" password "${PASSWORD}" \
    ifname "${WIFI_IFACE}" name "${connection_name}"; then
    echo "NetworkManager failed to connect interface ${WIFI_IFACE} to SSID: ${SSID}" >&2
    exit 70
  fi

  echo "Wi-Fi credentials applied with NetworkManager for SSID: ${SSID} on interface: ${WIFI_IFACE}"
}

apply_with_wpasupplicant() {
  conf=/etc/wpa_supplicant/wpa_supplicant-${WIFI_IFACE}.conf

  install -d -m 0755 /etc/wpa_supplicant
  tmp=$(mktemp "/etc/wpa_supplicant/.lumelo-${WIFI_IFACE}.XXXXXX")
  trap 'rm -f "${tmp}"' EXIT

  if [ -n "${PSK_HEX:-}" ]; then
    {
      printf 'ctrl_interface=DIR=/run/wpa_supplicant GROUP=netdev\n'
      printf 'update_config=1\n'
      printf 'country=%s\n\n' "${COUNTRY}"
      printf 'network={\n'
      printf '    ssid="%s"\n' "$(escape_wpa_string "${SSID}")"
      printf '    psk=%s\n' "${PSK_HEX}"
      printf '}\n'
    } > "${tmp}"
  else
    if ! command -v wpa_passphrase >/dev/null 2>&1; then
      echo "wpa_passphrase is unavailable" >&2
      exit 69
    fi

    {
      printf 'ctrl_interface=DIR=/run/wpa_supplicant GROUP=netdev\n'
      printf 'update_config=1\n'
      printf 'country=%s\n\n' "${COUNTRY}"
      wpa_passphrase "${SSID}" "${PASSWORD}" | sed '/^[[:space:]]*#psk=/d'
    } > "${tmp}"
  fi

  chmod 0600 "${tmp}"
  mv "${tmp}" "${conf}"
  trap - EXIT

  systemctl enable "wpa_supplicant@${WIFI_IFACE}.service" >/dev/null 2>&1 || true
  systemctl restart "wpa_supplicant@${WIFI_IFACE}.service" >/dev/null 2>&1 || true
  if command -v networkctl >/dev/null 2>&1; then
    networkctl reload >/dev/null 2>&1 || true
    networkctl reconfigure "${WIFI_IFACE}" >/dev/null 2>&1 || true
  fi

  if [ -n "${PSK_HEX:-}" ]; then
    echo "Wi-Fi PSK written for SSID: ${SSID} on interface: ${WIFI_IFACE}"
  else
    echo "Wi-Fi credentials written for SSID: ${SSID} on interface: ${WIFI_IFACE}"
  fi
}

PSK_HEX=
PASSWORD=

case "${1:-}" in
  --psk-hex)
    if [ "$#" -ne 3 ]; then
      usage
    fi
    PSK_HEX=$2
    SSID=$3
    ;;
  "")
    usage
    ;;
  *)
    if [ "$#" -ne 2 ]; then
      usage
    fi
    SSID=$1
    PASSWORD=$2
    ;;
esac

COUNTRY=${WIFI_COUNTRY:-00}
WIFI_IFACE=$(detect_wifi_iface) || {
  echo "No wireless interface found for Wi-Fi provisioning" >&2
  exit 69
}
BACKEND=$(selected_backend)

if [ -z "${SSID}" ]; then
  echo "SSID must not be empty" >&2
  exit 64
fi

if [ -n "${PSK_HEX}" ]; then
  case "${PSK_HEX}" in
    [0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]*)
      ;;
    *)
      echo "WPA-PSK hex must be 64 hexadecimal characters" >&2
      exit 64
      ;;
  esac
  if [ "${#PSK_HEX}" -ne 64 ]; then
    echo "WPA-PSK hex must be 64 hexadecimal characters" >&2
    exit 64
  fi
else
  if [ "${#PASSWORD}" -lt 8 ] || [ "${#PASSWORD}" -gt 63 ]; then
    echo "WPA-PSK password must be 8..63 characters" >&2
    exit 64
  fi
fi

case "${BACKEND}" in
  networkmanager)
    apply_with_networkmanager
    ;;
  wpa_supplicant)
    apply_with_wpasupplicant
    ;;
esac
~~~

## `base/rootfs/overlay/usr/lib/tmpfiles.d/lumelo.conf`

- bytes: 103
- segment: 1/1

~~~ini
d /run/lumelo 0755 root root -
d /var/lib/lumelo 0755 root root -
d /var/cache/lumelo 0755 root root -
~~~

## `base/rootfs/overlay/usr/libexec/lumelo/auth-recovery`

- bytes: 702
- segment: 1/1

~~~text
#!/bin/sh
set -eu

marker_name="${MARKER_NAME:-RESET_ADMIN_PASSWORD}"
scan_root="${RECOVERY_SCAN_ROOT:-/media}"
state_dir="${LUMELO_STATE_DIR:-${PRODUCT_STATE_DIR:-/var/lib/lumelo}}"
config_path="${LUMELO_CONFIG_PATH:-${PRODUCT_CONFIG_PATH:-/etc/lumelo/config.toml}}"

printf '%s\n' "auth-recovery placeholder: scan ${scan_root} for ${marker_name}" >&2
printf '%s\n' "auth-recovery placeholder: reset credentials in ${state_dir} and keep ${config_path} intact" >&2

# Real implementation will:
# 1. find a FAT32 TF/USB recovery medium
# 2. detect the exact marker file in the root directory
# 3. clear password/session state
# 4. disable SSH
# 5. hand control back to controld first-boot setup

exit 0
~~~

