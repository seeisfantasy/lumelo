#!/usr/bin/env python3
from __future__ import annotations

import subprocess
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Tuple


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "docs" / "review"
PART_LIMIT_BYTES = 58_000
SECTION_TARGET_BYTES = 38_000
GENERATED_PREFIX = "AI_Review_"
EXCLUDED_PREFIXES = ("docs/review/",)


LANGUAGE_BY_SUFFIX = {
    ".md": "md",
    ".py": "python",
    ".sh": "bash",
    ".go": "go",
    ".rs": "rust",
    ".java": "java",
    ".kts": "kotlin",
    ".toml": "toml",
    ".json": "json",
    ".xml": "xml",
    ".css": "css",
    ".html": "html",
    ".txt": "text",
    ".conf": "ini",
    ".service": "ini",
    ".rules": "udev",
    ".properties": "properties",
    ".sample": "text",
}


@dataclass
class FileEntry:
    relative_path: str
    size_bytes: int
    text: str | None
    reason: str | None = None


def run(argv: Sequence[str]) -> str:
    return subprocess.check_output(argv, cwd=ROOT, text=True)


def tracked_and_untracked_files() -> List[str]:
    output = run(["git", "ls-files", "--cached", "--others", "--exclude-standard"])
    results = []
    for raw in output.splitlines():
        path = raw.strip()
        if not path:
            continue
        if any(path.startswith(prefix) for prefix in EXCLUDED_PREFIXES):
            continue
        full_path = ROOT / path
        if not full_path.is_file():
            continue
        results.append(path)
    return sorted(results)


def classify_file(relative_path: str) -> FileEntry:
    full_path = ROOT / relative_path
    payload = full_path.read_bytes()
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError:
        return FileEntry(relative_path=relative_path, size_bytes=len(payload), text=None, reason="binary_or_non_utf8")
    if "\x00" in text:
        return FileEntry(relative_path=relative_path, size_bytes=len(payload), text=None, reason="binary_or_non_utf8")
    return FileEntry(relative_path=relative_path, size_bytes=len(payload), text=text)


def guess_language(relative_path: str) -> str:
    path = Path(relative_path)
    if path.name in {"gradlew"}:
        return "bash"
    return LANGUAGE_BY_SUFFIX.get(path.suffix.lower(), "text")


def utf8_len(text: str) -> int:
    return len(text.encode("utf-8"))


def split_long_line(line: str, max_bytes: int) -> List[str]:
    if utf8_len(line) <= max_bytes:
        return [line]
    pieces: List[str] = []
    current = ""
    for char in line:
        if current and utf8_len(current + char) > max_bytes:
            pieces.append(current)
            current = char
        else:
            current += char
    if current:
        pieces.append(current)
    return pieces


def split_text_segments(text: str, max_bytes: int) -> List[str]:
    lines = text.splitlines(keepends=True)
    chunks: List[str] = []
    current: List[str] = []
    current_bytes = 0

    for original_line in lines:
        for line in split_long_line(original_line, max_bytes):
            line_bytes = utf8_len(line)
            if current and current_bytes + line_bytes > max_bytes:
                chunks.append("".join(current))
                current = [line]
                current_bytes = line_bytes
            else:
                current.append(line)
                current_bytes += line_bytes

    if current:
        chunks.append("".join(current))

    if not chunks:
        chunks.append("")
    return chunks


def render_tree(paths: Iterable[str]) -> str:
    root: Dict[str, dict] = {}
    for path in paths:
        node = root
        parts = Path(path).parts
        for part in parts:
            node = node.setdefault(part, {})

    lines: List[str] = ["."]

    def walk(node: Dict[str, dict], prefix: str) -> None:
        names = sorted(node)
        for index, name in enumerate(names):
            child = node[name]
            last = index == len(names) - 1
            branch = "└── " if last else "├── "
            lines.append(prefix + branch + name)
            extension = "    " if last else "│   "
            walk(child, prefix + extension)

    walk(root, "")
    return "\n".join(lines)


def top_level_summary(entries: Sequence[FileEntry]) -> List[Tuple[str, int, int]]:
    buckets: Dict[str, List[FileEntry]] = defaultdict(list)
    for entry in entries:
        top = Path(entry.relative_path).parts[0] if len(Path(entry.relative_path).parts) > 1 else "(root)"
        buckets[top].append(entry)
    summary = []
    for top, group in sorted(buckets.items()):
        summary.append((top, len(group), sum(item.size_bytes for item in group)))
    return summary


def render_top_level_summary(entries: Sequence[FileEntry]) -> str:
    lines = ["| Top Level | Files | Bytes |", "| --- | ---: | ---: |"]
    for top, count, size in top_level_summary(entries):
        lines.append(f"| `{top}` | {count} | {size} |")
    return "\n".join(lines)


