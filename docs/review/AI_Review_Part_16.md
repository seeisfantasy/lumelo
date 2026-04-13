# AI Review Part 16

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `packaging/image/README.md`

- bytes: 3177
- segment: 1/1

~~~md
# Image Packaging

Current first target:

- `T4 smoke image`
- boot from `TF` on `FriendlyELEC NanoPC-T4`
- validate `network + SSH + Lumelo services + placeholder WebUI + library scan`
- do **not** block on real audio output before first flash

Locked smoke base:

- official `FriendlyELEC NanoPC-T4` SD image family:
  - `rk3399-sd-debian-trixie-core-4.19-arm64-YYYYMMDD.img.gz`
- lock metadata:
  - [t4-smoke-base.toml](/Volumes/SeeDisk/Codex/Lumelo/packaging/image/t4-smoke-base.toml)

Build entrypoint:

- [build-t4-smoke-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/build-t4-smoke-image.sh)

Current smoke builder strategy:

- reuse the official FriendlyELEC boot chain and partition layout
- remaster only the `rootfs` partition of the selected SD image
- inject `Lumelo` binaries plus `base/rootfs/overlay`
- enable `local-mode.target`
- enable wired DHCP via `systemd-networkd` when available

This is intentionally a bring-up shortcut for the first TF image.
The long-term V1 direction remains:

- `FriendlyELEC` board support for `kernel / dtb / u-boot`
- `Lumelo`-owned rootfs and upper-layer services

First `Lumelo-defined rootfs` builder:

- lock metadata:
  - [t4-lumelo-rootfs-base.toml](/Volumes/SeeDisk/Codex/Lumelo/packaging/image/t4-lumelo-rootfs-base.toml)
- package manifest:
  - [t4-bringup-packages.txt](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/manifests/t4-bringup-packages.txt)
- post-build hook:
  - [t4-bringup-postbuild.sh](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/hooks/t4-bringup-postbuild.sh)
- bring-up report tool:
  - [lumelo-t4-report](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-t4-report)
- manual ALSA smoke helper:
  - [lumelo-audio-smoke](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-audio-smoke)
- build entrypoint:
  - [build-t4-lumelo-rootfs-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/build-t4-lumelo-rootfs-image.sh)
- SSH bring-up wrapper:
  - [build-t4-ssh-bringup-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/build-t4-ssh-bringup-image.sh)
- offline verifier:
  - [verify-t4-lumelo-rootfs-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/verify-t4-lumelo-rootfs-image.sh)

Current custom-rootfs builder strategy:

- recreate the GPT layout instead of remastering the official rootfs
- copy the FriendlyELEC pre-partition RK3399 loader area before `p1`
- copy FriendlyELEC `p1-p7` board-support partitions into a new image
- create a smaller `p8 rootfs` and `p9 userdata`
- bootstrap `Debian trixie` with `mmdebstrap`
- inject matching FriendlyELEC kernel modules and firmware from the official image
- inject `Lumelo` binaries plus `base/rootfs/overlay`
- enable SSH by default for development / bring-up images
- allow direct `root/root` SSH login during the current debug phase
- keep `SSH_AUTHORIZED_KEYS_FILE=/path/to/id.pub` as an optional key-injection path
- require release-like images to set `ENABLE_SSH=0` explicitly
- provide manual ALSA diagnostics only; do not add an automatic audio smoke
  service
- validate each generated image with the read-only `p8` verifier before T4
  hardware bring-up
- write a sibling `.sha256` file after each successful image build
~~~

## `packaging/image/t4-lumelo-rootfs-base.toml`

- bytes: 1121
- segment: 1/1

~~~toml
[base]
profile = "t4-bringup"
strategy = "lumelo_defined_rootfs_with_friendly_bootchain"
board_vendor = "FriendlyELEC"
board_model = "NanoPC-T4"
soc = "RK3399"
board_source_image_family = "rk3399-sd-debian-trixie-core-4.19-arm64-YYYYMMDD.img.gz"
board_source_role = "borrow p1-p7 boot-chain partitions plus matching kernel modules/firmware"
rootfs_suite = "trixie"
rootfs_variant = "minbase"
rootfs_size_mib = 1024
userdata_size_mib = 128

[rootfs]
packages_manifest = "base/rootfs/manifests/t4-bringup-packages.txt"
postbuild_hook = "base/rootfs/hooks/t4-bringup-postbuild.sh"
overlay_root = "base/rootfs/overlay"

[notes]
why_this_profile = "First Lumelo-defined rootfs image for T4 hardware bring-up without carrying the full official userdata partition."
board_support_bridge = "Until FriendlyELEC board-support artifacts are checked into base/board-support/friendly, the builder copies boot-chain partitions and matching runtime kernel modules from the selected official image."
expected_outcome = "A smaller TF image that boots with FriendlyELEC board support while all userspace above the kernel is Lumelo-owned."
~~~

## `packaging/image/t4-smoke-base.toml`

- bytes: 1755
- segment: 1/1

~~~toml
[base]
profile = "t4-smoke"
strategy = "remaster_official_sd_image"
board_vendor = "FriendlyELEC"
board_model = "NanoPC-T4"
soc = "RK3399"
official_wiki = "https://wiki.friendlyelec.com/wiki/index.php/NanoPC-T4"
official_download_page = "https://download.friendlyelec.com/NanoPC-T4"
official_image_family = "rk3399-sd-debian-trixie-core-4.19-arm64-YYYYMMDD.img.gz"
official_image_note = "Debian 13 Core, no desktop environment, command line only"
locked_update_log_date = "2026-01-12"
kernel_line = "4.19.y"
uboot_line = "v2017.09"
partition_layout = "GPT"
rootfs_partition_number = 8
boot_partition_number = 7

[board_support]
packaging_tool = "sd-fuse_rk3399"
packaging_tool_branch = "kernel-4.19"
kernel_repo = "https://github.com/friendlyarm/kernel-rockchip"
kernel_branch = "nanopi4-v4.19.y"
kernel_defconfig = "nanopi4_linux_defconfig"
uboot_repo = "https://github.com/friendlyarm/uboot-rockchip"
uboot_branch = "nanopi4-v2017.09"
uboot_defconfig = "nanopi4_defconfig"

[smoke_scope]
goal = "first bootable TF smoke image for T4 bring-up"
includes = [
  "Ethernet DHCP",
  "SSH when base image already provides ssh.service",
  "Lumelo systemd overlay",
  "playbackd/sessiond/media-indexd/controld binaries",
  "library/list placeholder validation",
]
excludes = [
  "real ALSA output",
  "USB DAC or I2S validation",
  "eMMC flashing",
  "recovery/update pipeline",
]

[notes]
why_this_base = "Fastest path to a bootable T4 validation image while preserving the long-term direction of FriendlyELEC board support plus Lumelo-owned upper layers."
long_term_direction = "The smoke image uses an official FriendlyELEC SD image as the shortest bring-up shortcut. The long-term V1 direction remains FriendlyELEC board support plus Lumelo-defined rootfs."
~~~

