#!/bin/sh
set -eu

PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"

usage() {
  cat <<'EOF'
Usage:
  package-t4-usb-emmc-official-layout.sh \
    --source-image out/t4-rootfs/lumelo-t4-rootfs-YYYYMMDD-vN.img \
    --reference-usb-dir /path/to/rk3399-usb-debian-trixie-core-4.19-arm64-YYYYMMDD \
    [--version vN] \
    [--output-root out/t4-usb-emmc-official-layout] \
    [--force]

Notes:
  - Creates a Win11 RKDevTool "Download Image" multi-partition package.
  - Does not connect to T4 and does not write eMMC.
  - Does not modify the source TF/raw image.
  - Requires a FriendlyELEC official USB package directory as the reference.
  - The reference supplies RKDevTool, config.cfg/config.ini, parameter.txt,
    MiniLoaderAll.bin, and Windows helper files.
EOF
}

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "package-t4-usb-emmc-official-layout.sh must run as root" >&2
    exit 1
  fi
}

require_linux() {
  if [ "$(uname -s)" != "Linux" ]; then
    echo "package-t4-usb-emmc-official-layout.sh must run on Linux" >&2
    exit 1
  fi
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
  sha256sum "$1" | awk '{print $1}'
}

file_size() {
  wc -c <"$1" | tr -d ' '
}

cleanup() {
  status=$?
  if [ "${KEEP_WORKDIR:-0}" != "1" ] && [ -n "${WORKDIR:-}" ] && [ -d "${WORKDIR}" ]; then
    rm -rf "${WORKDIR}"
  fi
  exit "${status}"
}

SOURCE_IMAGE=
REFERENCE_USB_DIR=
VERSION=
OUTPUT_ROOT=out/t4-usb-emmc-official-layout
FORCE=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --source-image)
      SOURCE_IMAGE=$2
      shift 2
      ;;
    --reference-usb-dir)
      REFERENCE_USB_DIR=$2
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

if [ -z "${SOURCE_IMAGE}" ] || [ -z "${REFERENCE_USB_DIR}" ]; then
  usage >&2
  exit 1
fi

require_linux
require_root
require_cmd awk
require_cmd basename
require_cmd cp
require_cmd dirname
require_cmd e2fsck
require_cmd find
require_cmd git
require_cmd mkfs.ext4
require_cmd mktemp
require_cmd python3
require_cmd resize2fs
require_cmd rm
require_cmd sha256sum
require_cmd sort
require_cmd truncate
require_cmd wc

SOURCE_IMAGE_ABS=$(absolute_path "${SOURCE_IMAGE}")
REFERENCE_USB_DIR_ABS=$(absolute_path "${REFERENCE_USB_DIR}")
REPO_ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd -P)
VERIFY_ROOTFS="${REPO_ROOT}/scripts/verify-t4-lumelo-rootfs-image.sh"

source_basename=$(basename "${SOURCE_IMAGE_ABS}")
reference_basename=$(basename "${REFERENCE_USB_DIR_ABS}")

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

for path in \
  RKDevTool.exe \
  MiniLoaderAll.bin \
  config.cfg \
  config.ini \
  parameter.txt \
  bin \
  Language \
  doc; do
  if [ ! -e "${REFERENCE_USB_DIR_ABS}/${path}" ]; then
    echo "reference USB package missing: ${path}" >&2
    exit 1
  fi
done

if [ ! -x "${VERIFY_ROOTFS}" ]; then
  echo "rootfs verifier missing or not executable: ${VERIFY_ROOTFS}" >&2
  exit 1
fi

source_sha_before=$(sha256_file "${SOURCE_IMAGE_ABS}")
source_size=$(file_size "${SOURCE_IMAGE_ABS}")
reference_loader_sha=$(sha256_file "${REFERENCE_USB_DIR_ABS}/MiniLoaderAll.bin")
reference_config_sha=$(sha256_file "${REFERENCE_USB_DIR_ABS}/config.cfg")
reference_parameter_sha=$(sha256_file "${REFERENCE_USB_DIR_ABS}/parameter.txt")
git_commit=$(git -C "${REPO_ROOT}" rev-parse HEAD)

