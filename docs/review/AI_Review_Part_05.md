# AI Review Part 05

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `base/rootfs/overlay/etc/systemd/system/lumelo-ssh-hostkeys.service`

- bytes: 186
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo SSH Host Key Generation
ConditionPathExists=/usr/bin/ssh-keygen
Before=ssh.service

[Service]
Type=oneshot
ExecStart=/usr/bin/ssh-keygen -A
RemainAfterExit=yes
~~~

## `base/rootfs/overlay/etc/systemd/system/lumelo-wifi-provisiond.service`

- bytes: 475
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Bluetooth Wi-Fi Provisioning Service
After=bluetooth.service dbus.service lumelo-bluetooth-provisioning.service
Requires=bluetooth.service
ConditionPathExists=/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond

[Service]
Type=simple
ExecStart=/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond
Restart=on-failure
RestartSec=2
RuntimeDirectory=lumelo
RuntimeDirectoryMode=0755
RuntimeDirectoryPreserve=yes

[Install]
WantedBy=multi-user.target
~~~

## `base/rootfs/overlay/etc/systemd/system/media-indexd.service`

- bytes: 411
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Media Index Worker
After=local-fs.target
ConditionPathExists=/usr/bin/media-indexd

[Service]
Type=simple
Environment=LUMELO_RUNTIME_DIR=/run/lumelo
ExecStart=/usr/bin/media-indexd
RuntimeDirectory=lumelo
RuntimeDirectoryMode=0755
RuntimeDirectoryPreserve=yes
StateDirectory=lumelo
StateDirectoryMode=0755
CacheDirectory=lumelo
CacheDirectoryMode=0755
WorkingDirectory=/var/lib/lumelo
~~~

## `base/rootfs/overlay/etc/systemd/system/playbackd.service`

- bytes: 470
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Playback Core
After=local-fs.target
ConditionPathExists=/usr/bin/playbackd

[Service]
Type=simple
Environment=LUMELO_RUNTIME_DIR=/run/lumelo
ExecStart=/usr/bin/playbackd
Restart=on-failure
RestartSec=2
RuntimeDirectory=lumelo
RuntimeDirectoryMode=0755
RuntimeDirectoryPreserve=yes
StateDirectory=lumelo
StateDirectoryMode=0755
CacheDirectory=lumelo
CacheDirectoryMode=0755
WorkingDirectory=/var/lib/lumelo

[Install]
WantedBy=local-mode.target
~~~

## `base/rootfs/overlay/etc/systemd/system/sessiond.service`

- bytes: 427
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Quiet Mode Session Service
After=playbackd.service
Requires=playbackd.service
ConditionPathExists=/usr/bin/sessiond

[Service]
Type=simple
Environment=LUMELO_RUNTIME_DIR=/run/lumelo
EnvironmentFile=-/etc/lumelo/sessiond.env
ExecStart=/usr/bin/sessiond
Restart=on-failure
RestartSec=2
RuntimeDirectory=lumelo
RuntimeDirectoryMode=0755
RuntimeDirectoryPreserve=yes

[Install]
WantedBy=local-mode.target
~~~

## `base/rootfs/overlay/etc/systemd/system/ssh.service.d/10-lumelo-hostkeys.conf`

- bytes: 78
- segment: 1/1

~~~ini
[Unit]
Requires=lumelo-ssh-hostkeys.service
After=lumelo-ssh-hostkeys.service
~~~

## `base/rootfs/overlay/etc/udev/rules.d/90-lumelo-media-import.rules`

- bytes: 694
- segment: 1/1

~~~udev
ACTION=="add|change", SUBSYSTEM=="block", ENV{DEVTYPE}=="partition", ENV{ID_FS_USAGE}=="filesystem", ATTRS{removable}=="1", TAG+="systemd", ENV{SYSTEMD_WANTS}+="lumelo-media-import@%k.service"
ACTION=="add|change", SUBSYSTEM=="block", ENV{DEVTYPE}=="partition", ENV{ID_FS_USAGE}=="filesystem", ENV{ID_BUS}=="usb", TAG+="systemd", ENV{SYSTEMD_WANTS}+="lumelo-media-import@%k.service"
ACTION=="remove", SUBSYSTEM=="block", ENV{DEVTYPE}=="partition", ATTRS{removable}=="1", TAG+="systemd", ENV{SYSTEMD_WANTS}+="lumelo-media-reconcile.service"
ACTION=="remove", SUBSYSTEM=="block", ENV{DEVTYPE}=="partition", ENV{ID_BUS}=="usb", TAG+="systemd", ENV{SYSTEMD_WANTS}+="lumelo-media-reconcile.service"
~~~