def new_part_header(number: int) -> str:
    return (
        f"# AI Review Part {number:02d}\n\n"
        "这是给外部 AI 做静态审查的代码分卷。"
        "每一卷都只包含仓库快照中的一部分文本文件内容，"
        "按当前工作树生成。\n\n"
    )


def render_file_section(entry: FileEntry, segment: str, segment_index: int, segment_count: int) -> str:
    language = guess_language(entry.relative_path)
    segment_suffix = "" if segment_count == 1 else f" ({segment_index}/{segment_count})"
    return (
        f"## `{entry.relative_path}`{segment_suffix}\n\n"
        f"- bytes: {entry.size_bytes}\n"
        f"- segment: {segment_index}/{segment_count}\n\n"
        f"~~~{language}\n"
        f"{segment}"
        f"~~~\n\n"
    )


def write_review_bundle(entries: Sequence[FileEntry]) -> Tuple[List[str], Dict[str, List[str]], List[FileEntry]]:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for existing in OUTPUT_DIR.glob(f"{GENERATED_PREFIX}*.md"):
        existing.unlink()
    for existing in OUTPUT_DIR.glob("._*"):
        existing.unlink()
    readme_path = OUTPUT_DIR / "README.md"
    if readme_path.exists():
        readme_path.unlink()

    included = [entry for entry in entries if entry.text is not None]
    omitted = [entry for entry in entries if entry.text is None]
    part_names: List[str] = []
    file_parts: Dict[str, List[str]] = defaultdict(list)

    part_number = 1
    current_name = f"{GENERATED_PREFIX}Part_{part_number:02d}.md"
    current_path = OUTPUT_DIR / current_name
    current_text = new_part_header(part_number)

    for entry in included:
        segments = split_text_segments(entry.text or "", SECTION_TARGET_BYTES)
        for index, segment in enumerate(segments, start=1):
            section = render_file_section(entry, segment, index, len(segments))
            if utf8_len(current_text) + utf8_len(section) > PART_LIMIT_BYTES and current_text != new_part_header(part_number):
                current_path.write_text(current_text, encoding="utf-8")
                part_names.append(current_name)
                part_number += 1
                current_name = f"{GENERATED_PREFIX}Part_{part_number:02d}.md"
                current_path = OUTPUT_DIR / current_name
                current_text = new_part_header(part_number)
            current_text += section
            if current_name not in file_parts[entry.relative_path]:
                file_parts[entry.relative_path].append(current_name)

    current_path.write_text(current_text, encoding="utf-8")
    part_names.append(current_name)

    for part_name in part_names:
        part_path = OUTPUT_DIR / part_name
        size = part_path.stat().st_size
        if size >= 60_000:
            raise SystemExit(f"generated review part is too large: {part_path} ({size} bytes)")

    return part_names, file_parts, omitted


