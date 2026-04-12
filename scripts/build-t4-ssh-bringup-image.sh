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