## `base/rootfs/overlay/etc/wpa_supplicant/wpa_supplicant-wlan0.conf`

- bytes: 128
- segment: 1/1

~~~ini
ctrl_interface=DIR=/run/wpa_supplicant GROUP=netdev
update_config=1

# Lumelo provisioning writes the first network block here.
~~~

## `base/rootfs/overlay/usr/bin/lumelo-audio-smoke`

- bytes: 1644
- segment: 1/1

~~~text
#!/bin/sh
set -u

DEVICE=${1:-${LUMELO_AUDIO_DEVICE:-default}}
RATE=${LUMELO_AUDIO_SMOKE_RATE:-48000}
CHANNELS=${LUMELO_AUDIO_SMOKE_CHANNELS:-2}
FORMAT=${LUMELO_AUDIO_SMOKE_FORMAT:-S16_LE}
FREQUENCY=${LUMELO_AUDIO_SMOKE_FREQUENCY:-1000}
LOOPS=${LUMELO_AUDIO_SMOKE_LOOPS:-1}

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
  return "${status}"
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
    return 127
  fi
}

cat <<EOF
Lumelo ALSA smoke test

This is a manual bring-up helper. It does not start playbackd, does not enable
Quiet Mode, and does not change persistent audio configuration.

Target device: ${DEVICE}
Format: ${FORMAT}, ${RATE} Hz, ${CHANNELS} channel(s), ${FREQUENCY} Hz tone
EOF

section "ALSA Inventory"
cat_if_readable /proc/asound/cards
cat_if_readable /proc/asound/devices
command_if_present aplay -l || true
command_if_present aplay -L || true

section "Mixer"
command_if_present amixer || true

section "Tone"
if ! command -v speaker-test >/dev/null 2>&1; then
  printf 'speaker-test is not installed. Rebuild the bring-up image with alsa-utils.\n'
  exit 127
fi

run speaker-test \
  -D "${DEVICE}" \
  -c "${CHANNELS}" \
  -r "${RATE}" \
  -F "${FORMAT}" \
  -t sine \
  -f "${FREQUENCY}" \
  -l "${LOOPS}"
~~~

## `base/rootfs/overlay/usr/bin/lumelo-bluetooth-provisioning-mode`

- bytes: 664
- segment: 1/1

~~~text
#!/bin/sh
set -eu

echo "Lumelo Bluetooth provisioning mode"

if ! command -v bluetoothctl >/dev/null 2>&1; then
  echo "bluetoothctl is unavailable; skipping Bluetooth provisioning mode" >&2
  exit 0
fi

if ! command -v rfkill >/dev/null 2>&1; then
  echo "rfkill is unavailable; continuing without unblock step" >&2
else
  rfkill unblock bluetooth || true
  rfkill unblock all || true
fi

if command -v hciconfig >/dev/null 2>&1; then
  hciconfig hci0 up || true
  hciconfig hci0 name "Lumelo T4" || true
  hciconfig hci0 piscan || true
fi

bluetoothctl <<'EOF' || true
power on
system-alias Lumelo T4
discoverable-timeout 0
pairable on
discoverable on
show
EOF
~~~

## `base/rootfs/overlay/usr/bin/lumelo-media-import`

- bytes: 14477
- segment: 1/1

~~~text
#!/usr/bin/env python3
import argparse
import json
import os
import re
import sqlite3
import subprocess
import sys
import time
from pathlib import Path
from typing import Dict, Iterable, List, Optional


QUIET_MODE_PATH = Path("/run/lumelo/quiet_mode")
MEDIA_INDEX_BIN = os.environ.get("LUMELO_MEDIA_INDEX_BIN", "media-indexd")
DEFAULT_LIBRARY_DB = Path("/var/lib/lumelo/library.db")
DEFAULT_MOUNT_BASE = Path("/media")


def run(
    argv: List[str],
    *,
    check: bool = True,
    env: Optional[Dict[str, str]] = None,
    capture_output: bool = False,
) -> subprocess.CompletedProcess:
    print("$", " ".join(argv))
    return subprocess.run(
        argv,
        check=check,
        text=True,
        env=env,
        capture_output=capture_output,
    )