def render_overview(entries: Sequence[FileEntry], part_names: Sequence[str], omitted: Sequence[FileEntry]) -> str:
    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    included_paths = [entry.relative_path for entry in entries if entry.text is not None]
    tree_snapshot = render_tree(included_paths)
    part_lines = ["| Review Doc | Bytes |", "| --- | ---: |"]
    for name in part_names:
        part_lines.append(f"| [{name}](/Volumes/SeeDisk/Codex/Lumelo/docs/review/{name}) | {(OUTPUT_DIR / name).stat().st_size} |")

    omitted_lines = []
    for entry in omitted:
        omitted_lines.append(f"- `{entry.relative_path}`: {entry.reason} ({entry.size_bytes} bytes)")
    if not omitted_lines:
        omitted_lines.append("- none")

    return (
        "# Lumelo AI Review Overview\n\n"
        f"- generated_at: {generated_at}\n"
        f"- repo_root: `/Volumes/SeeDisk/Codex/Lumelo`\n"
        f"- included_text_files: {len(included_paths)}\n"
        f"- omitted_binary_or_non_utf8_files: {len(omitted)}\n\n"
        "## 1. 这是什么系统\n\n"
        "Lumelo 当前是一个本地优先的网络音频系统，目标运行在 T4 板子上。"
        "它提供板端媒体索引、播放控制、Web UI、经典蓝牙配网，以及 Android 侧配网 App。"
        "当前主路径已经能完成：配网、进入 WebUI、索引真实曲库、展示封面、并在板子上真实输出音频。\n\n"
        "## 2. 底座是什么\n\n"
        "- 硬件底座：T4 板级平台，当前调试主要跑在 `sd` 系统上。\n"
        "- 系统底座：Linux rootfs 镜像，基于 `base/rootfs/overlay` 和 `packaging/image` 组装。\n"
        "- 服务编排：`systemd`。\n"
        "- 网络与配网：`NetworkManager`、`wpa_supplicant`、BlueZ 经典蓝牙、RFCOMM 配网 daemon。\n"
        "- 控制与页面：Go 写的 `controld` 提供 Web UI 和 HTTP API。\n"
        "- 播放：Rust 写的 `playbackd`，当前通过 `aplay` 接 ALSA 输出。\n"
        "- 索引：`media-indexd` 负责扫描介质、写入 `library.db`、生成封面缓存。\n\n"
        "## 3. 当前架构\n\n"
        "1. Android App 通过经典蓝牙和板子上的 `classic-bluetooth-wifi-provisiond` 建立配网会话。\n"
        "2. 配网成功后，手机和板子在同网段，通过 `controld` 提供的 Web UI 继续操作。\n"
        "3. `media-indexd` 扫描本地目录、外部介质或测试 fixture，把媒体元数据写入 `library.db`。\n"
        "4. `controld` 读取 `library.db`，渲染首页、曲库页、封面缩略图和配网页。\n"
        "5. `playbackd` 通过 Unix socket 接收播放命令，解析媒体，最后用 ALSA 输出。\n"
        "6. `lumelo-media-import` 和 `lumelo-media-smoke` 是当前板端验证与导入的关键 helper。\n\n"
        "## 4. 特色功能\n\n"
        "- 经典蓝牙配网已经支持加密凭据传输，并移除了明文回退。\n"
        "- 板端 `wpa_supplicant` 落盘已改为 `psk=<64hex>`，不再明文落盘 Wi-Fi 密码。\n"
        "- 真曲库索引和封面缩略图已经贯通到 `/library` 页面。\n"
        "- `wav + m4a/aac + flac + mp3 + ogg` 已在真机上完成播放验证。\n"
        "- 外部媒体已有导入入口，且在无真介质条件下补了一轮模拟块设备导入验证。\n"
        "- 已有正式板端回归命令覆盖：播放回归、批量扫描回归、坏文件边界、`playbackd` 重启恢复。\n\n"
        "## 5. 当前已知未闭环\n\n"
        "- 真 TF / USB 介质在场下的热插入 / 热拔出闭环。\n"
        "- 整机重启后的状态回归。\n"
        "- 坏文件是否要在索引层直接过滤，避免出现在用户曲库里。\n"
        "- 调试 `sd` 系统目前仍需人工按键选择启动，不是默认启动介质。\n\n"
        "## 6. 顶层目录结构\n\n"
        f"```text\n{tree_snapshot}\n```\n\n"
        "## 7. 顶层体量概览\n\n"
        f"{render_top_level_summary(entries)}\n\n"
        "## 8. Review 分卷\n\n"
        + "\n".join(part_lines)
        + "\n\n## 9. 省略文件\n\n"
        + "\n".join(omitted_lines)
        + "\n"
    )


def render_index(entries: Sequence[FileEntry], file_parts: Dict[str, List[str]], omitted: Sequence[FileEntry]) -> str:
    lines = [
        "# Lumelo AI Review File Index",
        "",
        "- 这份索引用于告诉外部 AI：每个仓库文件被放到了哪一卷 review 文档里。",
        "- `docs/review/` 目录本身不会再次被打包，避免递归。",
        "",
        "## Included Text Files",
        "",
    ]
    for entry in entries:
        if entry.text is None:
            continue
        refs = ", ".join(f"[{name}](/Volumes/SeeDisk/Codex/Lumelo/docs/review/{name})" for name in file_parts[entry.relative_path])
        lines.append(f"- `{entry.relative_path}` ({entry.size_bytes} bytes) -> {refs}")

    lines.extend(["", "## Omitted Binary Or Non-UTF8 Files", ""])
    if omitted:
        for entry in omitted:
            lines.append(f"- `{entry.relative_path}` ({entry.size_bytes} bytes) -> {entry.reason}")
    else:
        lines.append("- none")
    return "\n".join(lines) + "\n"


def main() -> int:
    paths = tracked_and_untracked_files()
    entries = [classify_file(path) for path in paths]
    part_names, file_parts, omitted = write_review_bundle(entries)

    overview = render_overview(entries, part_names, omitted)
    index = render_index(entries, file_parts, omitted)

    overview_path = OUTPUT_DIR / "README.md"
    index_path = OUTPUT_DIR / f"{GENERATED_PREFIX}File_Index.md"
    overview_path.write_text(overview, encoding="utf-8")
    index_path.write_text(index, encoding="utf-8")

    if overview_path.stat().st_size >= 60_000:
        raise SystemExit(f"overview is too large: {overview_path.stat().st_size} bytes")
    if index_path.stat().st_size >= 60_000:
        raise SystemExit(f"file index is too large: {index_path.stat().st_size} bytes")

    print(f"Generated {len(part_names)} review parts in {OUTPUT_DIR}")
    print(f"Overview: {overview_path.relative_to(ROOT)}")
    print(f"Index:    {index_path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
