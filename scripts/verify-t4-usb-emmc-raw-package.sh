#!/bin/sh
set -eu

PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"

usage() {
  cat <<'EOF'
Usage:
  verify-t4-usb-emmc-raw-package.sh /path/to/lumelo-t4-usb-emmc-raw-YYYYMMDD-vN

Notes:
  - Runs offline and does not connect to T4 or write eMMC.
  - Verifies the experimental raw full-disk package structure.
  - This is NOT the Win11 RKDevTool Download Image package verifier.
  - Calls verify-t4-lumelo-rootfs-image.sh on the packaged raw image.
  - Must run on Linux as root because the rootfs image verifier mounts p8.
EOF
}

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "verify-t4-usb-emmc-raw-package.sh must run as root" >&2
    exit 1
  fi
}

require_linux() {
  if [ "$(uname -s)" != "Linux" ]; then
    echo "verify-t4-usb-emmc-raw-package.sh must run on Linux" >&2
    exit 1
  fi
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

pass() {
  printf 'PASS %s\n' "$1"
}

info() {
  printf 'INFO %s\n' "$1"
}

fail() {
  FAILURES=$((FAILURES + 1))
  printf 'FAIL %s\n' "$1"
}

expect_file() {
  path=$1
  label=$2
  if [ -f "${path}" ]; then
    pass "${label}: ${path}"
  else
    fail "${label}: missing ${path}"
  fi
}

if [ "$#" -eq 1 ]; then
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
  esac
fi

if [ "$#" -ne 1 ]; then
  usage >&2
  exit 1
fi

PACKAGE_DIR=$1
if [ ! -d "${PACKAGE_DIR}" ]; then
  echo "package directory not found: ${PACKAGE_DIR}" >&2
  exit 1
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd -P)
VERIFY_ROOTFS="${SCRIPT_DIR}/verify-t4-lumelo-rootfs-image.sh"

require_linux
require_root
require_cmd awk
require_cmd python3
require_cmd sha256sum
require_cmd uname

FAILURES=0

MANIFEST="${PACKAGE_DIR}/manifest.json"
SUMS="${PACKAGE_DIR}/SHA256SUMS.txt"
README="${PACKAGE_DIR}/README-WIN11-RKDEVTOOL.md"
LAYOUT="${PACKAGE_DIR}/flash-layout-notes.txt"
LOADER="${PACKAGE_DIR}/MiniLoaderAll.bin"

expect_file "${MANIFEST}" "manifest"
expect_file "${SUMS}" "checksums"
expect_file "${README}" "Win11 RKDevTool README"
expect_file "${LAYOUT}" "flash layout notes"
expect_file "${LOADER}" "MiniLoaderAll loader"

if [ "${FAILURES}" -ne 0 ]; then
  printf 'Summary: %s failure(s)\n' "${FAILURES}"
  exit 1
fi

(
  cd "${PACKAGE_DIR}"
  sha256sum -c SHA256SUMS.txt
)
pass "SHA256SUMS.txt validates"

export PACKAGE_DIR
python3 - <<'PY'
import hashlib
import json
import os
import struct
import sys


EXPECTED_NAMES = [
    "uboot",
    "trust",
    "misc",
    "dtbo",
    "resource",
    "kernel",
    "boot",
    "rootfs",
    "userdata",
]


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_gpt(image_path):
    with open(image_path, "rb") as f:
        f.seek(512)
        header = f.read(92)
        if header[:8] != b"EFI PART":
            raise ValueError(f"missing GPT header in {image_path}")
        entries_lba = struct.unpack_from("<Q", header, 72)[0]
        entry_count = struct.unpack_from("<I", header, 80)[0]
        entry_size = struct.unpack_from("<I", header, 84)[0]
        f.seek(entries_lba * 512)
        raw_entries = f.read(entry_count * entry_size)

    partitions = []
    for index in range(entry_count):
        entry = raw_entries[index * entry_size : (index + 1) * entry_size]
        if not entry[:16].strip(b"\x00"):
            continue
        first_lba = struct.unpack_from("<Q", entry, 32)[0]
        last_lba = struct.unpack_from("<Q", entry, 40)[0]
        name = entry[56:128].decode("utf-16le", errors="ignore").rstrip("\x00")
        partitions.append(
            {
                "number": index + 1,
                "name": name,
                "first_lba": first_lba,
                "last_lba": last_lba,
                "size_bytes": (last_lba - first_lba + 1) * 512,
            }
        )
    return partitions