def quiet_mode_state() -> str:
    if not QUIET_MODE_PATH.exists():
        return "off"
    try:
        return QUIET_MODE_PATH.read_text(encoding="utf-8").strip() or "active"
    except OSError:
        return "active"


def quiet_mode_active() -> bool:
    return quiet_mode_state() != "off"


def flatten_lsblk_devices(nodes: Iterable[Dict[str, object]]) -> Iterable[Dict[str, object]]:
    for node in nodes:
        yield node
        children = node.get("children")
        if isinstance(children, list):
            yield from flatten_lsblk_devices(children)


def normalize_mountpoints(raw: object) -> List[str]:
    mountpoints: List[str] = []
    if isinstance(raw, list):
        for item in raw:
            if isinstance(item, str) and item:
                mountpoints.append(item)
    elif isinstance(raw, str) and raw:
        mountpoints.append(raw)
    return mountpoints


def sanitize_component(raw: str, fallback: str) -> str:
    candidate = re.sub(r"[^a-z0-9._-]+", "-", raw.strip().lower()).strip(".-_")
    return candidate or fallback


def blkid_export(device: Path) -> Dict[str, str]:
    completed = run(
        ["blkid", "-o", "export", str(device)],
        check=False,
        capture_output=True,
    )
    if completed.returncode != 0:
        return {}
    payload: Dict[str, str] = {}
    for line in completed.stdout.splitlines():
        key, sep, value = line.partition("=")
        if sep and key:
            payload[key] = value
    return payload


def stable_volume_uuid(device: Path, blkid_info: Dict[str, str]) -> str:
    if blkid_info.get("UUID"):
        return "media-uuid-" + sanitize_component(blkid_info["UUID"], "device")
    if blkid_info.get("PARTUUID"):
        return "media-partuuid-" + sanitize_component(blkid_info["PARTUUID"], "device")
    return "media-dev-" + sanitize_component(device.name, "device")


def device_mount_target(device: Path) -> Optional[Path]:
    completed = run(
        ["findmnt", "-rn", "--source", str(device), "-o", "TARGET"],
        check=False,
        capture_output=True,
    )
    if completed.returncode != 0:
        return None
    target = completed.stdout.strip().splitlines()
    if not target:
        return None
    return Path(target[0])


def mountpoint_active(path: Path) -> bool:
    completed = run(
        ["findmnt", "-rn", "-M", str(path), "-o", "TARGET"],
        check=False,
        capture_output=True,
    )
    return completed.returncode == 0


def build_mountpoint(device: Path, blkid_info: Dict[str, str], mount_base: Path) -> Path:
    raw_name = (
        blkid_info.get("LABEL")
        or blkid_info.get("UUID")
        or blkid_info.get("PARTUUID")
        or device.name
    )
    component = sanitize_component(raw_name, sanitize_component(device.name, "media"))
    return mount_base / component


def wait_for_mount(device: Path, mountpoint: Path, timeout: float) -> Path:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        target = device_mount_target(device)
        if target is not None:
            return target
        if mountpoint_active(mountpoint):
            return mountpoint
        time.sleep(0.2)
    raise SystemExit(f"timed out waiting for mount: {device} -> {mountpoint}")


def ensure_device_mounted(
    device: Path, mount_base: Path, timeout: float
) -> tuple[Path, Dict[str, str], bool]:
    blkid_info = blkid_export(device)
    existing_target = device_mount_target(device)
    if existing_target is not None:
        return existing_target, blkid_info, False

    mountpoint = build_mountpoint(device, blkid_info, mount_base)
    mountpoint.mkdir(parents=True, exist_ok=True)
    run(
        [
            "systemd-mount",
            "--no-pager",
            "--no-ask-password",
            "--collect",
            "--automount=no",
            str(device),
            str(mountpoint),
        ]
    )
    return wait_for_mount(device, mountpoint, timeout), blkid_info, True