## `packaging/recovery/README.md`

- bytes: 82
- segment: 1/1

~~~md
# Recovery Packaging Placeholder

Place recovery-media assets and workflows here.
~~~

## `packaging/systemd/README.md`

- bytes: 240
- segment: 1/1

~~~md
# Systemd Packaging Notes

The first-pass service and target units live in:

- `base/rootfs/overlay/etc/systemd/system/`

Keep packaging-specific enablement logic here so the source-of-truth unit files
do not drift from the rootfs overlay.
~~~

## `packaging/update/README.md`

- bytes: 76
- segment: 1/1

~~~md
# Update Packaging Placeholder

Place offline update packaging assets here.
~~~

## `scripts/README.md`

- bytes: 650
- segment: 1/1

~~~md
# Project Scripts

Keep project-level helper scripts here.

Per the current docs, this repository should keep a single top-level
`scripts/` directory instead of introducing multiple script trees.

Current development helpers:

- `dev-playbackd.sh`
- `dev-sessiond.sh`
- `dev-controld.sh`
- `dev-media-indexd.sh`
- `dev-up.sh`
- `mount-lumelodev-apfs.sh`
- `sync-to-lumelodev-apfs.sh`
- `orbstack-bootstrap-lumelo-dev.sh`
- `build-t4-smoke-image.sh`
- `build-t4-lumelo-rootfs-image.sh`
- `build-t4-ssh-bringup-image.sh`
- `verify-t4-lumelo-rootfs-image.sh`
- `compare-t4-wireless-golden.sh`
- `deploy-t4-runtime-update.sh`
- `build-ai-review-docs.py`
~~~

## `scripts/build-ai-review-docs.py`

- bytes: 14206
- segment: 1/1

~~~python
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
~~~

## `scripts/build-t4-lumelo-rootfs-image.sh`

- bytes: 17417
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

PATH="/usr/local/go/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH}"
if [ -n "${SUDO_USER:-}" ] && [ -x "/home/${SUDO_USER}/.cargo/bin/cargo" ]; then
  PATH="/home/${SUDO_USER}/.cargo/bin:${PATH}"
fi
if [ -n "${HOME:-}" ] && [ -x "${HOME}/.cargo/bin/cargo" ]; then
  PATH="${HOME}/.cargo/bin:${PATH}"
fi
if [ -n "${SUDO_USER:-}" ] && [ -z "${CARGO_HOME:-}" ] && [ -d "/home/${SUDO_USER}/.cargo" ]; then
  CARGO_HOME="/home/${SUDO_USER}/.cargo"
fi
if [ -n "${SUDO_USER:-}" ] && [ -z "${RUSTUP_HOME:-}" ] && [ -d "/home/${SUDO_USER}/.rustup" ]; then
  RUSTUP_HOME="/home/${SUDO_USER}/.rustup"
fi
export PATH
export CARGO_HOME
export RUSTUP_HOME

: "${CARGO_TARGET_DIR:=/tmp/lumelo-t4-rootfs-cargo-target}"
: "${GOCACHE:=/tmp/lumelo-t4-rootfs-go-cache}"
: "${ROOTFS_SUITE:=trixie}"
: "${ROOTFS_COMPONENTS:=main,non-free-firmware}"
: "${ROOTFS_VARIANT:=minbase}"
: "${ROOTFS_SIZE_MIB:=1024}"
: "${DATA_SIZE_MIB:=128}"
: "${ENABLE_SSH:=1}"
: "${SSH_AUTHORIZED_KEYS_FILE:=}"
: "${ROOT_PASSWORD:=root}"
: "${LUMELO_IMAGE_PROFILE:=t4-bringup}"
export CARGO_TARGET_DIR
export GOCACHE

usage() {
  cat <<'EOF'
Usage:
  build-t4-lumelo-rootfs-image.sh \
    --board-base-image /path/to/rk3399-sd-*.img[.gz] \
    --output /path/to/lumelo-t4-rootfs.img

Environment:
  ROOTFS_SUITE         Debian suite for the Lumelo-owned rootfs (default: trixie)
  ROOTFS_COMPONENTS    APT components for the Lumelo-owned rootfs
                       (default: main,non-free-firmware)
  ROOTFS_VARIANT       mmdebstrap variant (default: minbase)
  ROOTFS_SIZE_MIB      Size of partition p8 in MiB (default: 1024)
  DATA_SIZE_MIB        Size of partition p9 in MiB (default: 128)
  ENABLE_SSH           Set to 1 to enable ssh.service in the image.
                       Development / bring-up images default to 1.
  SSH_AUTHORIZED_KEYS_FILE
                       Optional public key file copied to
                       /root/.ssh/authorized_keys when ENABLE_SSH=1
  ROOT_PASSWORD        Root console password for bring-up images.
                       Defaults to "root" during the debug phase.
  LUMELO_BUILD_ROOT    Build workspace directory; defaults next to the output
  KEEP_WORKDIR         Keep temporary workspace when set to 1

Notes:
  - Runs on Linux arm64 with root privileges.
  - Builds a Lumelo-defined rootfs from scratch.
  - Borrows FriendlyELEC boot-chain partitions and matching kernel modules
    from the selected official NanoPC-T4 image.
  - Development / bring-up images default to SSH enabled.
  - Set ENABLE_SSH=0 explicitly for release-like images.
EOF
}

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "build-t4-lumelo-rootfs-image.sh must run as root" >&2
    exit 1
  fi
}

require_linux() {
  if [ "$(uname -s)" != "Linux" ]; then
    echo "build-t4-lumelo-rootfs-image.sh must run on Linux" >&2
    exit 1
  fi
}

require_arm64() {
  case "$(uname -m)" in
    aarch64|arm64)
      ;;
    *)
      echo "build-t4-lumelo-rootfs-image.sh currently expects a Linux arm64 build host" >&2
      exit 1
      ;;
  esac
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

stage_executable() {
  source_path=$1
  destination_path=$2

  rm -f "${destination_path}"
  cp "${source_path}" "${destination_path}"
  chmod 0755 "${destination_path}"
}

