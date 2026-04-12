#!/bin/sh
set -eu

SPARSEBUNDLE_PATH="${SPARSEBUNDLE_PATH:-/Volumes/SeeDisk/Codex/Lumelo-dev.sparsebundle}"
VOLUME_NAME="${VOLUME_NAME:-LumeloDev}"
MOUNT_POINT="/Volumes/${VOLUME_NAME}"

if mount | grep -F " on ${MOUNT_POINT} " >/dev/null 2>&1; then
  echo "${MOUNT_POINT}"
  exit 0
fi

if [ ! -e "${SPARSEBUNDLE_PATH}" ]; then
  echo "sparsebundle not found: ${SPARSEBUNDLE_PATH}" >&2
  exit 1
fi

hdiutil attach "${SPARSEBUNDLE_PATH}" >/dev/null

if mount | grep -F " on ${MOUNT_POINT} " >/dev/null 2>&1; then
  echo "${MOUNT_POINT}"
  exit 0
fi

echo "attached ${SPARSEBUNDLE_PATH}, but ${MOUNT_POINT} was not found" >&2
exit 1