def discover_mounted_removable_media() -> List[Dict[str, str]]:
    completed = run(
        [
            "lsblk",
            "-J",
            "-o",
            "NAME,PATH,TYPE,RM,TRAN,LABEL,FSTYPE,MOUNTPOINTS",
        ],
        capture_output=True,
    )
    payload = json.loads(completed.stdout)
    devices = payload.get("blockdevices", [])
    results: List[Dict[str, str]] = []
    seen_mounts = set()
    for device in flatten_lsblk_devices(devices):
        mountpoints = normalize_mountpoints(device.get("mountpoints"))
        if not mountpoints:
            continue
        if str(device.get("type") or "") not in {"part", "disk"}:
            continue
        removable = bool(device.get("rm"))
        transport = str(device.get("tran") or "")
        if not removable and transport != "usb":
            continue
        device_path = Path(str(device.get("path") or ""))
        blkid_info = blkid_export(device_path) if str(device_path) else {}
        for mountpoint in mountpoints:
            if mountpoint in seen_mounts:
                continue
            if not mountpoint.startswith(("/media/", "/mnt/")):
                continue
            seen_mounts.add(mountpoint)
            results.append(
                {
                    "name": str(device.get("name") or ""),
                    "path": str(device_path),
                    "label": str(device.get("label") or blkid_info.get("LABEL", "")),
                    "uuid": blkid_info.get("UUID", ""),
                    "fstype": str(device.get("fstype") or blkid_info.get("TYPE", "")),
                    "transport": transport,
                    "mountpoint": mountpoint,
                    "volume_uuid": stable_volume_uuid(device_path, blkid_info),
                }
            )
    results.sort(key=lambda item: item["mountpoint"])
    return results


def scan_dir(path: Path, volume_uuid: Optional[str] = None) -> int:
    if not path.exists():
        print(f"scan path does not exist: {path}", file=sys.stderr)
        return 66
    if not path.is_dir():
        print(f"scan path is not a directory: {path}", file=sys.stderr)
        return 66
    env = os.environ.copy()
    if volume_uuid:
        env["MEDIA_INDEX_VOLUME_UUID"] = volume_uuid
    run([MEDIA_INDEX_BIN, "scan-dir", str(path)], env=env)
    return 0


def reconcile_volumes(db_path: Path) -> List[Dict[str, object]]:
    if not db_path.exists():
        print(f"library db does not exist yet: {db_path}", file=sys.stderr)
        return []

    connection = sqlite3.connect(str(db_path))
    connection.row_factory = sqlite3.Row
    rows = list(
        connection.execute(
            """
            SELECT volume_uuid, mount_path, is_available, last_seen_at
            FROM volumes
            WHERE mount_path LIKE '/media/%' OR mount_path LIKE '/mnt/%'
            ORDER BY mount_path ASC
            """
        )
    )

    updated: List[Dict[str, object]] = []
    now = int(time.time())
    for row in rows:
        mount_path = Path(str(row["mount_path"]))
        is_available = 1 if mountpoint_active(mount_path) else 0
        if is_available == int(row["is_available"]):
            continue
        if is_available:
            connection.execute(
                "UPDATE volumes SET is_available = 1, last_seen_at = ?2 WHERE volume_uuid = ?1",
                (row["volume_uuid"], now),
            )
        else:
            connection.execute(
                "UPDATE volumes SET is_available = 0 WHERE volume_uuid = ?1",
                (row["volume_uuid"],),
            )
        updated.append(
            {
                "volume_uuid": row["volume_uuid"],
                "mount_path": row["mount_path"],
                "is_available": bool(is_available),
            }
        )

    connection.commit()
    connection.close()
    return updated


def command_list_mounted(_args: argparse.Namespace) -> int:
    mounted = discover_mounted_removable_media()
    print(json.dumps(mounted, ensure_ascii=False, indent=2))
    return 0


def command_scan_path(args: argparse.Namespace) -> int:
    if quiet_mode_active() and not args.force:
        print(
            "playback quiet mode is active; refusing media scan unless --force is used",
            file=sys.stderr,
        )
        return 75
    code = scan_dir(Path(args.path))
    if code == 0:
        reconcile_volumes(Path(args.db))
    return code


def command_scan_mounted(args: argparse.Namespace) -> int:
    if quiet_mode_active() and not args.force:
        print(
            "playback quiet mode is active; refusing mounted media scan unless --force is used",
            file=sys.stderr,
        )
        return 75
    mounted = discover_mounted_removable_media()
    if not mounted:
        print("no mounted removable media found under /media or /mnt", file=sys.stderr)
        return 65
    for item in mounted:
        print(
            json.dumps(
                {
                    "mountpoint": item["mountpoint"],
                    "label": item["label"],
                    "path": item["path"],
                    "fstype": item["fstype"],
                    "volume_uuid": item["volume_uuid"],
                },
                ensure_ascii=False,
            )
        )
        scan_dir(Path(item["mountpoint"]), str(item["volume_uuid"]))
    updates = reconcile_volumes(Path(args.db))
    if updates:
        print(json.dumps({"reconciled": updates}, ensure_ascii=False, indent=2))
    return 0