normalize_overlay_permissions() {
  source_root=$1
  destination_root=$2

  find "${source_root}" -mindepth 1 \
    ! -name '._*' \
    ! -name '.DS_Store' \
    -print | while IFS= read -r source_path; do
      relative_path=${source_path#"${source_root}/"}
      destination_path=${destination_root}/${relative_path}

      if [ -d "${source_path}" ] && [ -d "${destination_path}" ]; then
        chmod 0755 "${destination_path}"
        continue
      fi

      if [ -f "${source_path}" ] && [ -f "${destination_path}" ]; then
        chmod 0644 "${destination_path}"
      fi
    done
}

apply_overlay_mode_overrides() {
  destination_root=$1

  if [ -d "${destination_root}/etc/bluetooth" ]; then
    chmod 0555 "${destination_root}/etc/bluetooth"
  fi
  if [ -f "${destination_root}/etc/bluetooth/main.conf" ]; then
    chmod 0644 "${destination_root}/etc/bluetooth/main.conf"
  fi
}

wait_for_partition() {
  partition=$1
  count=0
  while [ ! -b "${partition}" ] && [ "${count}" -lt 10 ]; do
    sleep 1
    count=$((count + 1))
  done
  [ -b "${partition}" ]
}

partition_guid_code() {
  partition_number=$1
  sgdisk -i "${partition_number}" "${BASE_RAW_IMAGE}" |
    sed -n 's/^Partition GUID code: \([0-9A-Fa-f-]*\).*/\1/p' |
    head -n 1
}

partition_start_sector() {
  partition_number=$1
  sgdisk -i "${partition_number}" "${BASE_RAW_IMAGE}" |
    sed -n 's/^First sector: \([0-9]*\).*/\1/p' |
    head -n 1
}

partition_end_sector() {
  partition_number=$1
  sgdisk -i "${partition_number}" "${BASE_RAW_IMAGE}" |
    sed -n 's/^Last sector: \([0-9]*\).*/\1/p' |
    head -n 1
}

partition_name() {
  partition_number=$1
  sgdisk -i "${partition_number}" "${BASE_RAW_IMAGE}" |
    sed -n "s/^Partition name: '\(.*\)'/\1/p" |
    head -n 1
}

cleanup() {
  status=$?
  if [ -n "${ROOTFS_MOUNT:-}" ] && mountpoint -q "${ROOTFS_MOUNT}" 2>/dev/null; then
    umount "${ROOTFS_MOUNT}" || true
  fi
  if [ -n "${BASE_ROOTFS_MOUNT:-}" ] && mountpoint -q "${BASE_ROOTFS_MOUNT}" 2>/dev/null; then
    umount "${BASE_ROOTFS_MOUNT}" || true
  fi
  if [ -n "${OUTPUT_LOOPDEV:-}" ]; then
    losetup -d "${OUTPUT_LOOPDEV}" || true
  fi
  if [ -n "${BASE_LOOPDEV:-}" ]; then
    losetup -d "${BASE_LOOPDEV}" || true
  fi
  if [ "${KEEP_WORKDIR:-0}" != "1" ] && [ -n "${WORKDIR:-}" ] && [ -d "${WORKDIR}" ]; then
    rm -rf "${WORKDIR}"
  fi
  exit "${status}"
}

BOARD_BASE_IMAGE=
OUTPUT_IMAGE=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --board-base-image)
      BOARD_BASE_IMAGE=$2
      shift 2
      ;;
    --output)
      OUTPUT_IMAGE=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ -z "${BOARD_BASE_IMAGE}" ] || [ -z "${OUTPUT_IMAGE}" ]; then
  usage >&2
  exit 1
fi

case "${ENABLE_SSH}" in
  0|1)
    ;;
  *)
    echo "ENABLE_SSH must be 0 or 1" >&2
    exit 1
    ;;
esac

require_linux
require_arm64
require_root
require_cmd cargo
require_cmd go
require_cmd gzip
require_cmd losetup
require_cmd mount
require_cmd mountpoint
require_cmd mmdebstrap
require_cmd mkfs.ext4
require_cmd partx
require_cmd rsync
require_cmd sha256sum
require_cmd sgdisk
require_cmd truncate
require_cmd tune2fs

BOARD_BASE_IMAGE_ABS="$(cd "$(dirname "${BOARD_BASE_IMAGE}")" && pwd)/$(basename "${BOARD_BASE_IMAGE}")"
mkdir -p "$(dirname "${OUTPUT_IMAGE}")"
OUTPUT_IMAGE_ABS="$(cd "$(dirname "${OUTPUT_IMAGE}")" && pwd)/$(basename "${OUTPUT_IMAGE}")"
SSH_AUTHORIZED_KEYS_FILE_ABS=

if [ ! -f "${BOARD_BASE_IMAGE_ABS}" ]; then
  echo "board base image not found: ${BOARD_BASE_IMAGE_ABS}" >&2
  exit 1
fi

if [ -n "${SSH_AUTHORIZED_KEYS_FILE}" ]; then
  SSH_AUTHORIZED_KEYS_FILE_ABS="$(cd "$(dirname "${SSH_AUTHORIZED_KEYS_FILE}")" && pwd)/$(basename "${SSH_AUTHORIZED_KEYS_FILE}")"
  if [ ! -f "${SSH_AUTHORIZED_KEYS_FILE_ABS}" ]; then
    echo "SSH_AUTHORIZED_KEYS_FILE not found: ${SSH_AUTHORIZED_KEYS_FILE_ABS}" >&2
    exit 1
  fi
fi

manifest_path="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)/base/rootfs/manifests/t4-bringup-packages.txt"
hook_path="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)/base/rootfs/hooks/t4-bringup-postbuild.sh"
overlay_root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)/base/rootfs/overlay"
repo_root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"

workdir_default="$(dirname "${OUTPUT_IMAGE_ABS}")/.lumelo-t4-rootfs-build"
WORKDIR="${LUMELO_BUILD_ROOT:-${workdir_default}}"
WORKDIR="${WORKDIR%/}"
BASE_RAW_IMAGE="${WORKDIR}/board-base.img"
BASE_ROOTFS_MOUNT="${WORKDIR}/mnt-base-rootfs"
ROOTFS_MOUNT="${WORKDIR}/mnt-rootfs"
STAGE_DIR="${WORKDIR}/stage"

rm -rf "${WORKDIR}"
mkdir -p "${BASE_ROOTFS_MOUNT}" "${ROOTFS_MOUNT}" "${STAGE_DIR}/bin" "${CARGO_TARGET_DIR}" "${GOCACHE}"

trap cleanup EXIT INT TERM

echo "==> building Linux arm64 binaries"
cargo build \
  --manifest-path "${repo_root}/services/rust/Cargo.toml" \
  --release \
  -p playbackd \
  -p sessiond \
  -p media-indexd
(cd "${repo_root}/services/controld" && GOOS=linux GOARCH=arm64 go build -o "${STAGE_DIR}/bin/controld" ./cmd/controld)

stage_executable "${CARGO_TARGET_DIR}/release/playbackd" "${STAGE_DIR}/bin/playbackd"
stage_executable "${CARGO_TARGET_DIR}/release/sessiond" "${STAGE_DIR}/bin/sessiond"
stage_executable "${CARGO_TARGET_DIR}/release/media-indexd" "${STAGE_DIR}/bin/media-indexd"

echo "==> preparing board-support source image"
case "${BOARD_BASE_IMAGE_ABS}" in
  *.gz)
    gzip -dc "${BOARD_BASE_IMAGE_ABS}" > "${BASE_RAW_IMAGE}"
    ;;
  *.img)
    cp "${BOARD_BASE_IMAGE_ABS}" "${BASE_RAW_IMAGE}"
    ;;
  *)
    echo "unsupported board base image format: ${BOARD_BASE_IMAGE_ABS}" >&2
    exit 1
    ;;