package_dir = os.environ["PACKAGE_DIR"]
manifest_path = os.path.join(package_dir, "manifest.json")

with open(manifest_path, "r", encoding="utf-8") as f:
    manifest = json.load(f)

required_top = [
    "schema_version",
    "package_kind",
    "git_commit",
    "package_name",
    "target",
    "source_image",
    "loader",
    "partition_table",
]
missing = [key for key in required_top if key not in manifest]
if missing:
    raise SystemExit(f"manifest missing fields: {', '.join(missing)}")

if manifest["package_kind"] != "lumelo-t4-usb-emmc-raw":
    raise SystemExit(f"unexpected package_kind: {manifest['package_kind']}")

target = manifest["target"]
expected_target = {
    "board": "NanoPC-T4",
    "soc": "RK3399",
    "storage": "eMMC",
    "host": "Win11",
    "tool": "RKDevTool",
    "mode": "MaskROM",
    "start_address": "0x0",
    "erase_all_default": False,
}
for key, expected in expected_target.items():
    if target.get(key) != expected:
        raise SystemExit(f"manifest target.{key}={target.get(key)!r}, expected {expected!r}")

source = manifest["source_image"]
loader = manifest["loader"]
image_path = os.path.join(package_dir, source["copied_as"])
loader_path = os.path.join(package_dir, loader["copied_as"])

if not os.path.isfile(image_path):
    raise SystemExit(f"packaged image missing: {image_path}")
if not os.path.isfile(loader_path):
    raise SystemExit(f"packaged loader missing: {loader_path}")

if sha256(image_path) != source["sha256"]:
    raise SystemExit("packaged image sha256 does not match manifest")
if os.path.getsize(image_path) != int(source["size_bytes"]):
    raise SystemExit("packaged image size does not match manifest")
if sha256(loader_path) != loader["sha256"]:
    raise SystemExit("packaged loader sha256 does not match manifest")
if os.path.getsize(loader_path) != int(loader["size_bytes"]):
    raise SystemExit("packaged loader size does not match manifest")

partitions = parse_gpt(image_path)
names = [part["name"] for part in partitions[: len(EXPECTED_NAMES)]]
if names != EXPECTED_NAMES:
    raise SystemExit(f"unexpected partition names: {names}, expected {EXPECTED_NAMES}")

manifest_partitions = manifest["partition_table"]["partitions"]
manifest_names = [part["name"] for part in manifest_partitions[: len(EXPECTED_NAMES)]]
if manifest_names != EXPECTED_NAMES:
    raise SystemExit(f"manifest partition names mismatch: {manifest_names}")

for actual, recorded in zip(partitions, manifest_partitions):
    for key in ("number", "name", "first_lba", "last_lba", "size_bytes"):
        if actual[key] != recorded[key]:
            raise SystemExit(f"manifest partition {actual['number']} field {key} mismatch")

source_path = source.get("path", "")
if source_path and os.path.exists(source_path):
    if sha256(source_path) != source["sha256"]:
        raise SystemExit(f"source image hash changed after packaging: {source_path}")
    print(f"PASS original source image hash unchanged: {source_path}")
else:
    print("INFO original source image path not present; source pollution check skipped")

print("PASS manifest fields validate")
print("PASS packaged raw image has expected p1-p9 layout")
PY

IMAGE_NAME=$(python3 - <<'PY'
import json
import os
with open(os.path.join(os.environ["PACKAGE_DIR"], "manifest.json"), "r", encoding="utf-8") as f:
    print(json.load(f)["source_image"]["copied_as"])
PY
)

if [ ! -x "${VERIFY_ROOTFS}" ]; then
  fail "rootfs verifier missing or not executable: ${VERIFY_ROOTFS}"
else
  "${VERIFY_ROOTFS}" "${PACKAGE_DIR}/${IMAGE_NAME}"
fi

if [ "${FAILURES}" -ne 0 ]; then
  printf 'Summary: %s failure(s)\n' "${FAILURES}"
  exit 1
fi

printf 'Summary: 0 failure(s)\n'