case "${OUTPUT_ROOT}" in
  /*) OUTPUT_ROOT_PATH=${OUTPUT_ROOT} ;;
  *) OUTPUT_ROOT_PATH="${REPO_ROOT}/${OUTPUT_ROOT}" ;;
esac
OUTPUT_ROOT_ABS=$(mkdir -p "${OUTPUT_ROOT_PATH}" && cd "${OUTPUT_ROOT_PATH}" && pwd -P)
PACKAGE_NAME="lumelo-t4-usb-emmc-official-layout-${image_date}-${VERSION}"
PACKAGE_DIR="${OUTPUT_ROOT_ABS}/${PACKAGE_NAME}"

if [ -e "${PACKAGE_DIR}" ]; then
  if [ "${FORCE}" != "1" ]; then
    echo "package directory already exists: ${PACKAGE_DIR}" >&2
    echo "rerun with --force to replace it" >&2
    exit 1
  fi
  rm -rf "${PACKAGE_DIR}"
fi

WORKDIR=$(mktemp -d "${TMPDIR:-/tmp}/lumelo-usb-emmc-official.XXXXXX")
trap cleanup EXIT INT TERM

echo "==> verifying source rootfs image"
"${VERIFY_ROOTFS}" "${SOURCE_IMAGE_ABS}"

echo "==> preparing package scaffold"
mkdir -p "${PACKAGE_DIR}"
cp -pR "${REFERENCE_USB_DIR_ABS}/bin" "${PACKAGE_DIR}/bin"
cp -pR "${REFERENCE_USB_DIR_ABS}/Language" "${PACKAGE_DIR}/Language"
cp -pR "${REFERENCE_USB_DIR_ABS}/doc" "${PACKAGE_DIR}/doc"
for file in RKDevTool.exe MiniLoaderAll.bin config.cfg config.ini parameter.txt; do
  cp -p "${REFERENCE_USB_DIR_ABS}/${file}" "${PACKAGE_DIR}/${file}"
done
if [ -f "${REFERENCE_USB_DIR_ABS}/idbloader.img" ]; then
  cp -p "${REFERENCE_USB_DIR_ABS}/idbloader.img" "${PACKAGE_DIR}/idbloader.img"
fi

cat >"${PACKAGE_DIR}/info.conf" <<EOF
title=Lumelo T4
require-board=rk3399
version=${image_date}-${VERSION}
EOF

export SOURCE_IMAGE_ABS
export PACKAGE_DIR
export WORKDIR
echo "==> extracting source partitions"
python3 - <<'PY'
import json
import os
import struct

SECTOR = 512
EXPECTED = {
    1: ("uboot", 0x00004000, 0x00002000),
    2: ("trust", 0x00006000, 0x00002000),
    3: ("misc", 0x00008000, 0x00002000),
    4: ("dtbo", 0x0000A000, 0x00002000),
    5: ("resource", 0x0000C000, 0x00008000),
    6: ("kernel", 0x00014000, 0x00014000),
    7: ("boot", 0x00028000, 0x00018000),
}
ROOTFS_START = 0x00040000
ROOTFS_TARGET_SECTORS = 0x00400000


def parse_gpt(path):
    with open(path, "rb") as f:
        f.seek(SECTOR)
        header = f.read(92)
        if header[:8] != b"EFI PART":
            raise SystemExit(f"missing GPT header: {path}")
        entries_lba = struct.unpack_from("<Q", header, 72)[0]
        entry_count = struct.unpack_from("<I", header, 80)[0]
        entry_size = struct.unpack_from("<I", header, 84)[0]
        f.seek(entries_lba * SECTOR)
        raw_entries = f.read(entry_count * entry_size)

    partitions = {}
    for index in range(entry_count):
        entry = raw_entries[index * entry_size : (index + 1) * entry_size]
        if not entry[:16].strip(b"\x00"):
            continue
        first_lba = struct.unpack_from("<Q", entry, 32)[0]
        last_lba = struct.unpack_from("<Q", entry, 40)[0]
        name = entry[56:128].decode("utf-16le", errors="ignore").rstrip("\x00")
        number = index + 1
        partitions[number] = {
            "number": number,
            "name": name,
            "first_lba": first_lba,
            "last_lba": last_lba,
            "sector_count": last_lba - first_lba + 1,
            "size_bytes": (last_lba - first_lba + 1) * SECTOR,
        }
    return partitions


def copy_range(src, dst, offset, size):
    remaining = size
    with open(src, "rb") as inf, open(dst, "wb") as outf:
        inf.seek(offset)
        while remaining:
            chunk = inf.read(min(1024 * 1024, remaining))
            if not chunk:
                raise SystemExit(f"unexpected EOF while extracting {dst}")
            outf.write(chunk)
            remaining -= len(chunk)


source = os.environ["SOURCE_IMAGE_ABS"]
package_dir = os.environ["PACKAGE_DIR"]
workdir = os.environ["WORKDIR"]
parts = parse_gpt(source)

for number, (name, start, sectors) in EXPECTED.items():
    part = parts.get(number)
    if part is None:
        raise SystemExit(f"missing partition p{number}: {name}")
    if part["name"] != name:
        raise SystemExit(f"p{number} name {part['name']!r} != {name!r}")
    if part["first_lba"] != start:
        raise SystemExit(f"p{number} start 0x{part['first_lba']:08x} != 0x{start:08x}")
    if part["sector_count"] != sectors:
        raise SystemExit(
            f"p{number} sectors 0x{part['sector_count']:08x} != 0x{sectors:08x}"
        )
    copy_range(
        source,
        os.path.join(package_dir, f"{name}.img"),
        part["first_lba"] * SECTOR,
        part["size_bytes"],
    )

rootfs = parts.get(8)
if rootfs is None or rootfs["name"] != "rootfs":
    raise SystemExit("missing p8 rootfs partition")
if rootfs["first_lba"] != ROOTFS_START:
    raise SystemExit(
        f"rootfs start 0x{rootfs['first_lba']:08x} != 0x{ROOTFS_START:08x}"
    )
if rootfs["sector_count"] > ROOTFS_TARGET_SECTORS:
    raise SystemExit(
        f"rootfs is larger than official 2 GiB slot: 0x{rootfs['sector_count']:08x}"
    )

copy_range(
    source,
    os.path.join(workdir, "rootfs.raw"),
    rootfs["first_lba"] * SECTOR,
    rootfs["size_bytes"],
)

layout = {
    "sector_size": SECTOR,
    "partitions": [parts[i] for i in sorted(parts)],
    "official_rootfs_target": {
        "first_lba": ROOTFS_START,
        "sector_count": ROOTFS_TARGET_SECTORS,
        "size_bytes": ROOTFS_TARGET_SECTORS * SECTOR,
    },
}
with open(os.path.join(workdir, "source-layout.json"), "w", encoding="utf-8") as f:
    json.dump(layout, f, indent=2)
    f.write("\n")
PY

ROOTFS_RAW="${WORKDIR}/rootfs.raw"
USERDATA_RAW="${WORKDIR}/userdata.raw"
ROOTFS_SPARSE="${PACKAGE_DIR}/rootfs.img"
USERDATA_SPARSE="${PACKAGE_DIR}/userdata.img"

echo "==> resizing rootfs partition image to official 2 GiB slot"
e2fsck -fy "${ROOTFS_RAW}" >/dev/null
resize2fs "${ROOTFS_RAW}" 2G >/dev/null
e2fsck -fy "${ROOTFS_RAW}" >/dev/null
rootfs_raw_size=$(file_size "${ROOTFS_RAW}")
if [ "${rootfs_raw_size}" -ne 2147483648 ]; then
  echo "resized rootfs.raw size ${rootfs_raw_size} != 2147483648" >&2
  exit 1
fi

echo "==> creating 200 MiB userdata image"
truncate -s 200M "${USERDATA_RAW}"
mkfs.ext4 -F -L userdata -m 0 "${USERDATA_RAW}" >/dev/null
e2fsck -fy "${USERDATA_RAW}" >/dev/null

export ROOTFS_RAW
export USERDATA_RAW
export ROOTFS_SPARSE
export USERDATA_SPARSE
echo "==> converting ext4 partition images to Android sparse images"
python3 - <<'PY'
import os
import struct

SPARSE_MAGIC = 0xED26FF3A
MAJOR = 1
MINOR = 0
FILE_HDR_SZ = 28
CHUNK_HDR_SZ = 12
BLK_SZ = 4096
CHUNK_TYPE_RAW = 0xCAC1
CHUNK_TYPE_DONT_CARE = 0xCAC3
READ_BLOCKS = 1024


def raw_to_sparse(raw_path, sparse_path):
    size = os.path.getsize(raw_path)
    if size % BLK_SZ:
        raise SystemExit(f"{raw_path} size is not {BLK_SZ}-byte aligned")
    total_blocks = size // BLK_SZ
    chunk_count = 0
    with open(raw_path, "rb") as inf, open(sparse_path, "wb+") as outf:
        outf.write(b"\x00" * FILE_HDR_SZ)
        remaining_blocks = total_blocks
        while remaining_blocks:
            blocks = min(READ_BLOCKS, remaining_blocks)
            data = inf.read(blocks * BLK_SZ)
            if len(data) != blocks * BLK_SZ:
                raise SystemExit(f"short read from {raw_path}")
            if any(data):
                outf.write(struct.pack("<HHII", CHUNK_TYPE_RAW, 0, blocks, CHUNK_HDR_SZ + len(data)))
                outf.write(data)
            else:
                outf.write(struct.pack("<HHII", CHUNK_TYPE_DONT_CARE, 0, blocks, CHUNK_HDR_SZ))
            chunk_count += 1
            remaining_blocks -= blocks

        outf.seek(0)
        outf.write(
            struct.pack(
                "<IHHHHIIII",
                SPARSE_MAGIC,
                MAJOR,
                MINOR,
                FILE_HDR_SZ,
                CHUNK_HDR_SZ,
                BLK_SZ,
                total_blocks,
                chunk_count,
                0,
            )
        )


raw_to_sparse(os.environ["ROOTFS_RAW"], os.environ["ROOTFS_SPARSE"])
raw_to_sparse(os.environ["USERDATA_RAW"], os.environ["USERDATA_SPARSE"])
PY

echo "==> writing README and manifest"
cat >"${PACKAGE_DIR}/README-WIN11-RKDEVTOOL.md" <<EOF
# Lumelo T4 USB-to-eMMC Package (${image_date}-${VERSION})

This package uses the FriendlyELEC/RKDevTool multi-partition layout.

Use RKDevTool's "Download Image" tab. Do not use "Upgrade Firmware" for this
package.

## Package Contents

- RKDevTool.exe
- MiniLoaderAll.bin
- config.cfg
- config.ini
- parameter.txt
- uboot.img
- trust.img
- misc.img
- dtbo.img
- resource.img
- kernel.img
- boot.img
- rootfs.img
- userdata.img
- manifest.json
- SHA256SUMS.txt

## Win11 Flash Steps

1. Install the Rockchip USB driver / DriverAssistant.
2. Extract this package on the Win11 PC.
3. Power off NanoPC-T4 and remove the TF card.
4. Hold the MASK key, connect the Type-C data cable, wait about 3 seconds, then
   release the key.
5. Run the included RKDevTool.exe.
6. Confirm the status shows "Found One MASKROM Device".
7. Use the "Download Image" tab.
8. Keep "Write by Address" checked.
9. Confirm the table matches:

| Name | Address | File |
| --- | --- | --- |
| Parameter | 0x00000000 | parameter.txt |
| Uboot | 0x00004000 | uboot.img |
| Trust | 0x00006000 | trust.img |
| Misc | 0x00008000 | misc.img |
| Dtbo | 0x0000A000 | dtbo.img |
| Resource | 0x0000C000 | resource.img |
| Kernel | 0x00014000 | kernel.img |
| Boot | 0x00028000 | boot.img |
| Rootfs | 0x00040000 | rootfs.img |
| Userdata | 0x00440000 | userdata.img |

10. Click "Run".
11. After RKDevTool reports success, power off, disconnect USB, keep the TF card
    removed, and cold boot from eMMC.

## Erase Policy

Do not use EraseAll by default.

Use EraseAll only when crossing from a different system, after a failed flash
that cannot boot, or during recovery/unbrick work.

## Validation After Boot

- SSH is reachable.
- WebUI opens at http://<T4_IP>/.
- lumelo.local is treated as an enhanced mDNS entry, not the only access path.
- USB DAC auto-select works.
- Local media scan works.
- lumelo-media-smoke play --first-wav passes.
EOF

export PACKAGE_NAME
export SOURCE_BASENAME="${source_basename}"
export SOURCE_SHA="${source_sha_before}"
export SOURCE_SIZE="${source_size}"
export REFERENCE_USB_DIR_ABS
export REFERENCE_BASENAME="${reference_basename}"
export REFERENCE_LOADER_SHA="${reference_loader_sha}"
export REFERENCE_CONFIG_SHA="${reference_config_sha}"
export REFERENCE_PARAMETER_SHA="${reference_parameter_sha}"
export GIT_COMMIT="${git_commit}"
export IMAGE_DATE="${image_date}"
export IMAGE_VERSION="${VERSION}"
python3 - <<'PY'
import hashlib
import json
import os
import struct
from datetime import datetime, timezone

SPARSE_MAGIC = 0xED26FF3A


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def inspect_sparse(path):
    with open(path, "rb") as f:
        header = f.read(28)
        if len(header) != 28:
            raise SystemExit(f"sparse header too short: {path}")
        (
            magic,
            major,
            minor,
            file_hdr_sz,
            chunk_hdr_sz,
            blk_sz,
            total_blks,
            total_chunks,
            checksum,
        ) = struct.unpack("<IHHHHIIII", header)
        if magic != SPARSE_MAGIC:
            raise SystemExit(f"not an Android sparse image: {path}")
        if file_hdr_sz != 28 or chunk_hdr_sz != 12:
            raise SystemExit(f"unexpected sparse header size: {path}")
        return {
            "sha256": sha256(path),
            "size_bytes": os.path.getsize(path),
            "sparse": {
                "major": major,
                "minor": minor,
                "block_size": blk_sz,
                "total_blocks": total_blks,
                "total_chunks": total_chunks,
                "expanded_size_bytes": blk_sz * total_blks,
                "checksum": checksum,
            },
        }


package_dir = os.environ["PACKAGE_DIR"]
workdir = os.environ["WORKDIR"]
with open(os.path.join(workdir, "source-layout.json"), "r", encoding="utf-8") as f:
    source_layout = json.load(f)

partition_images = {}
for name in ("uboot", "trust", "misc", "dtbo", "resource", "kernel", "boot"):
    path = os.path.join(package_dir, f"{name}.img")
    partition_images[name] = {
        "filename": f"{name}.img",
        "sha256": sha256(path),
        "size_bytes": os.path.getsize(path),
    }

partition_images["rootfs"] = {
    "filename": "rootfs.img",
    **inspect_sparse(os.path.join(package_dir, "rootfs.img")),
}
partition_images["userdata"] = {
    "filename": "userdata.img",
    **inspect_sparse(os.path.join(package_dir, "userdata.img")),
}

manifest = {
    "schema_version": 1,
    "package_kind": "lumelo-t4-usb-emmc-official-layout",
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
        "rkdevtool_tab": "Download Image",
        "write_by_address": True,
        "erase_all_default": False,
    },
    "source_image": {
        "path": os.environ["SOURCE_IMAGE_ABS"],
        "filename": os.environ["SOURCE_BASENAME"],
        "sha256": os.environ["SOURCE_SHA"],
        "size_bytes": int(os.environ["SOURCE_SIZE"]),
    },
    "reference_usb_package": {
        "path": os.environ["REFERENCE_USB_DIR_ABS"],
        "basename": os.environ["REFERENCE_BASENAME"],
        "loader_sha256": os.environ["REFERENCE_LOADER_SHA"],
        "config_cfg_sha256": os.environ["REFERENCE_CONFIG_SHA"],
        "parameter_txt_sha256": os.environ["REFERENCE_PARAMETER_SHA"],
    },
    "official_layout": {
        "sector_size": 512,
        "partitions": [
            {"name": "uboot", "address": "0x00004000", "sectors": "0x00002000"},
            {"name": "trust", "address": "0x00006000", "sectors": "0x00002000"},
            {"name": "misc", "address": "0x00008000", "sectors": "0x00002000"},
            {"name": "dtbo", "address": "0x0000A000", "sectors": "0x00002000"},
            {"name": "resource", "address": "0x0000C000", "sectors": "0x00008000"},
            {"name": "kernel", "address": "0x00014000", "sectors": "0x00014000"},
            {"name": "boot", "address": "0x00028000", "sectors": "0x00018000"},
            {"name": "rootfs", "address": "0x00040000", "sectors": "0x00400000"},
            {"name": "userdata", "address": "0x00440000", "sectors": "grow"},
        ],
    },
    "source_layout": source_layout,
    "partition_images": partition_images,
}

with open(os.path.join(package_dir, "manifest.json"), "w", encoding="utf-8") as f:
    json.dump(manifest, f, ensure_ascii=False, indent=2)
    f.write("\n")
PY

(
  cd "${PACKAGE_DIR}"
  find . -type f ! -name SHA256SUMS.txt -print |
    sort |
    while IFS= read -r file; do
      clean=${file#./}
      sha256sum "${clean}"
    done >SHA256SUMS.txt
)

source_sha_after=$(sha256_file "${SOURCE_IMAGE_ABS}")
if [ "${source_sha_after}" != "${source_sha_before}" ]; then
  echo "source image changed while packaging: ${SOURCE_IMAGE_ABS}" >&2
  exit 1
fi

echo "package created: ${PACKAGE_DIR}"
echo "source image sha256: ${source_sha_before}"
echo "reference loader sha256: ${reference_loader_sha}"