esac

BASE_LOOPDEV="$(losetup --find --partscan --show "${BASE_RAW_IMAGE}")"
partx -a "${BASE_LOOPDEV}" >/dev/null 2>&1 || true
wait_for_partition "${BASE_LOOPDEV}p8" || {
  echo "expected base rootfs partition not found: ${BASE_LOOPDEV}p8" >&2
  exit 1
}

base_root_part_name="$(basename "${BASE_LOOPDEV}p8")"
rootfs_start_sector="$(cat "/sys/class/block/${base_root_part_name}/start")"
first_board_partition_start="$(partition_start_sector 1)"
rootfs_type_code="$(partition_guid_code 8)"
userdata_type_code="$(partition_guid_code 9)"
mount -o ro "${BASE_LOOPDEV}p8" "${BASE_ROOTFS_MOUNT}"

rootfs_sectors=$((ROOTFS_SIZE_MIB * 2048))
data_sectors=$((DATA_SIZE_MIB * 2048))
rootfs_end_sector=$((rootfs_start_sector + rootfs_sectors - 1))
data_start_sector=$((rootfs_end_sector + 1))
data_end_sector=$((data_start_sector + data_sectors - 1))
total_sectors=$((data_end_sector + 2048))
total_bytes=$((total_sectors * 512))

echo "==> assembling board image skeleton"
mkdir -p "$(dirname "${OUTPUT_IMAGE_ABS}")"
rm -f "${OUTPUT_IMAGE_ABS}"
truncate -s "${total_bytes}" "${OUTPUT_IMAGE_ABS}"

sgdisk -o "${OUTPUT_IMAGE_ABS}" >/dev/null
for partition in 1 2 3 4 5 6 7; do
  start_sector="$(partition_start_sector "${partition}")"
  end_sector="$(partition_end_sector "${partition}")"
  guid_code="$(partition_guid_code "${partition}")"
  name="$(partition_name "${partition}")"
  sgdisk -n "${partition}:${start_sector}:${end_sector}" \
    -t "${partition}:${guid_code}" \
    -c "${partition}:${name}" \
    "${OUTPUT_IMAGE_ABS}" >/dev/null
done
sgdisk -n "8:${rootfs_start_sector}:${rootfs_end_sector}" \
  -t "8:${rootfs_type_code}" \
  -c "8:rootfs" \
  "${OUTPUT_IMAGE_ABS}" >/dev/null
sgdisk -n "9:${data_start_sector}:${data_end_sector}" \
  -t "9:${userdata_type_code}" \
  -c "9:userdata" \
  "${OUTPUT_IMAGE_ABS}" >/dev/null
sgdisk -v "${OUTPUT_IMAGE_ABS}" >/dev/null

echo "==> copying FriendlyELEC pre-partition bootloader area"
dd if="${BASE_RAW_IMAGE}" \
  of="${OUTPUT_IMAGE_ABS}" \
  bs=512 \
  skip=34 \
  seek=34 \
  count=$((first_board_partition_start - 34)) \
  conv=notrunc,fsync \
  status=none

OUTPUT_LOOPDEV="$(losetup --find --partscan --show "${OUTPUT_IMAGE_ABS}")"
partx -a "${OUTPUT_LOOPDEV}" >/dev/null 2>&1 || true
wait_for_partition "${OUTPUT_LOOPDEV}p8" || {
  echo "expected output rootfs partition not found: ${OUTPUT_LOOPDEV}p8" >&2
  exit 1
}
wait_for_partition "${OUTPUT_LOOPDEV}p9" || {
  echo "expected output userdata partition not found: ${OUTPUT_LOOPDEV}p9" >&2
  exit 1
}

for partition in 1 2 3 4 5 6 7; do
  dd if="${BASE_LOOPDEV}p${partition}" of="${OUTPUT_LOOPDEV}p${partition}" bs=4M conv=fsync status=none
done

echo "==> formatting Lumelo rootfs and userdata"
mkfs.ext4 -F -L rootfs "${OUTPUT_LOOPDEV}p8" >/dev/null
tune2fs -m 0 "${OUTPUT_LOOPDEV}p8" >/dev/null
mkfs.ext4 -F -L userdata "${OUTPUT_LOOPDEV}p9" >/dev/null
tune2fs -m 0 "${OUTPUT_LOOPDEV}p9" >/dev/null

mount "${OUTPUT_LOOPDEV}p8" "${ROOTFS_MOUNT}"

packages_csv="$(awk '!/^[[:space:]]*($|#)/ {print $1}' "${manifest_path}" | paste -sd, -)"
if [ "${ENABLE_SSH}" = "1" ]; then
  case ",${packages_csv}," in
    *,openssh-server,*)
      ;;
    *)
      packages_csv="${packages_csv},openssh-server"
      ;;
  esac
fi

echo "==> bootstrapping Lumelo-defined rootfs"
mmdebstrap \
  --architectures=arm64 \
  --variant="${ROOTFS_VARIANT}" \
  --components="${ROOTFS_COMPONENTS}" \
  --include="${packages_csv}" \
  --dpkgopt='path-exclude=/usr/share/doc/*' \
  --dpkgopt='path-include=/usr/share/doc/*/copyright' \
  --dpkgopt='path-exclude=/usr/share/man/*' \
  --dpkgopt='path-exclude=/usr/share/info/*' \
  --dpkgopt='path-exclude=/usr/share/locale/*' \
  --dpkgopt='path-include=/usr/share/locale/locale.alias' \
  "${ROOTFS_SUITE}" \
  "${ROOTFS_MOUNT}" \
  http://deb.debian.org/debian

echo "==> injecting FriendlyELEC runtime kernel support"
mkdir -p \
  "${ROOTFS_MOUNT}/etc" \
  "${ROOTFS_MOUNT}/etc/modprobe.d" \
  "${ROOTFS_MOUNT}/lib/modules" \
  "${ROOTFS_MOUNT}/lib/firmware" \
  "${ROOTFS_MOUNT}/system/etc" \
  "${ROOTFS_MOUNT}/usr/bin"
rsync -a \
  --exclude='._*' \
  --exclude='.DS_Store' \
  "${BASE_ROOTFS_MOUNT}/lib/modules/" \
  "${ROOTFS_MOUNT}/lib/modules/"
if [ -d "${BASE_ROOTFS_MOUNT}/lib/firmware" ]; then
  rsync -a \
    --exclude='._*' \
    --exclude='.DS_Store' \
    "${BASE_ROOTFS_MOUNT}/lib/firmware/" \
    "${ROOTFS_MOUNT}/lib/firmware/"
fi
if [ -d "${BASE_ROOTFS_MOUNT}/etc/firmware" ]; then
  mkdir -p "${ROOTFS_MOUNT}/etc/firmware"
  rsync -a \
    --exclude='._*' \
    --exclude='.DS_Store' \
    "${BASE_ROOTFS_MOUNT}/etc/firmware/" \
    "${ROOTFS_MOUNT}/etc/firmware/"
