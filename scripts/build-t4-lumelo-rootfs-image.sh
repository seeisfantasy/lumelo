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
