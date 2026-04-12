#!/bin/sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
repo_root_default="$(CDPATH= cd -- "${script_dir}/.." && pwd)"

SOURCE_REPO_PATH="${SOURCE_REPO_PATH:-${repo_root_default}}"
DEST_REPO_PATH="${DEST_REPO_PATH:-/Volumes/LumeloDev/Codex/Lumelo}"

if [ "${SOURCE_REPO_PATH}" = "${DEST_REPO_PATH}" ]; then
  echo "source and destination must be different: ${SOURCE_REPO_PATH}" >&2
  exit 1
fi

"${script_dir}/mount-lumelodev-apfs.sh" >/dev/null
mkdir -p "$(dirname "${DEST_REPO_PATH}")"

rsync -a --delete \
  --exclude='._*' \
  --exclude='.DS_Store' \
  --exclude='out/' \
  --exclude='tmp/' \
  --exclude='apps/android-provisioning/.gradle/' \
  --exclude='apps/android-provisioning/build/' \
  --exclude='apps/android-provisioning/app/build/' \
  --exclude='services/rust/target/' \
  "${SOURCE_REPO_PATH}/" "${DEST_REPO_PATH}/"

echo "${DEST_REPO_PATH}"