else
  echo "expected FriendlyELEC bluetooth patch directory missing: /etc/firmware" >&2
  exit 1
fi
if [ -f "${BASE_ROOTFS_MOUNT}/etc/modprobe.d/bcmdhd.conf" ]; then
  install -D -m 0644 "${BASE_ROOTFS_MOUNT}/etc/modprobe.d/bcmdhd.conf" \
    "${ROOTFS_MOUNT}/etc/modprobe.d/bcmdhd.conf"
else
  echo "expected FriendlyELEC wireless driver policy missing: /etc/modprobe.d/bcmdhd.conf" >&2
  exit 1
fi
if [ ! -f "${ROOTFS_MOUNT}/etc/firmware/BCM4356A2.hcd" ]; then
  echo "expected FriendlyELEC bluetooth patch firmware missing after copy: /etc/firmware/BCM4356A2.hcd" >&2
  exit 1
fi
if [ -d "${BASE_ROOTFS_MOUNT}/system/etc/firmware" ]; then
  mkdir -p "${ROOTFS_MOUNT}/system/etc/firmware"
  rsync -a \
    --exclude='._*' \
    --exclude='.DS_Store' \
    "${BASE_ROOTFS_MOUNT}/system/etc/firmware/" \
    "${ROOTFS_MOUNT}/system/etc/firmware/"
else
  echo "expected FriendlyELEC vendor wireless firmware directory missing: /system/etc/firmware" >&2
  exit 1
fi
for vendor_firmware in \
  fw_bcm4356a2_ag.bin \
  nvram_ap6356.txt; do
  if [ ! -f "${ROOTFS_MOUNT}/system/etc/firmware/${vendor_firmware}" ]; then
    echo "expected FriendlyELEC vendor wireless firmware missing after copy: /system/etc/firmware/${vendor_firmware}" >&2
    exit 1
  fi
done
if [ -x "${BASE_ROOTFS_MOUNT}/usr/bin/hciattach.rk" ]; then
  install -m 0755 "${BASE_ROOTFS_MOUNT}/usr/bin/hciattach.rk" \
    "${ROOTFS_MOUNT}/usr/bin/hciattach.rk"
else
  echo "expected FriendlyELEC bluetooth UART attach helper missing: /usr/bin/hciattach.rk" >&2
  exit 1
fi

echo "==> injecting Lumelo binaries and overlay"
install -m 0755 "${STAGE_DIR}/bin/playbackd" "${ROOTFS_MOUNT}/usr/bin/playbackd"
install -m 0755 "${STAGE_DIR}/bin/sessiond" "${ROOTFS_MOUNT}/usr/bin/sessiond"
install -m 0755 "${STAGE_DIR}/bin/media-indexd" "${ROOTFS_MOUNT}/usr/bin/media-indexd"
install -m 0755 "${STAGE_DIR}/bin/controld" "${ROOTFS_MOUNT}/usr/bin/controld"

rsync -a \
  --exclude='._*' \
  --exclude='.DS_Store' \
  "${overlay_root}/" \
  "${ROOTFS_MOUNT}/"

# The overlay tree may live on a host filesystem that does not preserve Linux
# modes well. Normalize only the copied overlay paths so non-root system users
# can still traverse /usr, /usr/lib, /etc, and execute the expected helpers.
normalize_overlay_permissions "${overlay_root}" "${ROOTFS_MOUNT}"
apply_overlay_mode_overrides "${ROOTFS_MOUNT}"

for overlay_bin in "${ROOTFS_MOUNT}"/usr/bin/lumelo-*; do
  if [ -f "${overlay_bin}" ]; then
    chmod 0755 "${overlay_bin}"
  fi
