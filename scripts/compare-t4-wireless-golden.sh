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
