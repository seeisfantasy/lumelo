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
  --exclude='__pycache__/' \
  --exclude='.pytest_cache/' \
  --exclude='*.pyc' \
  --exclude='*~' \
  "${REPO_ROOT}/base/rootfs/overlay/" "${ROOTFS_MOUNT}/"

echo "==> enabling smoke services"
mkdir -p "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants"
rm -f "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants/local-mode.target"
rm -f "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants/bridge-mode.target"
ln -snf ../lumelo-mode-manager.service "${ROOTFS_MOUNT}/etc/systemd/system/multi-user.target.wants/lumelo-mode-manager.service"

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