done
for overlay_libexec in "${ROOTFS_MOUNT}"/usr/libexec/lumelo/* "${ROOTFS_MOUNT}"/usr/libexec/product/*; do
  if [ -f "${overlay_libexec}" ]; then
    chmod 0755 "${overlay_libexec}"
  fi
done

if [ ! -x "${ROOTFS_MOUNT}/usr/bin/lumelo-t4-report" ] ||
  [ ! -x "${ROOTFS_MOUNT}/usr/bin/lumelo-audio-smoke" ] ||
  [ ! -x "${ROOTFS_MOUNT}/usr/bin/lumelo-bluetooth-provisioning-mode" ] ||
  [ ! -x "${ROOTFS_MOUNT}/usr/bin/lumelo-wifi-apply" ] ||
  [ ! -x "${ROOTFS_MOUNT}/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond" ]; then
  echo "expected Lumelo bring-up overlay tools missing or not executable" >&2
  exit 1
fi

echo "==> running post-build hook"
LUMELO_IMAGE_PROFILE="${LUMELO_IMAGE_PROFILE}" \
BOARD_SOURCE_IMAGE="$(basename "${BOARD_BASE_IMAGE_ABS}")" \
ENABLE_SSH="${ENABLE_SSH}" \
SSH_AUTHORIZED_KEYS_FILE="${SSH_AUTHORIZED_KEYS_FILE_ABS}" \
ROOT_PASSWORD="${ROOT_PASSWORD}" \
  sh "${hook_path}" "${ROOTFS_MOUNT}"

sync
umount "${ROOTFS_MOUNT}"
ROOTFS_MOUNT=
umount "${BASE_ROOTFS_MOUNT}"
BASE_ROOTFS_MOUNT=
losetup -d "${OUTPUT_LOOPDEV}"
OUTPUT_LOOPDEV=
losetup -d "${BASE_LOOPDEV}"
BASE_LOOPDEV=

sha256sum "${OUTPUT_IMAGE_ABS}" > "${OUTPUT_IMAGE_ABS}.sha256"

echo "==> done"
echo "output image: ${OUTPUT_IMAGE_ABS}"
echo "sha256: ${OUTPUT_IMAGE_ABS}.sha256"
~~~

## `scripts/build-t4-smoke-image.sh`

- bytes: 6978
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:${PATH}"
export PATH

: "${CARGO_TARGET_DIR:=/tmp/lumelo-t4-smoke-cargo-target}"
: "${GOCACHE:=/tmp/lumelo-t4-smoke-go-cache}"
export CARGO_TARGET_DIR
export GOCACHE

usage() {
  cat <<'EOF'
Usage:
  build-t4-smoke-image.sh --base-image /path/to/rk3399-sd-*.img[.gz] --output /path/to/lumelo-t4-smoke.img

Environment:
  LUMELO_BUILD_ROOT    Working directory for mounts and staging.
                       Default: /tmp/lumelo-t4-smoke-build
  KEEP_WORKDIR         Keep temporary loop/mount workspace on exit when set to 1.

Notes:
  - This script must run on Linux with root privileges.
  - It remasters an official FriendlyELEC NanoPC-T4 SD image in place.
  - For the first smoke image, it modifies only the rootfs partition and
    reuses the board's existing boot chain from the selected FriendlyELEC base.
EOF
}

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "build-t4-smoke-image.sh must run as root" >&2
    exit 1
  fi
}

require_linux() {
  if [ "$(uname -s)" != "Linux" ]; then
    echo "build-t4-smoke-image.sh must run on Linux" >&2
    exit 1
  fi
}

require_arm64() {
  case "$(uname -m)" in
    aarch64|arm64)
      ;;
    *)
      echo "build-t4-smoke-image.sh currently expects a Linux arm64 build host" >&2
      exit 1
      ;;
  esac
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

cleanup() {
  status=$?
  if [ -n "${ROOTFS_MOUNT:-}" ] && mountpoint -q "${ROOTFS_MOUNT}" 2>/dev/null; then
    umount "${ROOTFS_MOUNT}" || true
  fi
  if [ -n "${LOOPDEV:-}" ]; then
    losetup -d "${LOOPDEV}" || true
  fi
  if [ "${KEEP_WORKDIR:-0}" != "1" ] && [ -n "${WORKDIR:-}" ] && [ -d "${WORKDIR}" ]; then
    rm -rf "${WORKDIR}"
  fi
  exit "${status}"
}

BASE_IMAGE=
OUTPUT_IMAGE=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --base-image)
      BASE_IMAGE="$2"
      shift 2
      ;;
    --output)
      OUTPUT_IMAGE="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ -z "${BASE_IMAGE}" ] || [ -z "${OUTPUT_IMAGE}" ]; then
  usage >&2
  exit 1
fi

require_linux
require_arm64
require_root
require_cmd cargo
require_cmd go
require_cmd gzip
require_cmd losetup
require_cmd mount
require_cmd mountpoint
require_cmd partx
require_cmd rsync
require_cmd sha256sum

BASE_IMAGE_ABS="$(cd "$(dirname "${BASE_IMAGE}")" && pwd)/$(basename "${BASE_IMAGE}")"
OUTPUT_IMAGE_ABS="$(cd "$(dirname "${OUTPUT_IMAGE}")" && pwd)/$(basename "${OUTPUT_IMAGE}")"
if [ ! -f "${BASE_IMAGE_ABS}" ]; then
  echo "base image not found: ${BASE_IMAGE_ABS}" >&2
  exit 1
fi

WORKDIR="${LUMELO_BUILD_ROOT:-/tmp/lumelo-t4-smoke-build}"
WORKDIR="${WORKDIR%/}"
ROOTFS_MOUNT="${WORKDIR}/mnt-rootfs"
STAGE_DIR="${WORKDIR}/stage"
REPO_ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"

rm -rf "${WORKDIR}"
mkdir -p "${ROOTFS_MOUNT}" "${STAGE_DIR}/bin"
mkdir -p "${CARGO_TARGET_DIR}" "${GOCACHE}"

trap cleanup EXIT INT TERM

echo "==> building Linux arm64 binaries"
cargo build \
  --manifest-path "${REPO_ROOT}/services/rust/Cargo.toml" \
  --release \
  -p playbackd \
  -p sessiond \
  -p media-indexd

(cd "${REPO_ROOT}/services/controld" && GOOS=linux GOARCH=arm64 go build -o "${STAGE_DIR}/bin/controld" ./cmd/controld)

install -m 0755 "${CARGO_TARGET_DIR}/release/playbackd" "${STAGE_DIR}/bin/playbackd"
install -m 0755 "${CARGO_TARGET_DIR}/release/sessiond" "${STAGE_DIR}/bin/sessiond"
install -m 0755 "${CARGO_TARGET_DIR}/release/media-indexd" "${STAGE_DIR}/bin/media-indexd"

echo "==> preparing output image"
mkdir -p "$(dirname "${OUTPUT_IMAGE_ABS}")"
case "${BASE_IMAGE_ABS}" in
  *.gz)
    gzip -dc "${BASE_IMAGE_ABS}" > "${OUTPUT_IMAGE_ABS}"
    ;;
  *.img)
    cp "${BASE_IMAGE_ABS}" "${OUTPUT_IMAGE_ABS}"
    ;;
  *)
    echo "unsupported base image format: ${BASE_IMAGE_ABS}" >&2
    exit 1
    ;;
esac

echo "==> mounting rootfs partition"
LOOPDEV="$(losetup --find --partscan --show "${OUTPUT_IMAGE_ABS}")"
ROOTFS_PARTITION="${LOOPDEV}p8"
if [ ! -b "${ROOTFS_PARTITION}" ]; then
  partx -a "${LOOPDEV}" >/dev/null 2>&1 || true
fi
wait_count=0
while [ ! -b "${ROOTFS_PARTITION}" ] && [ "${wait_count}" -lt 10 ]; do
  sleep 1
  wait_count=$((wait_count + 1))
done
if [ ! -b "${ROOTFS_PARTITION}" ]; then
  echo "expected rootfs partition device not found: ${ROOTFS_PARTITION}" >&2
  exit 1
fi
mount "${ROOTFS_PARTITION}" "${ROOTFS_MOUNT}"

echo "==> injecting Lumelo binaries and overlay"
install -d "${ROOTFS_MOUNT}/usr/bin"
install -m 0755 "${STAGE_DIR}/bin/playbackd" "${ROOTFS_MOUNT}/usr/bin/playbackd"
install -m 0755 "${STAGE_DIR}/bin/sessiond" "${ROOTFS_MOUNT}/usr/bin/sessiond"
install -m 0755 "${STAGE_DIR}/bin/media-indexd" "${ROOTFS_MOUNT}/usr/bin/media-indexd"
install -m 0755 "${STAGE_DIR}/bin/controld" "${ROOTFS_MOUNT}/usr/bin/controld"

rsync -a \
  --exclude='._*' \
  --exclude='.DS_Store' \
  "${REPO_ROOT}/base/rootfs/overlay/" "${ROOTFS_MOUNT}/"

echo "==> enabling smoke services"
mkdir -p "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants"
ln -snf ../local-mode.target "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants/local-mode.target"

SYSTEMD_UNIT_DIR=
if [ -d "${ROOTFS_MOUNT}/usr/lib/systemd/system" ]; then
  SYSTEMD_UNIT_DIR=/usr/lib/systemd/system
elif [ -d "${ROOTFS_MOUNT}/lib/systemd/system" ]; then
  SYSTEMD_UNIT_DIR=/lib/systemd/system
fi

if [ -n "${SYSTEMD_UNIT_DIR}" ] && [ -f "${ROOTFS_MOUNT}${SYSTEMD_UNIT_DIR}/systemd-networkd.service" ]; then
  mkdir -p "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants"
  ln -snf "${SYSTEMD_UNIT_DIR}/systemd-networkd.service" \
    "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants/systemd-networkd.service"
fi

if [ -n "${SYSTEMD_UNIT_DIR}" ] && [ -f "${ROOTFS_MOUNT}${SYSTEMD_UNIT_DIR}/systemd-resolved.service" ]; then
  mkdir -p "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants"
  ln -snf "${SYSTEMD_UNIT_DIR}/systemd-resolved.service" \
    "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants/systemd-resolved.service"
  ln -snf /run/systemd/resolve/stub-resolv.conf "${ROOTFS_MOUNT}/etc/resolv.conf"
fi

if [ -n "${SYSTEMD_UNIT_DIR}" ] && [ -f "${ROOTFS_MOUNT}${SYSTEMD_UNIT_DIR}/ssh.service" ]; then
  mkdir -p "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants"
  ln -snf "${SYSTEMD_UNIT_DIR}/ssh.service" \
    "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants/ssh.service"
fi

printf '%s\n' "Lumelo T4 smoke image built from $(basename "${BASE_IMAGE_ABS}") on $(date -u '+%Y-%m-%dT%H:%M:%SZ')" \
  > "${ROOTFS_MOUNT}/etc/lumelo/smoke-build.txt"

sync
umount "${ROOTFS_MOUNT}"
LOOPDEV_TO_DETACH="${LOOPDEV}"
LOOPDEV=
losetup -d "${LOOPDEV_TO_DETACH}"

sha256sum "${OUTPUT_IMAGE_ABS}" > "${OUTPUT_IMAGE_ABS}.sha256"

echo "==> done"
echo "output image: ${OUTPUT_IMAGE_ABS}"
echo "sha256: ${OUTPUT_IMAGE_ABS}.sha256"
~~~

## `scripts/build-t4-ssh-bringup-image.sh`

- bytes: 2564
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  build-t4-ssh-bringup-image.sh [--ssh-authorized-keys /path/to/id_ed25519.pub] [--output /path/to/image.img]

Options:
  --ssh-authorized-keys  Optional public key file to install as
                         /root/.ssh/authorized_keys.
  --board-base-image     FriendlyELEC NanoPC-T4 base image. Defaults to the
                         checked-in bring-up base image under out/t4-smoke.
  --output               Output image. Defaults to out/t4-rootfs/lumelo-t4-rootfs-ssh-YYYYMMDD.img.

Notes:
  - Runs on Linux arm64 with root privileges.
  - This is a bring-up convenience wrapper only.
  - Development / bring-up images now default to SSH enabled.
  - Root password login follows the underlying build defaults.
  - Pass --ssh-authorized-keys when you also want key-based access.
EOF
}

need_value() {
  option=$1
  if [ "$#" -lt 2 ] || [ -z "${2:-}" ]; then
    echo "${option} requires a value" >&2
    exit 1
  fi
}

SSH_AUTHORIZED_KEYS=
BOARD_BASE_IMAGE=
OUTPUT_IMAGE=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --ssh-authorized-keys)
      need_value "$@"
      SSH_AUTHORIZED_KEYS=$2
      shift 2
      ;;
    --board-base-image)
      need_value "$@"
      BOARD_BASE_IMAGE=$2
      shift 2
      ;;
    --output)
      need_value "$@"
      OUTPUT_IMAGE=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

REPO_ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
today="$(date +%Y%m%d)"

if [ -z "${BOARD_BASE_IMAGE}" ]; then
  BOARD_BASE_IMAGE="${REPO_ROOT}/out/t4-smoke/rk3399-sd-debian-trixie-core-4.19-arm64-20260319.img.gz"
fi

if [ -z "${OUTPUT_IMAGE}" ]; then
  OUTPUT_IMAGE="${REPO_ROOT}/out/t4-rootfs/lumelo-t4-rootfs-ssh-${today}.img"
fi

if [ -n "${SSH_AUTHORIZED_KEYS}" ]; then
  ENABLE_SSH=1 \
  SSH_AUTHORIZED_KEYS_FILE="${SSH_AUTHORIZED_KEYS}" \
    "${REPO_ROOT}/scripts/build-t4-lumelo-rootfs-image.sh" \
      --board-base-image "${BOARD_BASE_IMAGE}" \
      --output "${OUTPUT_IMAGE}"
else
  ENABLE_SSH=1 \
    "${REPO_ROOT}/scripts/build-t4-lumelo-rootfs-image.sh" \
      --board-base-image "${BOARD_BASE_IMAGE}" \
      --output "${OUTPUT_IMAGE}"
fi

printf '\nSSH bring-up image ready:\n'
printf '  image:  %s\n' "${OUTPUT_IMAGE}"
printf '  sha256: %s.sha256\n' "${OUTPUT_IMAGE}"
printf '\nAfter flashing, log in with:\n'
printf '  ssh root@<T4_IP>\n'
printf '\nOptional:\n'
printf '  add --ssh-authorized-keys /path/to/id_ed25519.pub for key injection\n'
~~~

## `scripts/compare-t4-wireless-golden.sh`

- bytes: 7254
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  compare-t4-wireless-golden.sh \
    --board-base-image /path/to/rk3399-sd-*.img[.gz] \
    --image /path/to/lumelo-t4-rootfs-YYYYMMDD-vN.img

Notes:
  - Runs on Linux with root privileges.
  - Mounts partition p8 from both images read-only.
  - Compares the key NanoPC-T4 wireless board-support assets against the
    FriendlyELEC base image used as the current golden sample.
EOF
}

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "compare-t4-wireless-golden.sh must run as root" >&2
    exit 1
  fi
}

require_linux() {
  if [ "$(uname -s)" != "Linux" ]; then
    echo "compare-t4-wireless-golden.sh must run on Linux" >&2
    exit 1
  fi
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

wait_for_partition() {
  partition=$1
  count=0
  while [ ! -b "${partition}" ] && [ "${count}" -lt 10 ]; do
    sleep 1
    count=$((count + 1))
  done
  [ -b "${partition}" ]
}

pass() {
  printf 'PASS %s\n' "$1"
}

warn() {
  WARNINGS=$((WARNINGS + 1))
  printf 'WARN %s\n' "$1"
}

fail() {
  FAILURES=$((FAILURES + 1))
  printf 'FAIL %s\n' "$1"
}

compare_file_hash() {
  rel_path=$1
  label=$2

  base_path="${BASE_ROOTFS_MOUNT}${rel_path}"
  target_path="${TARGET_ROOTFS_MOUNT}${rel_path}"

  if [ ! -f "${base_path}" ]; then
    fail "${label}: missing in base image ${rel_path}"
    return
  fi

  if [ ! -f "${target_path}" ]; then
    fail "${label}: missing in target image ${rel_path}"
    return
  fi

  base_sha=$(sha256sum "${base_path}" | awk '{print $1}')
  target_sha=$(sha256sum "${target_path}" | awk '{print $1}')

  if [ "${base_sha}" = "${target_sha}" ]; then
    pass "${label}: ${target_sha}"
  else
    fail "${label}: target ${target_sha} != base ${base_sha}"
  fi
}

compare_text_file() {
  rel_path=$1
  label=$2

  base_path="${BASE_ROOTFS_MOUNT}${rel_path}"
  target_path="${TARGET_ROOTFS_MOUNT}${rel_path}"

  if [ ! -f "${base_path}" ]; then
    fail "${label}: missing in base image ${rel_path}"
    return
  fi

  if [ ! -f "${target_path}" ]; then
    fail "${label}: missing in target image ${rel_path}"
    return
  fi

  if diff -u "${base_path}" "${target_path}" >/dev/null 2>&1; then
    pass "${label}: ${rel_path}"
  else
    fail "${label}: differs from base ${rel_path}"
    diff -u "${base_path}" "${target_path}" || true
  fi
}

expect_target_text() {
  rel_path=$1
  needle=$2
  label=$3

  target_path="${TARGET_ROOTFS_MOUNT}${rel_path}"
  if [ ! -f "${target_path}" ]; then
    fail "${label}: missing in target image ${rel_path}"
    return
  fi

  if grep -F "${needle}" "${target_path}" >/dev/null 2>&1; then
    pass "${label}: found '${needle}'"
  else
    fail "${label}: missing '${needle}' in ${rel_path}"
  fi
}

cleanup() {
  if mountpoint -q "${TARGET_ROOTFS_MOUNT}" 2>/dev/null; then
    umount "${TARGET_ROOTFS_MOUNT}" || true
  fi
  if mountpoint -q "${BASE_ROOTFS_MOUNT}" 2>/dev/null; then
    umount "${BASE_ROOTFS_MOUNT}" || true
  fi
  if [ -n "${TARGET_LOOPDEV:-}" ]; then
    losetup -d "${TARGET_LOOPDEV}" || true
  fi
  if [ -n "${BASE_LOOPDEV:-}" ]; then
    losetup -d "${BASE_LOOPDEV}" || true
  fi
  rm -rf "${WORKDIR}"
}

BOARD_BASE_IMAGE=
TARGET_IMAGE=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --board-base-image)
      BOARD_BASE_IMAGE=$2
      shift 2
      ;;
    --image)
      TARGET_IMAGE=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ -z "${BOARD_BASE_IMAGE}" ] || [ -z "${TARGET_IMAGE}" ]; then
  usage >&2
  exit 1
fi

require_root
require_linux
require_cmd losetup
require_cmd partx
require_cmd mount
require_cmd umount
require_cmd sha256sum
require_cmd diff
require_cmd grep

BOARD_BASE_IMAGE_ABS=$(readlink -f "${BOARD_BASE_IMAGE}")
TARGET_IMAGE_ABS=$(readlink -f "${TARGET_IMAGE}")

if [ ! -f "${BOARD_BASE_IMAGE_ABS}" ]; then
  echo "board base image not found: ${BOARD_BASE_IMAGE_ABS}" >&2
  exit 1
fi

if [ ! -f "${TARGET_IMAGE_ABS}" ]; then
  echo "target image not found: ${TARGET_IMAGE_ABS}" >&2
  exit 1
fi

TMP_BASE=${TMPDIR:-/var/tmp}
WORKDIR=$(mktemp -d "${TMP_BASE%/}/lumelo-wireless-golden-compare.XXXXXX")
BASE_RAW_IMAGE="${WORKDIR}/base.img"
BASE_ROOTFS_MOUNT="${WORKDIR}/base-rootfs"
TARGET_ROOTFS_MOUNT="${WORKDIR}/target-rootfs"
mkdir -p "${BASE_ROOTFS_MOUNT}" "${TARGET_ROOTFS_MOUNT}"

FAILURES=0
WARNINGS=0
BASE_LOOPDEV=
TARGET_LOOPDEV=

trap cleanup EXIT INT TERM

case "${BOARD_BASE_IMAGE_ABS}" in
  *.img)
    cp "${BOARD_BASE_IMAGE_ABS}" "${BASE_RAW_IMAGE}"
    ;;
  *.img.gz|*.gz)
    gzip -dc "${BOARD_BASE_IMAGE_ABS}" > "${BASE_RAW_IMAGE}"
    ;;
  *)
    echo "unsupported board base image format: ${BOARD_BASE_IMAGE_ABS}" >&2
    exit 1
    ;;
esac

BASE_LOOPDEV=$(losetup --find --partscan --show "${BASE_RAW_IMAGE}")
partx -a "${BASE_LOOPDEV}" >/dev/null 2>&1 || true
TARGET_LOOPDEV=$(losetup --find --partscan --show "${TARGET_IMAGE_ABS}")
partx -a "${TARGET_LOOPDEV}" >/dev/null 2>&1 || true

BASE_ROOTFS_PART="${BASE_LOOPDEV}p8"
TARGET_ROOTFS_PART="${TARGET_LOOPDEV}p8"

wait_for_partition "${BASE_ROOTFS_PART}" || {
  echo "missing base rootfs partition device: ${BASE_ROOTFS_PART}" >&2
  exit 1
}
wait_for_partition "${TARGET_ROOTFS_PART}" || {
  echo "missing target rootfs partition device: ${TARGET_ROOTFS_PART}" >&2
  exit 1
}

mount -o ro "${BASE_ROOTFS_PART}" "${BASE_ROOTFS_MOUNT}"
mount -o ro "${TARGET_ROOTFS_PART}" "${TARGET_ROOTFS_MOUNT}"

printf 'Comparing base image: %s\n' "${BOARD_BASE_IMAGE_ABS}"
printf 'Against target image: %s\n\n' "${TARGET_IMAGE_ABS}"

compare_file_hash /etc/firmware/BCM4356A2.hcd "official bluetooth patch firmware"
compare_file_hash /usr/bin/hciattach.rk "official bluetooth UART attach helper"
compare_text_file /etc/modprobe.d/bcmdhd.conf "official bcmdhd driver policy"
compare_file_hash /system/etc/firmware/fw_bcm4356a2_ag.bin "official Broadcom Wi-Fi firmware blob"
compare_file_hash /system/etc/firmware/nvram_ap6356.txt "official AP6356 NVRAM calibration"

expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach "/sys/module/bcmdhd" "attach waits for bcmdhd"
expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach "/sys/class/rfkill/rfkill0/state" "attach toggles rfkill0 state"
expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach "rm -f /dev/rfkill" "attach clears stale rfkill node"
expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach "timeout 5 btmgmt info" "attach bounds btmgmt probe"
expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach "grep -Eq '^hci[0-9]+:'" "attach requires discovered hci controller"
expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach 'exec "${ATTACH_HELPER}" "${ATTACH_UART}" "${ATTACH_CHIPSET}" "${ATTACH_BAUD}"' "attach exec wiring"
expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach "/dev/ttyS0" "attach default UART"
expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach "bcm43xx" "attach default chipset"
expect_target_text /usr/libexec/lumelo/bluetooth-uart-attach "1500000" "attach default baud"

printf '\nSummary: %s failure(s), %s warning(s)\n' "${FAILURES}" "${WARNINGS}"

if [ "${FAILURES}" -ne 0 ]; then
  exit 1
fi
~~~