def command_import_device(args: argparse.Namespace) -> int:
    device = Path(args.device)
    if not device.exists():
        print(f"device path does not exist: {device}", file=sys.stderr)
        return 66

    mountpoint, blkid_info, mounted_now = ensure_device_mounted(
        device, Path(args.mount_base), args.timeout
    )
    volume_uuid = stable_volume_uuid(device, blkid_info)
    print(
        json.dumps(
            {
                "device": str(device),
                "mountpoint": str(mountpoint),
                "mounted_now": mounted_now,
                "label": blkid_info.get("LABEL", ""),
                "uuid": blkid_info.get("UUID", ""),
                "fstype": blkid_info.get("TYPE", ""),
                "volume_uuid": volume_uuid,
            },
            ensure_ascii=False,
        )
    )

    if args.mount_only:
        updates = reconcile_volumes(Path(args.db))
        if updates:
            print(json.dumps({"reconciled": updates}, ensure_ascii=False, indent=2))
        return 0

    if quiet_mode_active() and not args.force:
        print(
            "playback quiet mode is active; mounted device but skipped scan unless --force is used",
            file=sys.stderr,
        )
        updates = reconcile_volumes(Path(args.db))
        if updates:
            print(json.dumps({"reconciled": updates}, ensure_ascii=False, indent=2))
        return 0

    code = scan_dir(mountpoint, volume_uuid)
    if code == 0:
        updates = reconcile_volumes(Path(args.db))
        if updates:
            print(json.dumps({"reconciled": updates}, ensure_ascii=False, indent=2))
    return code


def command_reconcile_volumes(args: argparse.Namespace) -> int:
    updates = reconcile_volumes(Path(args.db))
    print(json.dumps({"reconciled": updates}, ensure_ascii=False, indent=2))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Lumelo removable-media import helper."
    )
    subparsers = parser.add_subparsers(dest="command")

    list_cmd = subparsers.add_parser(
        "list-mounted",
        help="List currently mounted removable media candidates under /media or /mnt.",
    )
    list_cmd.set_defaults(func=command_list_mounted)

    scan_path_cmd = subparsers.add_parser(
        "scan-path",
        help="Scan one explicit directory into library.db via media-indexd.",
    )
    scan_path_cmd.add_argument("path")
    scan_path_cmd.add_argument("--db", default=str(DEFAULT_LIBRARY_DB))
    scan_path_cmd.add_argument("--force", action="store_true")
    scan_path_cmd.set_defaults(func=command_scan_path)

    scan_mounted_cmd = subparsers.add_parser(
        "scan-mounted",
        help="Scan all currently mounted removable media candidates.",
    )
    scan_mounted_cmd.add_argument("--db", default=str(DEFAULT_LIBRARY_DB))
    scan_mounted_cmd.add_argument("--force", action="store_true")
    scan_mounted_cmd.set_defaults(func=command_scan_mounted)

    import_device_cmd = subparsers.add_parser(
        "import-device",
        help="Mount one block device under /media, then scan it into library.db.",
    )
    import_device_cmd.add_argument("device")
    import_device_cmd.add_argument("--db", default=str(DEFAULT_LIBRARY_DB))
    import_device_cmd.add_argument("--mount-base", default=str(DEFAULT_MOUNT_BASE))
    import_device_cmd.add_argument("--mount-only", action="store_true")
    import_device_cmd.add_argument("--timeout", type=float, default=10.0)
    import_device_cmd.add_argument("--force", action="store_true")
    import_device_cmd.set_defaults(func=command_import_device)

    reconcile_cmd = subparsers.add_parser(
        "reconcile-volumes",
        help="Mark /media and /mnt volumes online/offline based on current mount state.",
    )
    reconcile_cmd.add_argument("--db", default=str(DEFAULT_LIBRARY_DB))
    reconcile_cmd.set_defaults(func=command_reconcile_volumes)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    if not getattr(args, "command", None):
        parser.print_help(sys.stderr)
        return 64
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
~~~

