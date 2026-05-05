#!/bin/sh
set -eu

PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"

usage() {
  cat <<'EOF'
Usage:
  package-t4-usb-emmc-raw.sh \
    --source-image out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img \
    --loader /path/to/MiniLoaderAll.bin \
    [--version vN] \
    [--output-root out/t4-usb-emmc-raw] \
    --experimental-raw-wl0 \
    [--force]

Notes:
  - Experimental only: packages a raw full-disk image for a future
    Linux rkdeveloptool wl 0 path.
  - This is NOT the Win11 RKDevTool Download Image package.
  - Does not modify the source image or the existing TF/raw image build chain.
  - MiniLoaderAll.bin must be supplied explicitly from a trusted official source.
  - The Win11 path must use an official-layout multi-partition package.
EOF
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

absolute_path() {
  path=$1
  dir=$(dirname "${path}")
  base=$(basename "${path}")
  if [ ! -e "${path}" ]; then
    echo "path does not exist: ${path}" >&2
    exit 1
  fi
  printf '%s/%s\n' "$(cd "${dir}" && pwd -P)" "${base}"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
    return
  fi
  echo "missing required command: sha256sum or shasum" >&2
  exit 1
}

file_size() {
  wc -c <"$1" | tr -d ' '
}

SOURCE_IMAGE=
LOADER=
VERSION=
OUTPUT_ROOT=out/t4-usb-emmc-raw
FORCE=0
EXPERIMENTAL_RAW_WL0=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --source-image)
      SOURCE_IMAGE=$2
      shift 2
      ;;
    --loader)
      LOADER=$2
      shift 2
      ;;
    --version)
      VERSION=$2
      shift 2
      ;;
    --output-root)
      OUTPUT_ROOT=$2
      shift 2
      ;;
    --experimental-raw-wl0)
      EXPERIMENTAL_RAW_WL0=1
      shift
      ;;
    --force)
      FORCE=1
      shift
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

if [ -z "${SOURCE_IMAGE}" ] || [ -z "${LOADER}" ]; then
  usage >&2
  exit 1
fi

if [ "${EXPERIMENTAL_RAW_WL0}" -ne 1 ]; then
  echo "refusing to create a raw full-disk package without --experimental-raw-wl0" >&2
  echo "Win11 RKDevTool must use the official-layout multi-partition package instead." >&2
  exit 1
fi

require_cmd awk
require_cmd basename
require_cmd cp
require_cmd dirname
require_cmd git
require_cmd mkdir
require_cmd python3
require_cmd sed
require_cmd tr
require_cmd wc

SOURCE_IMAGE_ABS=$(absolute_path "${SOURCE_IMAGE}")
LOADER_ABS=$(absolute_path "${LOADER}")
REPO_ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd -P)

source_basename=$(basename "${SOURCE_IMAGE_ABS}")
loader_basename=$(basename "${LOADER_ABS}")

tag=$(printf '%s\n' "${source_basename}" |
  sed -n 's/^lumelo-t4-rootfs-\([0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-v[0-9][0-9]*\)\.img$/\1/p')
if [ -z "${tag}" ]; then
  echo "source image name must match lumelo-t4-rootfs-YYYYMMDD-vN.img: ${source_basename}" >&2
  exit 1
fi

image_date=${tag%-v*}
image_version="v${tag##*-v}"
if [ -n "${VERSION}" ] && [ "${VERSION}" != "${image_version}" ]; then
  echo "requested version ${VERSION} does not match source image version ${image_version}" >&2
  exit 1
fi
VERSION=${image_version}

if [ "${loader_basename}" != "MiniLoaderAll.bin" ]; then
  echo "loader file must be named MiniLoaderAll.bin: ${loader_basename}" >&2
  exit 1
fi

loader_size=$(file_size "${LOADER_ABS}")
if [ "${loader_size}" -lt 4096 ]; then
  echo "loader file is unexpectedly small: ${loader_size} bytes" >&2
  exit 1
fi

source_sha_before=$(sha256_file "${SOURCE_IMAGE_ABS}")
source_size=$(file_size "${SOURCE_IMAGE_ABS}")
loader_sha=$(sha256_file "${LOADER_ABS}")
git_commit=$(git -C "${REPO_ROOT}" rev-parse HEAD)

