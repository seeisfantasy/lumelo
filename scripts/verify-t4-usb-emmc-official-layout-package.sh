#!/bin/sh
set -eu

PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"

usage() {
  cat <<'EOF'
Usage:
  verify-t4-usb-emmc-official-layout-package.sh \
    /path/to/lumelo-t4-usb-emmc-official-layout-YYYYMMDD-vN

Notes:
  - Runs offline and does not connect to T4 or write eMMC.
  - Verifies the Win11 RKDevTool "Download Image" multi-partition package.
  - Checks hashes, required files, parameter.txt layout, and Android sparse
    rootfs/userdata headers.
EOF
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

require_cmd python3
require_cmd sha256sum

FAILURES=0

for file in \
  RKDevTool.exe \
  MiniLoaderAll.bin \
  config.cfg \
  config.ini \
  parameter.txt \
  info.conf \
  uboot.img \
  trust.img \
  misc.img \
  dtbo.img \
  resource.img \
  kernel.img \
  boot.img \
  rootfs.img \
  userdata.img \
  manifest.json \
  SHA256SUMS.txt \
  README-WIN11-RKDEVTOOL.md; do
  expect_file "${PACKAGE_DIR}/${file}" "${file}"
done

if [ -d "${PACKAGE_DIR}/bin" ]; then
  pass "bin directory: ${PACKAGE_DIR}/bin"
else
  fail "bin directory missing"
fi
if [ -d "${PACKAGE_DIR}/Language" ]; then
  pass "Language directory: ${PACKAGE_DIR}/Language"
else
  fail "Language directory missing"
fi
if [ -d "${PACKAGE_DIR}/doc" ]; then
  pass "doc directory: ${PACKAGE_DIR}/doc"
else
  fail "doc directory missing"
fi

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

SPARSE_MAGIC = 0xED26FF3A
EXPECTED_PARAMETER = (
    "CMDLINE: mtdparts=rk29xxnand:"
    "0x00002000@0x00004000(uboot),"
    "0x00002000@0x00006000(trust),"
    "0x00002000@0x00008000(misc),"
    "0x00002000@0x0000a000(dtbo),"
    "0x00008000@0x0000c000(resource),"
    "0x00014000@0x00014000(kernel),"
    "0x00018000@0x00028000(boot),"
    "0x00400000@0x00040000(rootfs),"
    "-@0x00440000(userdata:grow)"
)
EXPECTED_SIZES = {
    "uboot": 0x00002000 * 512,
    "trust": 0x00002000 * 512,
    "misc": 0x00002000 * 512,
    "dtbo": 0x00002000 * 512,
    "resource": 0x00008000 * 512,
    "kernel": 0x00014000 * 512,
    "boot": 0x00018000 * 512,
}


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
        if major != 1 or minor != 0:
            raise SystemExit(f"unexpected sparse version in {path}: {major}.{minor}")
        if file_hdr_sz != 28 or chunk_hdr_sz != 12:
            raise SystemExit(f"unexpected sparse header size in {path}")
        f.seek(file_hdr_sz)
        blocks_seen = 0
        for _ in range(total_chunks):
            chunk = f.read(chunk_hdr_sz)
            if len(chunk) != chunk_hdr_sz:
                raise SystemExit(f"truncated sparse chunk header in {path}")
            chunk_type, _reserved, chunk_blocks, total_size = struct.unpack("<HHII", chunk)
            if chunk_type == 0xCAC1:
                expected_total = chunk_hdr_sz + chunk_blocks * blk_sz
                if total_size != expected_total:
                    raise SystemExit(f"bad RAW chunk size in {path}")
                f.seek(chunk_blocks * blk_sz, os.SEEK_CUR)
            elif chunk_type == 0xCAC3:
                if total_size != chunk_hdr_sz:
                    raise SystemExit(f"bad DONT_CARE chunk size in {path}")
            else:
                raise SystemExit(f"unsupported sparse chunk type 0x{chunk_type:04x} in {path}")
            blocks_seen += chunk_blocks
        if blocks_seen != total_blks:
            raise SystemExit(f"sparse block count mismatch in {path}")
        return {
            "block_size": blk_sz,
            "total_blocks": total_blks,
            "total_chunks": total_chunks,
            "expanded_size_bytes": blk_sz * total_blks,
            "checksum": checksum,
        }


package_dir = os.environ["PACKAGE_DIR"]
manifest_path = os.path.join(package_dir, "manifest.json")
with open(manifest_path, "r", encoding="utf-8") as f:
    manifest = json.load(f)

if manifest.get("package_kind") != "lumelo-t4-usb-emmc-official-layout":
    raise SystemExit(f"unexpected package_kind: {manifest.get('package_kind')!r}")

target = manifest.get("target", {})
expected_target = {
    "board": "NanoPC-T4",
    "soc": "RK3399",
    "storage": "eMMC",
    "host": "Win11",
    "tool": "RKDevTool",
    "mode": "MaskROM",
    "rkdevtool_tab": "Download Image",
    "write_by_address": True,
    "erase_all_default": False,
}
for key, expected in expected_target.items():
    if target.get(key) != expected:
        raise SystemExit(f"manifest target.{key}={target.get(key)!r}, expected {expected!r}")

with open(os.path.join(package_dir, "parameter.txt"), "r", encoding="utf-8") as f:
    parameter = f.read()
if EXPECTED_PARAMETER not in parameter:
    raise SystemExit("parameter.txt does not match expected FriendlyELEC RK3399 layout")

partition_images = manifest.get("partition_images", {})
for name, expected_size in EXPECTED_SIZES.items():
    path = os.path.join(package_dir, f"{name}.img")
    actual_size = os.path.getsize(path)
    if actual_size != expected_size:
        raise SystemExit(f"{name}.img size {actual_size} != {expected_size}")
    recorded = partition_images.get(name, {})
    if recorded.get("sha256") != sha256(path):
        raise SystemExit(f"{name}.img sha256 does not match manifest")
    if recorded.get("size_bytes") != actual_size:
        raise SystemExit(f"{name}.img size does not match manifest")

rootfs_sparse = inspect_sparse(os.path.join(package_dir, "rootfs.img"))
userdata_sparse = inspect_sparse(os.path.join(package_dir, "userdata.img"))
if rootfs_sparse["block_size"] != 4096 or rootfs_sparse["expanded_size_bytes"] != 2147483648:
    raise SystemExit(f"rootfs sparse size mismatch: {rootfs_sparse}")
if userdata_sparse["block_size"] != 4096 or userdata_sparse["expanded_size_bytes"] != 209715200:
    raise SystemExit(f"userdata sparse size mismatch: {userdata_sparse}")

if partition_images.get("rootfs", {}).get("sha256") != sha256(os.path.join(package_dir, "rootfs.img")):
    raise SystemExit("rootfs.img sha256 does not match manifest")
if partition_images.get("userdata", {}).get("sha256") != sha256(os.path.join(package_dir, "userdata.img")):
    raise SystemExit("userdata.img sha256 does not match manifest")

source = manifest.get("source_image", {})
source_path = source.get("path")
if source_path and os.path.exists(source_path):
    if sha256(source_path) != source.get("sha256"):
        raise SystemExit(f"source image hash changed after packaging: {source_path}")
    print(f"PASS original source image hash unchanged: {source_path}")
else:
    print("INFO original source image path not present; source pollution check skipped")

print("PASS manifest fields validate")
print("PASS parameter.txt layout validates")
print("PASS partition image sizes validate")
print("PASS rootfs/userdata Android sparse headers validate")
PY

printf 'Summary: 0 failure(s)\n'