case "${OUTPUT_ROOT}" in
  /*) OUTPUT_ROOT_PATH=${OUTPUT_ROOT} ;;
  *) OUTPUT_ROOT_PATH="${REPO_ROOT}/${OUTPUT_ROOT}" ;;
esac
OUTPUT_ROOT_ABS=$(mkdir -p "${OUTPUT_ROOT_PATH}" && cd "${OUTPUT_ROOT_PATH}" && pwd -P)
PACKAGE_NAME="lumelo-t4-usb-emmc-raw-${image_date}-${VERSION}"
PACKAGE_DIR="${OUTPUT_ROOT_ABS}/${PACKAGE_NAME}"

if [ -e "${PACKAGE_DIR}" ]; then
  if [ "${FORCE}" != "1" ]; then
    echo "package directory already exists: ${PACKAGE_DIR}" >&2
    echo "rerun with --force to replace it" >&2
    exit 1
  fi
  require_cmd rm
  rm -rf "${PACKAGE_DIR}"
fi

mkdir -p "${PACKAGE_DIR}"
cp -p "${SOURCE_IMAGE_ABS}" "${PACKAGE_DIR}/${source_basename}"
cp -p "${LOADER_ABS}" "${PACKAGE_DIR}/MiniLoaderAll.bin"

export SOURCE_IMAGE_ABS
export LOADER_ABS
export PACKAGE_DIR
export PACKAGE_NAME
export SOURCE_BASENAME="${source_basename}"
export SOURCE_SHA="${source_sha_before}"
export SOURCE_SIZE="${source_size}"
export LOADER_ORIGINAL_BASENAME="${loader_basename}"
export LOADER_SHA="${loader_sha}"
export LOADER_SIZE="${loader_size}"
export GIT_COMMIT="${git_commit}"
export IMAGE_DATE="${image_date}"
export IMAGE_VERSION="${VERSION}"

python3 - <<'PY'
import json
import os
import struct
from datetime import datetime, timezone


def parse_gpt(image_path):
    with open(image_path, "rb") as f:
        f.seek(512)
        header = f.read(92)
        if header[:8] != b"EFI PART":
            raise SystemExit(f"image does not contain a GPT header: {image_path}")
        current_lba = struct.unpack_from("<Q", header, 24)[0]
        first_usable_lba = struct.unpack_from("<Q", header, 40)[0]
        last_usable_lba = struct.unpack_from("<Q", header, 48)[0]
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

    return {
        "current_lba": current_lba,
        "first_usable_lba": first_usable_lba,
        "last_usable_lba": last_usable_lba,
        "partitions": partitions,
    }


source_image_abs = os.environ["SOURCE_IMAGE_ABS"]
package_dir = os.environ["PACKAGE_DIR"]
source_basename = os.environ["SOURCE_BASENAME"]

partition_table = parse_gpt(source_image_abs)
manifest = {
    "schema_version": 1,
    "package_kind": "lumelo-t4-usb-emmc-raw",
    "status": "mvp",
    "generated_at_utc": datetime.now(timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace("+00:00", "Z"),
    "git_commit": os.environ["GIT_COMMIT"],
    "package_name": os.environ["PACKAGE_NAME"],
    "target": {
        "board": "NanoPC-T4",
        "soc": "RK3399",
        "storage": "eMMC",
        "host": "Win11",
        "tool": "RKDevTool",
        "mode": "MaskROM",
        "write_method": "raw image, write by address",
        "start_address": "0x0",
        "erase_all_default": False,
    },
    "source_image": {
        "path": source_image_abs,
        "filename": source_basename,
        "copied_as": source_basename,
        "sha256": os.environ["SOURCE_SHA"],
        "size_bytes": int(os.environ["SOURCE_SIZE"]),
    },
    "loader": {
        "path": os.environ["LOADER_ABS"],
        "filename": os.environ["LOADER_ORIGINAL_BASENAME"],
        "copied_as": "MiniLoaderAll.bin",
        "sha256": os.environ["LOADER_SHA"],
        "size_bytes": int(os.environ["LOADER_SIZE"]),
        "source_policy": "caller_supplied_trusted_official_artifact",
    },
    "output": {
        "package_dir": package_dir,
        "image_date": os.environ["IMAGE_DATE"],
        "version": os.environ["IMAGE_VERSION"],
    },
    "partition_table": partition_table,
}

with open(os.path.join(package_dir, "manifest.json"), "w", encoding="utf-8") as f:
    json.dump(manifest, f, ensure_ascii=False, indent=2)
    f.write("\n")

with open(os.path.join(package_dir, "flash-layout-notes.txt"), "w", encoding="utf-8") as f:
    f.write("Lumelo T4 USB-to-eMMC raw image MVP flash layout\n")
    f.write("=================================================\n\n")
    f.write("RKDevTool path: Download Image / raw image write by address\n")
    f.write("Target storage: eMMC\n")
    f.write("Start address: 0x0\n")
    f.write("Default erase policy: do not EraseAll unless recovery requires it\n\n")
    f.write("Raw disk image partition table:\n")
    for part in partition_table["partitions"]:
        f.write(
            "{number:02d} {name:<10} first_lba={first_lba:<10} "
            "last_lba={last_lba:<10} size_bytes={size_bytes}\n".format(**part)
        )
PY

cat >"${PACKAGE_DIR}/README-WIN11-RKDEVTOOL.md" <<EOF
# Lumelo T4 USB-to-eMMC Raw Image MVP (${image_date}-${VERSION})

This package is an MVP for flashing a Lumelo raw disk image to NanoPC-T4 eMMC
from Win11 using Rockchip RKDevTool. It is not a final Rockchip update.img or a
FriendlyELEC multi-partition USB upgrade package.

## Package Contents

- \`${source_basename}\`
- \`MiniLoaderAll.bin\`
- \`manifest.json\`
- \`SHA256SUMS.txt\`
- \`flash-layout-notes.txt\`

## Required Host Tools

- Win11 PC
- Rockchip USB driver / DriverAssistant
- RKDevTool
- USB Type-C data cable
- NanoPC-T4 12V/2A DC power supply

## Board Mode

1. Power off the NanoPC-T4.
2. Remove the TF card and unnecessary USB devices.
3. Hold the \`MASK\` / \`BOOT\` key.
4. Connect the NanoPC-T4 Type-C port to the Win11 PC with a data cable.
5. Keep holding the key until the status LED has been on for about 3 seconds,
   then release it.
6. RKDevTool should show \`Found One MASKROM Device\`.

## RKDevTool Flash Steps

1. Install DriverAssistant and open RKDevTool.
2. Confirm RKDevTool shows \`Found One MASKROM Device\`.
3. Select \`MiniLoaderAll.bin\` as the loader.
4. Select \`${source_basename}\` as the raw system image.
5. Select target storage \`eMMC\`.
6. Enable \`Write by Address\`.
7. Set the start address to \`0x0\`.
8. Click \`Run\`.
9. When flashing finishes, power off the board.
10. Disconnect USB, keep the TF card removed, then cold boot from eMMC.

## EraseAll Policy

Do not use \`EraseAll\` by default.

Use \`EraseAll\` only when the existing eMMC contains a different system, the
first flash does not boot, or the board is in recovery/unbrick flow.

## Verification After Boot

- Confirm the board boots without a TF card.
- Confirm Lumelo WebUI opens at \`http://<T4_IP>/\`.
- Confirm \`http://lumelo.local/\` only as an enhanced mDNS entry, not the sole
  access path.
- Run the T4 bring-up checklist before treating this package as board-verified.

## Checksums

Run checksum verification before flashing:

\`\`\`powershell
Get-FileHash ${source_basename} -Algorithm SHA256
Get-FileHash MiniLoaderAll.bin -Algorithm SHA256
\`\`\`

Compare against \`SHA256SUMS.txt\`.
EOF

(
  cd "${PACKAGE_DIR}"
  {
    sha256_file "${source_basename}" | awk -v f="${source_basename}" '{print $1 "  " f}'
    sha256_file "MiniLoaderAll.bin" | awk '{print $1 "  MiniLoaderAll.bin"}'
    sha256_file "manifest.json" | awk '{print $1 "  manifest.json"}'
    sha256_file "README-WIN11-RKDEVTOOL.md" | awk '{print $1 "  README-WIN11-RKDEVTOOL.md"}'
    sha256_file "flash-layout-notes.txt" | awk '{print $1 "  flash-layout-notes.txt"}'
  } >SHA256SUMS.txt
)

source_sha_after=$(sha256_file "${SOURCE_IMAGE_ABS}")
if [ "${source_sha_after}" != "${source_sha_before}" ]; then
  echo "source image changed while packaging: ${SOURCE_IMAGE_ABS}" >&2
  exit 1
fi

echo "package created: ${PACKAGE_DIR}"
echo "source image sha256: ${source_sha_before}"
echo "loader sha256: ${loader_sha}"
