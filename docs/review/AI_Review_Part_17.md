# AI Review Part 17

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `scripts/deploy-t4-runtime-update.sh`

- bytes: 5867
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

usage() {
  cat <<'EOF' >&2
usage:
  deploy-t4-runtime-update.sh --host <T4_IP> [--user root] [--restart-unit <unit>] <overlay-file>...
  deploy-t4-runtime-update.sh --host <T4_IP> [--user root] [--restart-unit <unit>] --map <local:remote> ...

Deploy one or more files from base/rootfs/overlay onto a live T4 board over SSH.

Optional:
  LUMELO_T4_SSH_OPTIONS='-o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/tmp/lumelo_known_hosts'

Examples:
  ./scripts/deploy-t4-runtime-update.sh \
    --host 192.168.1.120 \
    --restart-unit lumelo-wifi-provisiond.service \
    base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond

  ./scripts/deploy-t4-runtime-update.sh \
    --host 192.168.1.120 \
    base/rootfs/overlay/usr/bin/lumelo-wifi-apply \
    base/rootfs/overlay/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond

  ./scripts/deploy-t4-runtime-update.sh \
    --host 192.168.1.120 \
    --restart-unit controld.service \
    --map /absolute/path/to/controld:/usr/bin/controld
EOF
  exit 64
}

stat_mode() {
  if mode=$(stat -f '%Lp' "$1" 2>/dev/null); then
    printf '%s\n' "$mode"
    return 0
  fi

  stat -c '%a' "$1"
}

append_newline() {
  if [ -z "$1" ]; then
    printf '%s' "$2"
  else
    printf '%s\n%s' "$1" "$2"
  fi
}

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "${script_dir}/.." && pwd)"
overlay_root="${repo_root}/base/rootfs/overlay"

host=""
user="root"
restart_units=""
copied_paths=""
needs_daemon_reload=0
mapped_entries=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --host)
      [ "$#" -ge 2 ] || usage
      host="$2"
      shift 2
      ;;
    --user)
      [ "$#" -ge 2 ] || usage
      user="$2"
      shift 2
      ;;
    --restart-unit)
      [ "$#" -ge 2 ] || usage
      restart_units=$(append_newline "${restart_units}" "$2")
      shift 2
      ;;
    --map)
      [ "$#" -ge 2 ] || usage
      mapped_entries=$(append_newline "${mapped_entries}" "$2")
      shift 2
      ;;
    --help|-h)
      usage
      ;;
    --*)
      echo "unknown option: $1" >&2
      usage
      ;;
    *)
      break
      ;;
  esac
done

[ -n "${host}" ] || usage
[ "$#" -gt 0 ] || [ -n "${mapped_entries}" ] || usage

remote="${user}@${host}"
run_id="$(date +%Y%m%d-%H%M%S)-$$"
tmp_root="/tmp/lumelo-runtime-update-${run_id}"
ssh_options="${LUMELO_T4_SSH_OPTIONS:-}"

ssh_cmd() {
  # shellcheck disable=SC2086
  ssh ${ssh_options} "${remote}" "$@"
}

scp_cmd() {
  # shellcheck disable=SC2086
  scp ${ssh_options} "$@"
}

printf 'Preparing runtime update on %s\n' "${remote}"
ssh_cmd "mkdir -p '${tmp_root}'"

cleanup() {
  ssh_cmd "rm -rf '${tmp_root}'" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

for source_arg in "$@"; do
  case "${source_arg}" in
    /*)
      source_path="${source_arg}"
      ;;
    *)
      source_path="${repo_root}/${source_arg}"
      ;;
  esac

  if [ ! -f "${source_path}" ]; then
    echo "overlay source file not found: ${source_arg}" >&2
    exit 66
  fi

  case "${source_path}" in
    "${overlay_root}"/*)
      ;;
    *)
      echo "source must live under ${overlay_root}: ${source_path}" >&2
      exit 64
      ;;
  esac

  relative_path="${source_path#${overlay_root}/}"
  remote_path="/${relative_path}"
  remote_dir=$(dirname "${remote_path}")
  remote_tmp="${tmp_root}/$(basename "${remote_path}")"
  mode=$(stat_mode "${source_path}")

  printf 'Deploying %s -> %s:%s\n' "${relative_path}" "${remote}" "${remote_path}"
  scp_cmd "${source_path}" "${remote}:${remote_tmp}"
  ssh_cmd "\
    mkdir -p '${remote_dir}' && \
    if [ -e '${remote_path}' ]; then \
      cp '${remote_path}' '${remote_path}.bak.${run_id}'; \
    fi && \
    install -m ${mode} '${remote_tmp}' '${remote_path}'"

  copied_paths=$(append_newline "${copied_paths}" "${remote_path}")

  case "${remote_path}" in
    /etc/systemd/system/*|/usr/lib/systemd/system/*)
      needs_daemon_reload=1
      ;;
  esac
done

if [ -n "${mapped_entries}" ]; then
  old_ifs=$IFS
  IFS='
'
  for mapped_entry in ${mapped_entries}; do
    [ -n "${mapped_entry}" ] || continue
    case "${mapped_entry}" in
      *:/*)
        local_path=${mapped_entry%%:*}
        remote_path=${mapped_entry#*:}
        ;;
      *)
        echo "mapped entry must look like local_path:/remote/path : ${mapped_entry}" >&2
        exit 64
        ;;
    esac

    if [ ! -f "${local_path}" ]; then
      echo "mapped source file not found: ${local_path}" >&2
      exit 66
    fi

    remote_dir=$(dirname "${remote_path}")
    remote_tmp="${tmp_root}/$(basename "${remote_path}")"
    mode=$(stat_mode "${local_path}")

    printf 'Deploying mapped artifact %s -> %s:%s\n' "${local_path}" "${remote}" "${remote_path}"
    scp_cmd "${local_path}" "${remote}:${remote_tmp}"
    ssh_cmd "\
      mkdir -p '${remote_dir}' && \
      if [ -e '${remote_path}' ]; then \
        cp '${remote_path}' '${remote_path}.bak.${run_id}'; \
      fi && \
      install -m ${mode} '${remote_tmp}' '${remote_path}'"

    copied_paths=$(append_newline "${copied_paths}" "${remote_path}")

    case "${remote_path}" in
      /etc/systemd/system/*|/usr/lib/systemd/system/*)
        needs_daemon_reload=1
        ;;
    esac
  done
  IFS=$old_ifs
fi

if [ "${needs_daemon_reload}" -eq 1 ]; then
  printf 'Running systemctl daemon-reload on %s\n' "${remote}"
  ssh_cmd "systemctl daemon-reload"
fi

if [ -n "${restart_units}" ]; then
  printf '%s\n' "${restart_units}" | while IFS= read -r unit; do
    [ -n "${unit}" ] || continue
    printf 'Restarting %s on %s\n' "${unit}" "${remote}"
    ssh_cmd "systemctl restart '${unit}' && systemctl is-active '${unit}'"
  done
fi

printf 'Runtime update applied to %s\n' "${remote}"
printf 'Updated paths:\n'
printf '%s\n' "${copied_paths}" | while IFS= read -r path; do
  [ -n "${path}" ] || continue
  printf '  %s\n' "${path}"
done
~~~

## `scripts/dev-controld.sh`

- bytes: 662
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

runtime_dir="${LUMELO_RUNTIME_DIR:-${PRODUCT_RUNTIME_DIR:-/tmp/lumelo}}"
state_dir="${LUMELO_STATE_DIR:-${PRODUCT_STATE_DIR:-/tmp/lumelo-state}}"
listen_addr="${CONTROLD_LISTEN_ADDR:-127.0.0.1:18080}"
script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"

mkdir -p "${runtime_dir}"
mkdir -p "${state_dir}"

export LUMELO_RUNTIME_DIR="${runtime_dir}"
export PRODUCT_RUNTIME_DIR="${runtime_dir}"
export LUMELO_STATE_DIR="${state_dir}"
export PRODUCT_STATE_DIR="${state_dir}"
export CONTROLD_LISTEN_ADDR="${listen_addr}"
export GOCACHE="${GOCACHE:-/tmp/lumelo-go-build-cache}"

cd "${script_dir}/../services/controld"
exec go run ./cmd/controld
~~~

## `scripts/dev-media-indexd.sh`

- bytes: 747
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

state_dir="${LUMELO_STATE_DIR:-${PRODUCT_STATE_DIR:-/tmp/lumelo-state}}"
cache_dir="${LUMELO_CACHE_DIR:-${PRODUCT_CACHE_DIR:-/tmp/lumelo-cache}}"
script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"

mkdir -p "${state_dir}"
mkdir -p "${cache_dir}/artwork"

export LUMELO_STATE_DIR="${state_dir}"
export PRODUCT_STATE_DIR="${state_dir}"
export LUMELO_CACHE_DIR="${cache_dir}"
export PRODUCT_CACHE_DIR="${cache_dir}"
export LIBRARY_DB_PATH="${LIBRARY_DB_PATH:-${state_dir}/library.db}"
export ARTWORK_CACHE_DIR="${ARTWORK_CACHE_DIR:-${cache_dir}/artwork}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/lumelo-cargo-target}"

exec cargo run --manifest-path "${script_dir}/../services/rust/Cargo.toml" -p media-indexd -- "$@"
~~~

## `scripts/dev-playbackd.sh`

- bytes: 599
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

runtime_dir="${LUMELO_RUNTIME_DIR:-${PRODUCT_RUNTIME_DIR:-/tmp/lumelo}}"
state_dir="${LUMELO_STATE_DIR:-${PRODUCT_STATE_DIR:-/tmp/lumelo-state}}"
script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"

mkdir -p "${runtime_dir}"
mkdir -p "${state_dir}"

export LUMELO_RUNTIME_DIR="${runtime_dir}"
export PRODUCT_RUNTIME_DIR="${runtime_dir}"
export LUMELO_STATE_DIR="${state_dir}"
export PRODUCT_STATE_DIR="${state_dir}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/lumelo-cargo-target}"

exec cargo run --manifest-path "${script_dir}/../services/rust/Cargo.toml" -p playbackd
~~~

## `scripts/dev-sessiond.sh`

- bytes: 598
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

runtime_dir="${LUMELO_RUNTIME_DIR:-${PRODUCT_RUNTIME_DIR:-/tmp/lumelo}}"
state_dir="${LUMELO_STATE_DIR:-${PRODUCT_STATE_DIR:-/tmp/lumelo-state}}"
script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"

mkdir -p "${runtime_dir}"
mkdir -p "${state_dir}"

export LUMELO_RUNTIME_DIR="${runtime_dir}"
export PRODUCT_RUNTIME_DIR="${runtime_dir}"
export LUMELO_STATE_DIR="${state_dir}"
export PRODUCT_STATE_DIR="${state_dir}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/lumelo-cargo-target}"

exec cargo run --manifest-path "${script_dir}/../services/rust/Cargo.toml" -p sessiond
~~~

## `scripts/dev-up.sh`

- bytes: 931
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

runtime_dir="${LUMELO_RUNTIME_DIR:-${PRODUCT_RUNTIME_DIR:-/tmp/lumelo}}"
state_dir="${LUMELO_STATE_DIR:-${PRODUCT_STATE_DIR:-/tmp/lumelo-state}}"
listen_addr="${CONTROLD_LISTEN_ADDR:-127.0.0.1:18080}"
script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"

mkdir -p "${runtime_dir}"
mkdir -p "${state_dir}"

cat <<EOF
Start these in separate terminals:

1. LUMELO_RUNTIME_DIR=${runtime_dir} LUMELO_STATE_DIR=${state_dir} ${script_dir}/dev-playbackd.sh
2. LUMELO_RUNTIME_DIR=${runtime_dir} LUMELO_STATE_DIR=${state_dir} ${script_dir}/dev-sessiond.sh
3. LUMELO_RUNTIME_DIR=${runtime_dir} LUMELO_STATE_DIR=${state_dir} CONTROLD_LISTEN_ADDR=${listen_addr} ${script_dir}/dev-controld.sh

Then open:
  http://${listen_addr}/

Optional one-shot library init:
  LUMELO_STATE_DIR=${state_dir} ${script_dir}/dev-media-indexd.sh ensure-schema
  LUMELO_STATE_DIR=${state_dir} ${script_dir}/dev-media-indexd.sh seed-demo
EOF
~~~

## `scripts/mount-lumelodev-apfs.sh`

- bytes: 631
- segment: 1/1

~~~bash
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
~~~

## `scripts/orbstack-bootstrap-fono-dev.sh`

- bytes: 133
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
exec "${script_dir}/orbstack-bootstrap-lumelo-dev.sh" "$@"
~~~

## `scripts/orbstack-bootstrap-lumelo-dev.sh`

- bytes: 3061
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
repo_root_default="$(CDPATH= cd -- "${script_dir}/.." && pwd)"

ORB_BIN="${ORB_BIN:-/opt/homebrew/bin/orb}"
MACHINE_NAME="${MACHINE_NAME:-lumelo-dev}"
DISTRO="${DISTRO:-debian:12}"
GO_VERSION="${GO_VERSION:-1.26.1}"
GO_TARBALL="go${GO_VERSION}.linux-arm64.tar.gz"
GO_URL="https://go.dev/dl/${GO_TARBALL}"
REPO_HOST_PATH="${REPO_HOST_PATH:-${repo_root_default}}"
REPO_VM_PATH="${REPO_VM_PATH:-/mnt/mac${REPO_HOST_PATH}}"

if [ ! -x "${ORB_BIN}" ]; then
  echo "orb not found at ${ORB_BIN}" >&2
  exit 1
fi

if [ ! -d "${REPO_HOST_PATH}" ]; then
  echo "repo path not found: ${REPO_HOST_PATH}" >&2
  exit 1
fi

orb_status="$(${ORB_BIN} status 2>/dev/null || true)"
if [ "${orb_status}" != "Running" ]; then
  echo "OrbStack is not running yet." >&2
  echo "Open /Applications/OrbStack.app once, finish the first-run onboarding," >&2
  echo "and rerun this script." >&2
  exit 1
fi

if ! "${ORB_BIN}" info "${MACHINE_NAME}" >/dev/null 2>&1; then
  "${ORB_BIN}" create -a arm64 "${DISTRO}" "${MACHINE_NAME}"
fi

"${ORB_BIN}" default "${MACHINE_NAME}"

"${ORB_BIN}" -m "${MACHINE_NAME}" sudo apt-get update
"${ORB_BIN}" -m "${MACHINE_NAME}" sudo apt-get install -y \
  ca-certificates \
  curl \
  git \
  build-essential \
  pkg-config \
  libasound2-dev \
  sqlite3 \
  systemd

"${ORB_BIN}" -m "${MACHINE_NAME}" bash -lc '
set -eu
if [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  curl -fsSL https://sh.rustup.rs | sh -s -- -y
fi
'

"${ORB_BIN}" -m "${MACHINE_NAME}" sudo bash -lc "
set -eu
curl -fsSL '${GO_URL}' -o '/tmp/${GO_TARBALL}'
rm -rf /usr/local/go
tar -C /usr/local -xzf '/tmp/${GO_TARBALL}'
install -d -m 0755 /etc/profile.d
printf '%s\n' 'export PATH=/usr/local/go/bin:\$PATH' > /etc/profile.d/go.sh
chmod 0644 /etc/profile.d/go.sh
"

"${ORB_BIN}" -m "${MACHINE_NAME}" bash -lc '
set -eu
profile_line="export PATH=/usr/local/go/bin:\$HOME/.cargo/bin:\$PATH"
if ! grep -Fqs "$profile_line" "$HOME/.profile"; then
  printf "\n%s\n" "$profile_line" >> "$HOME/.profile"
fi
'

"${ORB_BIN}" -m "${MACHINE_NAME}" bash -lc '
set -eu
. "$HOME/.cargo/env"
export PATH=/usr/local/go/bin:$PATH
go version
cargo --version
rustc --version
'

cat <<EOF
OrbStack machine is ready.

Useful commands:
  orb -m ${MACHINE_NAME}
  orb -m ${MACHINE_NAME} -w '${REPO_VM_PATH}' bash -lc '. "\$HOME/.cargo/env" && export PATH=/usr/local/go/bin:\$PATH LUMELO_RUNTIME_DIR=/tmp/lumelo CARGO_TARGET_DIR=/tmp/lumelo-cargo-target && cargo test --manifest-path services/rust/Cargo.toml'
  orb -m ${MACHINE_NAME} -w '${REPO_VM_PATH}/services/controld' bash -lc 'export PATH=/usr/local/go/bin:\$HOME/.cargo/bin:\$PATH LUMELO_RUNTIME_DIR=/tmp/lumelo GOCACHE=/tmp/lumelo-go-build-cache && go test ./...'
  orb -m ${MACHINE_NAME} -w '${REPO_VM_PATH}' bash -lc 'systemd-analyze verify base/rootfs/overlay/etc/systemd/system/playbackd.service base/rootfs/overlay/etc/systemd/system/sessiond.service base/rootfs/overlay/etc/systemd/system/controld.service base/rootfs/overlay/etc/systemd/system/media-indexd.service'
EOF
~~~

## `scripts/sync-to-lumelodev-apfs.sh`

- bytes: 878
- segment: 1/1

~~~bash
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
~~~

## `scripts/verify-t4-lumelo-rootfs-image.sh`

- bytes: 15113
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  verify-t4-lumelo-rootfs-image.sh /path/to/lumelo-t4-rootfs.img

Notes:
  - Runs on Linux with root privileges.
  - Mounts partition p8 read-only and validates the minimal Lumelo rootfs payload.
  - Does not attempt to boot the image or validate real T4 hardware devices.
EOF
}

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "verify-t4-lumelo-rootfs-image.sh must run as root" >&2
    exit 1
  fi
}

require_linux() {
  if [ "$(uname -s)" != "Linux" ]; then
    echo "verify-t4-lumelo-rootfs-image.sh must run on Linux" >&2
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

expect_file() {
  path=$1
  label=$2
  if [ -f "${ROOTFS_MOUNT}${path}" ]; then
    pass "${label}: ${path}"
  else
    fail "${label}: missing ${path}"
  fi
}

expect_executable() {
  path=$1
  label=$2
  if [ -x "${ROOTFS_MOUNT}${path}" ]; then
    pass "${label}: ${path}"
  else
    fail "${label}: missing or not executable ${path}"
  fi
}

expect_dir() {
  path=$1
  label=$2
  if [ -d "${ROOTFS_MOUNT}${path}" ]; then
    pass "${label}: ${path}"
  else
    fail "${label}: missing ${path}"
  fi
}

expect_symlink() {
  path=$1
  label=$2
  if [ -L "${ROOTFS_MOUNT}${path}" ]; then
    pass "${label}: ${path}"
  else
    fail "${label}: missing symlink ${path}"
  fi
}

expect_symlink_target() {
  path=$1
  expected=$2
  label=$3

  if [ ! -L "${ROOTFS_MOUNT}${path}" ]; then
    fail "${label}: missing symlink ${path}"
    return
  fi

  actual=$(readlink "${ROOTFS_MOUNT}${path}")
  if [ "${actual}" = "${expected}" ]; then
    pass "${label}: ${path} -> ${actual}"
  else
    fail "${label}: ${path} -> ${actual} (expected ${expected})"
  fi
}

expect_text() {
  path=$1
  needle=$2
  label=$3
  if [ ! -f "${ROOTFS_MOUNT}${path}" ]; then
    fail "${label}: missing ${path}"
    return
  fi

  if grep -F "${needle}" "${ROOTFS_MOUNT}${path}" >/dev/null 2>&1; then
    pass "${label}: found '${needle}' in ${path}"
  else
    fail "${label}: did not find '${needle}' in ${path}"
  fi
}

expect_traversable_dir() {
  path=$1
  label=$2

  if [ ! -d "${ROOTFS_MOUNT}${path}" ]; then
    fail "${label}: missing ${path}"
    return
  fi

  mode=$(stat -c '%a' "${ROOTFS_MOUNT}${path}")
  group_digit=$(printf '%s' "${mode}" | sed 's/.*\(.\).$/\1/')
  other_digit=$(printf '%s' "${mode}" | sed 's/.*\(.\)$/\1/')

  case "${group_digit}" in
    1|3|5|7) group_exec=1 ;;
    *) group_exec=0 ;;
  esac
  case "${other_digit}" in
    1|3|5|7) other_exec=1 ;;
    *) other_exec=0 ;;
  esac

  if [ "${group_exec}" = "1" ] && [ "${other_exec}" = "1" ]; then
    pass "${label}: ${path} mode ${mode}"
  else
    fail "${label}: ${path} mode ${mode} is not traversable for non-root services"
  fi
}

expect_mode() {
  path=$1
  expected_mode=$2
  label=$3

  if [ ! -e "${ROOTFS_MOUNT}${path}" ]; then
    fail "${label}: missing ${path}"
    return
  fi

  actual_mode=$(stat -c '%a' "${ROOTFS_MOUNT}${path}")
  if [ "${actual_mode}" = "${expected_mode}" ]; then
    pass "${label}: ${path} mode ${actual_mode}"
  else
    fail "${label}: ${path} mode ${actual_mode} != expected ${expected_mode}"
  fi
}

cleanup() {
  status=$?
  if [ -n "${ROOTFS_MOUNT:-}" ] && mountpoint -q "${ROOTFS_MOUNT}" 2>/dev/null; then
    umount "${ROOTFS_MOUNT}" || true
  fi
  if [ -n "${LOOPDEV:-}" ]; then
    losetup -d "${LOOPDEV}" || true
  fi
  if [ -n "${ROOTFS_MOUNT:-}" ] && [ -d "${ROOTFS_MOUNT}" ]; then
    rmdir "${ROOTFS_MOUNT}" 2>/dev/null || true
  fi
  exit "${status}"
}

if [ "$#" -ne 1 ]; then
  usage >&2
  exit 1
fi

IMAGE=$1
if [ ! -f "${IMAGE}" ]; then
  echo "image not found: ${IMAGE}" >&2
  exit 1
fi

require_linux
require_root
require_cmd grep
require_cmd losetup
require_cmd mount
require_cmd mountpoint
require_cmd mktemp
require_cmd partx
require_cmd readlink
require_cmd stat
require_cmd umount
require_cmd rmdir

WARNINGS=0
FAILURES=0
ROOTFS_MOUNT=
LOOPDEV=
trap cleanup EXIT INT TERM

ROOTFS_MOUNT=$(mktemp -d "${TMPDIR:-/tmp}/lumelo-rootfs-verify.XXXXXX")
LOOPDEV=$(losetup --find --partscan --show "${IMAGE}")
partx -a "${LOOPDEV}" >/dev/null 2>&1 || true

wait_for_partition "${LOOPDEV}p8" || {
  echo "expected rootfs partition not found: ${LOOPDEV}p8" >&2
  exit 1
}
wait_for_partition "${LOOPDEV}p9" || {
  echo "expected userdata partition not found: ${LOOPDEV}p9" >&2
  exit 1
}

mount -o ro "${LOOPDEV}p8" "${ROOTFS_MOUNT}"

printf 'Verifying image: %s\n' "${IMAGE}"
printf 'Rootfs mount: %s\n\n' "${ROOTFS_MOUNT}"

expect_text /etc/os-release "trixie" "Debian suite"
expect_text /etc/lumelo/image-build.txt "Lumelo-defined rootfs image profile: t4-bringup" "build marker"
expect_text /etc/lumelo/config.toml 'mode = "local"' "runtime config"
expect_traversable_dir /etc "etc directory permissions"
expect_traversable_dir /usr "usr directory permissions"
expect_traversable_dir /usr/lib "usr lib directory permissions"
expect_mode /etc/bluetooth 555 "bluetooth config directory mode"

expect_executable /usr/bin/playbackd "playbackd"
expect_executable /usr/bin/sessiond "sessiond"
expect_executable /usr/bin/media-indexd "media-indexd"
expect_executable /usr/bin/controld "controld"
expect_executable /usr/bin/hciattach.rk "Rockchip Bluetooth UART attach helper"
expect_executable /usr/bin/sdptool "Bluetooth SDP helper"
expect_executable /usr/lib/systemd/systemd-networkd "systemd-networkd binary"
expect_executable /usr/lib/systemd/systemd-resolved "systemd-resolved binary"

expect_file /etc/systemd/network/20-wired-dhcp.network "wired DHCP"
expect_text /etc/systemd/network/20-wired-dhcp.network "LinkLocalAddressing=no" "wired DHCP link-local policy"
expect_text /etc/systemd/network/20-wired-dhcp.network "LLMNR=no" "wired DHCP LLMNR policy"
expect_text /etc/systemd/network/20-wired-dhcp.network "MulticastDNS=no" "wired DHCP mDNS policy"
expect_text /etc/systemd/network/20-wired-dhcp.network "ClientIdentifier=mac" "wired DHCP client id"
expect_text /etc/systemd/network/30-wireless-dhcp.network "LinkLocalAddressing=no" "wireless DHCP link-local policy"
expect_text /etc/systemd/network/30-wireless-dhcp.network "LLMNR=no" "wireless DHCP LLMNR policy"
expect_text /etc/systemd/network/30-wireless-dhcp.network "MulticastDNS=no" "wireless DHCP mDNS policy"
expect_file /etc/NetworkManager/NetworkManager.conf "NetworkManager baseline config"
expect_text /etc/NetworkManager/NetworkManager.conf "plugins=ifupdown,keyfile" "NetworkManager plugin baseline"
expect_file /etc/NetworkManager/conf.d/12-managed-wifi.conf "NetworkManager managed Wi-Fi policy"
expect_text /etc/NetworkManager/conf.d/12-managed-wifi.conf "unmanaged-devices=wl*,except:type:wifi" "NetworkManager wl* exception policy"
expect_file /etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf "NetworkManager wlan1 exclusion policy"
expect_text /etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf "unmanaged-devices=interface-name:wlan1" "NetworkManager wlan1 unmanaged policy"
expect_file /etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf "NetworkManager scan MAC policy"
expect_text /etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf "wifi.scan-rand-mac-address=no" "NetworkManager scan MAC randomization policy"
expect_file /etc/network/interfaces "ifupdown base interfaces file"
expect_text /etc/network/interfaces "source /etc/network/interfaces.d/*" "ifupdown include policy"
expect_text /etc/systemd/resolved.conf.d/lumelo.conf "LLMNR=no" "resolved LLMNR policy"
expect_text /etc/systemd/resolved.conf.d/lumelo.conf "MulticastDNS=no" "resolved mDNS policy"
expect_text /etc/systemd/system/controld.service "CONTROLD_LISTEN_ADDR=0.0.0.0:18080" "controld bring-up listen address"
expect_file /etc/systemd/system/local-mode.target "local-mode target"
expect_text /etc/systemd/system/local-mode.target "media-indexd.service" "local-mode target media-index dependency"
expect_file /etc/systemd/system/playbackd.service "playbackd unit"
expect_file /etc/systemd/system/sessiond.service "sessiond unit"
expect_file /etc/systemd/system/media-indexd.service "media-indexd unit"
expect_file /etc/systemd/system/controld.service "controld unit"
expect_file /etc/systemd/system/bluetooth.service.d/10-lumelo-rfkill-unblock.conf "bluetooth rfkill unblock drop-in"
expect_text /etc/systemd/system/bluetooth.service.d/10-lumelo-rfkill-unblock.conf "rfkill unblock bluetooth" "bluetooth unblock policy"
expect_file /etc/systemd/system/bluetooth.service.d/20-lumelo-uart-attach.conf "bluetooth UART attach drop-in"
expect_text /etc/systemd/system/bluetooth.service.d/20-lumelo-uart-attach.conf "Requires=lumelo-bluetooth-uart-attach.service" "bluetooth UART attach dependency"
expect_text /etc/systemd/system/bluetooth.service.d/20-lumelo-uart-attach.conf "After=lumelo-bluetooth-uart-attach.service" "bluetooth UART attach ordering"
expect_file /etc/systemd/system/lumelo-bluetooth-uart-attach.service "Bluetooth UART attach unit"
expect_executable /usr/libexec/lumelo/bluetooth-uart-attach "Bluetooth UART attach wrapper"
expect_symlink /etc/systemd/system/multi-user.target.wants/local-mode.target "local-mode enablement"
expect_symlink /etc/systemd/system/multi-user.target.wants/lumelo-bluetooth-uart-attach.service "Bluetooth UART attach enablement"

expect_dir /lib/modules/4.19.232 "FriendlyELEC kernel modules"
expect_dir /lib/firmware "FriendlyELEC firmware directory"
expect_dir /etc/firmware "FriendlyELEC bluetooth patch directory"
expect_file /etc/firmware/BCM4356A2.hcd "FriendlyELEC BCM4356 bluetooth patch firmware"
expect_file /etc/modprobe.d/bcmdhd.conf "FriendlyELEC bcmdhd driver policy"
expect_text /etc/modprobe.d/bcmdhd.conf "options bcmdhd op_mode=5" "bcmdhd op_mode policy"
expect_text /etc/modprobe.d/bcmdhd.conf "alias sdio:c*v02D0d4356* bcmdhd" "bcmdhd BCM4356 alias"
expect_dir /system/etc/firmware "FriendlyELEC vendor wireless firmware directory"
expect_file /system/etc/firmware/fw_bcm4356a2_ag.bin "FriendlyELEC Broadcom Wi-Fi firmware blob"
expect_file /system/etc/firmware/nvram_ap6356.txt "FriendlyELEC AP6356 NVRAM calibration"
expect_text /usr/libexec/lumelo/bluetooth-uart-attach "/sys/module/bcmdhd" "bluetooth attach waits for bcmdhd"
expect_text /usr/libexec/lumelo/bluetooth-uart-attach "timeout 5 btmgmt info" "bluetooth attach bounds btmgmt probe"
expect_text /usr/libexec/lumelo/bluetooth-uart-attach "grep -Eq '^hci[0-9]+:'" "bluetooth attach requires discovered hci controller"
expect_text /usr/libexec/lumelo/bluetooth-uart-attach 'exec "${ATTACH_HELPER}" "${ATTACH_UART}" "${ATTACH_CHIPSET}" "${ATTACH_BAUD}"' "bluetooth attach helper exec"

if grep -F "SSH enabled in image: 1" "${ROOTFS_MOUNT}/etc/lumelo/image-build.txt" >/dev/null 2>&1; then
  expect_text /etc/lumelo/config.toml "ssh_enabled = true" "ssh runtime config"
  expect_symlink /etc/systemd/system/multi-user.target.wants/ssh.service "ssh enablement"
  expect_text /etc/ssh/sshd_config.d/90-lumelo-development.conf "PermitRootLogin yes" "ssh root login policy"
  expect_text /etc/ssh/sshd_config.d/90-lumelo-development.conf "PasswordAuthentication yes" "ssh password login policy"
  expect_file /etc/systemd/system/lumelo-ssh-hostkeys.service "ssh host key generator unit"
  expect_text /etc/systemd/system/lumelo-ssh-hostkeys.service "ExecStart=/usr/bin/ssh-keygen -A" "ssh host key generation policy"
  expect_text /etc/systemd/system/ssh.service.d/10-lumelo-hostkeys.conf "Requires=lumelo-ssh-hostkeys.service" "ssh host key dependency"
  expect_text /etc/systemd/system/ssh.service.d/10-lumelo-hostkeys.conf "After=lumelo-ssh-hostkeys.service" "ssh host key ordering"

  if grep -F "SSH authorized_keys injected: 1" "${ROOTFS_MOUNT}/etc/lumelo/image-build.txt" >/dev/null 2>&1; then
    expect_file /root/.ssh/authorized_keys "ssh authorized_keys"
  fi
else
  expect_text /etc/lumelo/config.toml "ssh_enabled = false" "ssh runtime config"
fi

if [ -x "${ROOTFS_MOUNT}/usr/bin/lumelo-t4-report" ]; then
  pass "bring-up report tool: /usr/bin/lumelo-t4-report"
else
  warn "bring-up report tool not present; expected only in images rebuilt after 2026-04-07 08:50"
fi

if [ -x "${ROOTFS_MOUNT}/usr/bin/lumelo-audio-smoke" ]; then
  pass "ALSA smoke helper: /usr/bin/lumelo-audio-smoke"
else
  warn "ALSA smoke helper not present; expected only in images rebuilt after 2026-04-07 09:10"
fi

if [ -x "${ROOTFS_MOUNT}/usr/bin/lumelo-bluetooth-provisioning-mode" ]; then
  pass "Bluetooth provisioning helper: /usr/bin/lumelo-bluetooth-provisioning-mode"
else
  warn "Bluetooth provisioning helper not present; expected only in images rebuilt after 2026-04-08 02:55"
fi

if [ -x "${ROOTFS_MOUNT}/usr/bin/lumelo-wifi-apply" ]; then
  pass "Wi-Fi credential helper: /usr/bin/lumelo-wifi-apply"
else
  warn "Wi-Fi credential helper not present; expected only in images rebuilt after 2026-04-08 02:55"
fi

if [ -f "${ROOTFS_MOUNT}/etc/systemd/system/lumelo-bluetooth-provisioning.service" ]; then
  pass "Bluetooth provisioning unit: /etc/systemd/system/lumelo-bluetooth-provisioning.service"
else
  warn "Bluetooth provisioning unit not present; expected only in images rebuilt after 2026-04-08 02:55"
fi

if [ -x "${ROOTFS_MOUNT}/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond" ]; then
  pass "Classic Bluetooth Wi-Fi provisioning daemon: /usr/libexec/lumelo/classic-bluetooth-wifi-provisiond"
else
  warn "Classic Bluetooth Wi-Fi provisioning daemon not present; expected only in images rebuilt after 2026-04-12 16:30"
fi

if [ -f "${ROOTFS_MOUNT}/etc/systemd/system/lumelo-wifi-provisiond.service" ]; then
  pass "Bluetooth Wi-Fi provisioning unit: /etc/systemd/system/lumelo-wifi-provisiond.service"
else
  warn "Bluetooth Wi-Fi provisioning unit not present; expected only in images rebuilt after 2026-04-12 16:30"
fi

if [ -f "${ROOTFS_MOUNT}/etc/dbus-1/system.d/org.lumelo.provisioning.conf" ]; then
  pass "Provisioning DBus policy: /etc/dbus-1/system.d/org.lumelo.provisioning.conf"
  expect_text \
    /etc/dbus-1/system.d/org.lumelo.provisioning.conf \
    'allow own="org.lumelo.provisioning"' \
    "Provisioning DBus own policy"
  expect_text \
    /etc/dbus-1/system.d/org.lumelo.provisioning.conf \
    'allow send_destination="org.lumelo.provisioning"' \
    "Provisioning DBus send policy"
else
  fail "Provisioning DBus policy: missing /etc/dbus-1/system.d/org.lumelo.provisioning.conf"
fi

if [ -f "${ROOTFS_MOUNT}/etc/systemd/network/30-wireless-dhcp.network" ]; then
  pass "wireless DHCP network: /etc/systemd/network/30-wireless-dhcp.network"
else
  warn "wireless DHCP network not present; expected only in images rebuilt after 2026-04-08 02:55"
fi

printf '\nSummary: %s failure(s), %s warning(s)\n' "${FAILURES}" "${WARNINGS}"
if [ "${FAILURES}" -ne 0 ]; then
  exit 1
fi
~~~

## `services/controld/README.md`

- bytes: 378
- segment: 1/1

~~~md
# Controld

`controld` is the Go control-plane service for V1.

This skeleton keeps the scope narrow:

- embedded SSR templates and static files
- a tiny HTTP server entrypoint
- placeholder internal packages for auth, settings, SSH, playback IPC, and
  library access

Real playback commands, auth storage, and settings persistence will be layered
in on top of this structure.
~~~

## `services/controld/cmd/controld/main.go`

- bytes: 2693
- segment: 1/1

~~~go
package main

import (
	"log"
	"net/http"
	"os"
	"path/filepath"

	"github.com/lumelo/controld/internal/api"
	"github.com/lumelo/controld/internal/auth"
	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/logclient"
	"github.com/lumelo/controld/internal/playbackclient"
	"github.com/lumelo/controld/internal/provisioningclient"
	"github.com/lumelo/controld/internal/settings"
	"github.com/lumelo/controld/internal/sshctl"
	"github.com/lumelo/controld/web"
)

func main() {
	configPath := getenvWithFallbacks([]string{"CONTROLD_CONFIG_PATH", "LUMELO_CONFIG_PATH"}, settings.Default().ConfigPath)
	cfg, err := settings.Load(configPath)
	if err != nil {
		log.Printf("load controld config %s: %v; using defaults", configPath, err)
		cfg = settings.Default()
		cfg.ConfigPath = configPath
	}
	runtimeDir := getenvWithFallbacks([]string{"LUMELO_RUNTIME_DIR", "PRODUCT_RUNTIME_DIR"}, "/run/lumelo")
	stateDir := getenvWithFallbacks([]string{"LUMELO_STATE_DIR", "PRODUCT_STATE_DIR"}, "/var/lib/lumelo")
	commandSocket := getenv("CONTROLD_PLAYBACK_CMD_SOCKET", filepath.Join(runtimeDir, "playback_cmd.sock"))
	eventSocket := getenv("CONTROLD_PLAYBACK_EVT_SOCKET", filepath.Join(runtimeDir, "playback_evt.sock"))
	libraryDBPath := getenvWithFallbacks(
		[]string{"CONTROLD_LIBRARY_DB_PATH", "LIBRARY_DB_PATH"},
		filepath.Join(stateDir, "library.db"),
	)
	artworkCacheRoot := getenv("CONTROLD_ARTWORK_CACHE_DIR", "/var/cache/lumelo/artwork")
	provisioningStatusPath := getenv("CONTROLD_PROVISIONING_STATUS_PATH", filepath.Join(runtimeDir, "provisioning-status.json"))

	server, err := api.New(api.Dependencies{
		Auth:             auth.NewService(false),
		Playback:         playbackclient.New(commandSocket, eventSocket),
		Library:          libraryclient.New(libraryDBPath),
		Logs:             logclient.New(),
		Provisioning:     provisioningclient.New(provisioningStatusPath),
		Settings:         cfg,
		SSH:              sshctl.NewController(cfg.SSHEnabled),
		Templates:        web.Assets,
		Static:           web.Assets,
		ArtworkCacheRoot: artworkCacheRoot,
	})
	if err != nil {
		log.Fatalf("build controld server: %v", err)
	}

	addr := getenv("CONTROLD_LISTEN_ADDR", ":8080")
	log.Printf("lumelo controld listening on %s", addr)

	if err := http.ListenAndServe(addr, server.Handler()); err != nil {
		log.Fatalf("serve controld: %v", err)
	}
}

func getenv(key, fallback string) string {
	value := os.Getenv(key)
	if value == "" {
		return fallback
	}

	return value
}

func getenvWithFallbacks(keys []string, fallback string) string {
	for _, key := range keys {
		if value := os.Getenv(key); value != "" {
			return value
		}
	}

	return fallback
}
~~~

## `services/controld/go.mod`

- bytes: 521
- segment: 1/1

~~~text
module github.com/lumelo/controld

go 1.22

require modernc.org/sqlite v1.34.5

require (
	github.com/dustin/go-humanize v1.0.1 // indirect
	github.com/google/uuid v1.6.0 // indirect
	github.com/mattn/go-isatty v0.0.20 // indirect
	github.com/ncruces/go-strftime v0.1.9 // indirect
	github.com/remyoudompheng/bigfft v0.0.0-20230129092748-24d4a6f8daec // indirect
	golang.org/x/sys v0.22.0 // indirect
	modernc.org/libc v1.55.3 // indirect
	modernc.org/mathutil v1.6.0 // indirect
	modernc.org/memory v1.8.0 // indirect
)
~~~

## `services/controld/go.sum`

- bytes: 3562
- segment: 1/1

~~~text
github.com/dustin/go-humanize v1.0.1 h1:GzkhY7T5VNhEkwH0PVJgjz+fX1rhBrR7pRT3mDkpeCY=
github.com/dustin/go-humanize v1.0.1/go.mod h1:Mu1zIs6XwVuF/gI1OepvI0qD18qycQx+mFykh5fBlto=
github.com/google/pprof v0.0.0-20240409012703-83162a5b38cd h1:gbpYu9NMq8jhDVbvlGkMFWCjLFlqqEZjEmObmhUy6Vo=
github.com/google/pprof v0.0.0-20240409012703-83162a5b38cd/go.mod h1:kf6iHlnVGwgKolg33glAes7Yg/8iWP8ukqeldJSO7jw=
github.com/google/uuid v1.6.0 h1:NIvaJDMOsjHA8n1jAhLSgzrAzy1Hgr+hNrb57e+94F0=
github.com/google/uuid v1.6.0/go.mod h1:TIyPZe4MgqvfeYDBFedMoGGpEw/LqOeaOT+nhxU+yHo=
github.com/mattn/go-isatty v0.0.20 h1:xfD0iDuEKnDkl03q4limB+vH+GxLEtL/jb4xVJSWWEY=
github.com/mattn/go-isatty v0.0.20/go.mod h1:W+V8PltTTMOvKvAeJH7IuucS94S2C6jfK/D7dTCTo3Y=
github.com/ncruces/go-strftime v0.1.9 h1:bY0MQC28UADQmHmaF5dgpLmImcShSi2kHU9XLdhx/f4=
github.com/ncruces/go-strftime v0.1.9/go.mod h1:Fwc5htZGVVkseilnfgOVb9mKy6w1naJmn9CehxcKcls=
github.com/remyoudompheng/bigfft v0.0.0-20230129092748-24d4a6f8daec h1:W09IVJc94icq4NjY3clb7Lk8O1qJ8BdBEF8z0ibU0rE=
github.com/remyoudompheng/bigfft v0.0.0-20230129092748-24d4a6f8daec/go.mod h1:qqbHyh8v60DhA7CoWK5oRCqLrMHRGoxYCSS9EjAz6Eo=
golang.org/x/mod v0.16.0 h1:QX4fJ0Rr5cPQCF7O9lh9Se4pmwfwskqZfq5moyldzic=
golang.org/x/mod v0.16.0/go.mod h1:hTbmBsO62+eylJbnUtE2MGJUyE7QWk4xUqPFrRgJ+7c=
golang.org/x/sys v0.6.0/go.mod h1:oPkhp1MJrh7nUepCBck5+mAzfO9JrbApNNgaTdGDITg=
golang.org/x/sys v0.22.0 h1:RI27ohtqKCnwULzJLqkv897zojh5/DwS/ENaMzUOaWI=
golang.org/x/sys v0.22.0/go.mod h1:/VUhepiaJMQUp4+oa/7Zr1D23ma6VTLIYjOOTFZPUcA=
golang.org/x/tools v0.19.0 h1:tfGCXNR1OsFG+sVdLAitlpjAvD/I6dHDKnYrpEZUHkw=
golang.org/x/tools v0.19.0/go.mod h1:qoJWxmGSIBmAeriMx19ogtrEPrGtDbPK634QFIcLAhc=
modernc.org/cc/v4 v4.21.4 h1:3Be/Rdo1fpr8GrQ7IVw9OHtplU4gWbb+wNgeoBMmGLQ=
modernc.org/cc/v4 v4.21.4/go.mod h1:HM7VJTZbUCR3rV8EYBi9wxnJ0ZBRiGE5OeGXNA0IsLQ=
modernc.org/ccgo/v4 v4.19.2 h1:lwQZgvboKD0jBwdaeVCTouxhxAyN6iawF3STraAal8Y=
modernc.org/ccgo/v4 v4.19.2/go.mod h1:ysS3mxiMV38XGRTTcgo0DQTeTmAO4oCmJl1nX9VFI3s=
modernc.org/fileutil v1.3.0 h1:gQ5SIzK3H9kdfai/5x41oQiKValumqNTDXMvKo62HvE=
modernc.org/fileutil v1.3.0/go.mod h1:XatxS8fZi3pS8/hKG2GH/ArUogfxjpEKs3Ku3aK4JyQ=
modernc.org/gc/v2 v2.4.1 h1:9cNzOqPyMJBvrUipmynX0ZohMhcxPtMccYgGOJdOiBw=
modernc.org/gc/v2 v2.4.1/go.mod h1:wzN5dK1AzVGoH6XOzc3YZ+ey/jPgYHLuVckd62P0GYU=
modernc.org/libc v1.55.3 h1:AzcW1mhlPNrRtjS5sS+eW2ISCgSOLLNyFzRh/V3Qj/U=
modernc.org/libc v1.55.3/go.mod h1:qFXepLhz+JjFThQ4kzwzOjA/y/artDeg+pcYnY+Q83w=
modernc.org/mathutil v1.6.0 h1:fRe9+AmYlaej+64JsEEhoWuAYBkOtQiMEU7n/XgfYi4=
modernc.org/mathutil v1.6.0/go.mod h1:Ui5Q9q1TR2gFm0AQRqQUaBWFLAhQpCwNcuhBOSedWPo=
modernc.org/memory v1.8.0 h1:IqGTL6eFMaDZZhEWwcREgeMXYwmW83LYW8cROZYkg+E=
modernc.org/memory v1.8.0/go.mod h1:XPZ936zp5OMKGWPqbD3JShgd/ZoQ7899TUuQqxY+peU=
modernc.org/opt v0.1.3 h1:3XOZf2yznlhC+ibLltsDGzABUGVx8J6pnFMS3E4dcq4=
modernc.org/opt v0.1.3/go.mod h1:WdSiB5evDcignE70guQKxYUl14mgWtbClRi5wmkkTX0=
modernc.org/sortutil v1.2.0 h1:jQiD3PfS2REGJNzNCMMaLSp/wdMNieTbKX920Cqdgqc=
modernc.org/sortutil v1.2.0/go.mod h1:TKU2s7kJMf1AE84OoiGppNHJwvB753OYfNl2WRb++Ss=
modernc.org/sqlite v1.34.5 h1:Bb6SR13/fjp15jt70CL4f18JIN7p7dnMExd+UFnF15g=
modernc.org/sqlite v1.34.5/go.mod h1:YLuNmX9NKs8wRNK2ko1LW1NGYcc9FkBO69JOt1AR9JE=
modernc.org/strutil v1.2.0 h1:agBi9dp1I+eOnxXeiZawM8F4LawKv4NzGWSaLfyeNZA=
modernc.org/strutil v1.2.0/go.mod h1:/mdcBmfOibveCTBxUl5B5l6W+TTH1FXPLHZE6bTosX0=
modernc.org/token v1.1.0 h1:Xl7Ap9dKaEs5kLoOQeQmPWevfnk/DM5qcLcYlA8ys6Y=
modernc.org/token v1.1.0/go.mod h1:UGzOrNV1mAFSEB63lOFHIpNRUVMvYTc6yu1SMY/XTDM=
~~~

## `services/controld/internal/api/server.go`

- bytes: 18048
- segment: 1/1

~~~go
package api

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"html/template"
	"io/fs"
	"net/http"
	"strconv"
	"time"

	"github.com/lumelo/controld/internal/auth"
	"github.com/lumelo/controld/internal/libraryclient"
	"github.com/lumelo/controld/internal/playbackclient"
	"github.com/lumelo/controld/internal/provisioningclient"
	"github.com/lumelo/controld/internal/settings"
	"github.com/lumelo/controld/internal/sshctl"
)

type Dependencies struct {
	Auth             *auth.Service
	Playback         *playbackclient.Client
	Library          *libraryclient.Client
	Logs             LogSource
	Provisioning     ProvisioningSource
	Settings         settings.Config
	SSH              *sshctl.Controller
	Templates        fs.FS
	Static           fs.FS
	ArtworkCacheRoot string
}

type LogSource interface {
	Recent(ctx context.Context, lines int) (string, error)
}

type ProvisioningSource interface {
	Snapshot(ctx context.Context) provisioningclient.Snapshot
}

type Server struct {
	handler http.Handler
}

type homeViewData struct {
	CurrentPage        string
	Mode               string
	InterfaceMode      string
	DSDPolicy          string
	PasswordConfigured bool
	SSHEnabled         bool
	CommandSocket      string
	EventSocket        string
	LibraryDBPath      string
	ConfigPath         string
	PlaybackStatus     playbackclient.Status
	QueueSnapshot      playbackclient.QueueSnapshot
	QueueEntries       []queueEntryView
	CurrentOrderLabel  string
	CommandMessage     string
	CommandError       string
	SuggestedTrackID   string
	PlaybackStreamPath string
	Provisioning       provisioningclient.Snapshot
}

type libraryViewData struct {
	CurrentPage       string
	LibraryDBPath     string
	LibrarySnapshot   libraryclient.Snapshot
	PlaybackStatus    playbackclient.Status
	PlaybackScanBlock bool
	VolumeEntries     []libraryVolumeView
	AlbumEntries      []libraryAlbumView
	TrackEntries      []libraryTrackView
}

type logsViewData struct {
	CurrentPage string
	Lines       int
	LogText     string
	LogError    string
	LogTextPath string
}

type provisioningViewData struct {
	CurrentPage  string
	Provisioning provisioningclient.Snapshot
	RawJSON      string
}

type queueEntryView struct {
	DisplayIndex string
	QueueEntryID string
	TrackUID     string
	RelativePath string
	Title        string
	IsCurrent    bool
}

type libraryVolumeView struct {
	Label       string
	MountPath   string
	VolumeUUID  string
	LastSeenAt  string
	IsAvailable bool
}

type libraryAlbumView struct {
	Title           string
	AlbumArtist     string
	YearLabel       string
	TrackCount      int
	DurationLabel   string
	RootDirHint     string
	CoverThumbLabel string
	CoverThumbPath  string
}

type libraryTrackView struct {
	Title         string
	Artist        string
	RelativePath  string
	FormatLabel   string
	DurationLabel string
}

type healthView struct {
	Status                string `json:"status"`
	Mode                  string `json:"mode"`
	InterfaceMode         string `json:"interface_mode"`
	SSHEnabled            bool   `json:"ssh_enabled"`
	PlaybackAvailable     bool   `json:"playback_available"`
	PlaybackState         string `json:"playback_state,omitempty"`
	PlaybackError         string `json:"playback_error,omitempty"`
	LibraryAvailable      bool   `json:"library_available"`
	LibraryDBPath         string `json:"library_db_path"`
	LibraryError          string `json:"library_error,omitempty"`
	ProvisioningAvailable bool   `json:"provisioning_available"`
	ProvisioningState     string `json:"provisioning_state,omitempty"`
	ProvisioningMessage   string `json:"provisioning_message,omitempty"`
	ProvisioningReadError string `json:"provisioning_read_error,omitempty"`
}

const defaultLogLines = 300

func New(deps Dependencies) (*Server, error) {
	tmpl, err := template.ParseFS(deps.Templates, "templates/*.html")
	if err != nil {
		return nil, fmt.Errorf("parse templates: %w", err)
	}

	staticFS, err := fs.Sub(deps.Static, "static")
	if err != nil {
		return nil, fmt.Errorf("load static assets: %w", err)
	}

	logs := deps.Logs
	if logs == nil {
		logs = unavailableLogSource{}
	}
	provisioning := deps.Provisioning
	if provisioning == nil {
		provisioning = unavailableProvisioningSource{}
	}

	mux := http.NewServeMux()
	mux.Handle("/static/", http.StripPrefix("/static/", http.FileServer(http.FS(staticFS))))
	if deps.ArtworkCacheRoot != "" {
		mux.Handle("/artwork/", http.StripPrefix("/artwork/", http.FileServer(http.Dir(deps.ArtworkCacheRoot))))
	}

	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		playbackStatus := deps.Playback.Status(r.Context())
		librarySnapshot := deps.Library.Snapshot(r.Context())
		provisioningSnapshot := provisioning.Snapshot(r.Context())
		response := healthView{
			Status:                "ok",
			Mode:                  deps.Settings.Mode,
			InterfaceMode:         deps.Settings.InterfaceMode,
			SSHEnabled:            deps.SSH.Enabled(),
			PlaybackAvailable:     playbackStatus.Available,
			PlaybackState:         playbackStatus.State,
			PlaybackError:         playbackStatus.Error,
			LibraryAvailable:      librarySnapshot.Available,
			LibraryDBPath:         deps.Library.LibraryDBPath,
			LibraryError:          librarySnapshot.Error,
			ProvisioningAvailable: provisioningSnapshot.Available,
			ProvisioningState:     provisioningSnapshot.State,
			ProvisioningMessage:   provisioningSnapshot.Message,
			ProvisioningReadError: provisioningSnapshot.ReadError,
		}

		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		if err := json.NewEncoder(w).Encode(response); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	renderHome := func(w http.ResponseWriter, r *http.Request, commandMessage, commandError string) {
		status := deps.Playback.Status(r.Context())
		queueSnapshot := deps.Playback.QueueSnapshot(r.Context())
		librarySnapshot := deps.Library.Snapshot(r.Context())
		provisioningSnapshot := provisioning.Snapshot(r.Context())

		data := homeViewData{
			CurrentPage:        "home",
			Mode:               deps.Settings.Mode,
			InterfaceMode:      deps.Settings.InterfaceMode,
			DSDPolicy:          deps.Settings.DSDPolicy,
			PasswordConfigured: deps.Auth.PasswordConfigured(),
			SSHEnabled:         deps.SSH.Enabled(),
			CommandSocket:      deps.Playback.CommandSocket,
			EventSocket:        deps.Playback.EventSocket,
			LibraryDBPath:      deps.Library.LibraryDBPath,
			ConfigPath:         deps.Settings.ConfigPath,
			PlaybackStatus:     status,
			QueueSnapshot:      queueSnapshot,
			QueueEntries:       buildQueueEntryViews(queueSnapshot),
			CurrentOrderLabel:  currentOrderLabel(queueSnapshot),
			CommandMessage:     commandMessage,
			CommandError:       commandError,
			SuggestedTrackID:   suggestedTrackID(status, librarySnapshot),
			PlaybackStreamPath: "/events/playback",
			Provisioning:       provisioningSnapshot,
		}

		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if err := tmpl.ExecuteTemplate(w, "index.html", data); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" {
			http.NotFound(w, r)
			return
		}
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		renderHome(w, r, "", "")
	})

	renderLibrary := func(w http.ResponseWriter, r *http.Request) {
		snapshot := deps.Library.Snapshot(r.Context())
		playbackStatus := deps.Playback.Status(r.Context())

		data := libraryViewData{
			CurrentPage:       "library",
			LibraryDBPath:     deps.Library.LibraryDBPath,
			LibrarySnapshot:   snapshot,
			PlaybackStatus:    playbackStatus,
			PlaybackScanBlock: playbackBlocksScan(playbackStatus),
			VolumeEntries:     buildLibraryVolumeViews(snapshot),
			AlbumEntries:      buildLibraryAlbumViews(snapshot),
			TrackEntries:      buildLibraryTrackViews(snapshot),
		}

		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if err := tmpl.ExecuteTemplate(w, "library.html", data); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/library", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		renderLibrary(w, r)
	})

	renderLogs := func(w http.ResponseWriter, r *http.Request) {
		lines := parseLogLines(r)
		logText, err := logs.Recent(r.Context(), lines)
		data := logsViewData{
			CurrentPage: "logs",
			Lines:       lines,
			LogText:     logText,
			LogTextPath: fmt.Sprintf("/logs.txt?lines=%d", lines),
		}
		if err != nil {
			data.LogError = err.Error()
		}

		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if err := tmpl.ExecuteTemplate(w, "logs.html", data); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/logs", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		renderLogs(w, r)
	})

	mux.HandleFunc("/logs.txt", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		lines := parseLogLines(r)
		logText, err := logs.Recent(r.Context(), lines)
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		if err != nil {
			_, _ = fmt.Fprintf(w, "log read error: %v\n\n", err)
		}
		_, _ = fmt.Fprint(w, logText)
	})

	renderProvisioning := func(w http.ResponseWriter, r *http.Request) {
		snapshot := provisioning.Snapshot(r.Context())
		rawJSON, err := json.MarshalIndent(snapshot, "", "  ")
		if err != nil {
			rawJSON = []byte(fmt.Sprintf("{\"read_error\":%q}", err.Error()))
		}

		data := provisioningViewData{
			CurrentPage:  "provisioning",
			Provisioning: snapshot,
			RawJSON:      string(rawJSON),
		}

		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if err := tmpl.ExecuteTemplate(w, "provisioning.html", data); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	}

	mux.HandleFunc("/provisioning", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		renderProvisioning(w, r)
	})

	mux.HandleFunc("/provisioning-status", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		if err := json.NewEncoder(w).Encode(provisioning.Snapshot(r.Context())); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
		}
	})

	mux.HandleFunc("/commands", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		if err := r.ParseForm(); err != nil {
			renderHome(w, r, "", fmt.Sprintf("parse command form: %v", err))
			return
		}

		action := r.Form.Get("action")
		trackID := r.Form.Get("track_id")
		message, err := deps.Playback.Execute(r.Context(), action, trackID)
		if err != nil {
			renderHome(w, r, "", err.Error())
			return
		}

		renderHome(w, r, message, "")
	})

	mux.HandleFunc("/events/playback", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}

		flusher, ok := w.(http.Flusher)
		if !ok {
			http.Error(w, "streaming unsupported", http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "text/event-stream")
		w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
		w.Header().Set("Connection", "keep-alive")
		w.Header().Set("X-Accel-Buffering", "no")

		_, _ = fmt.Fprint(w, ": lumelo playback stream\n\n")
		flusher.Flush()

		ctx := r.Context()
		eventCh := make(chan playbackclient.Event, 8)
		errCh := make(chan error, 1)

		go func() {
			errCh <- deps.Playback.SubscribeEvents(ctx, func(event playbackclient.Event) error {
				select {
				case eventCh <- event:
					return nil
				case <-ctx.Done():
					return ctx.Err()
				}
			})
		}()

		keepAlive := time.NewTicker(20 * time.Second)
		defer keepAlive.Stop()

		for {
			select {
			case <-ctx.Done():
				return
			case event := <-eventCh:
				payload, err := json.Marshal(event)
				if err != nil {
					http.Error(w, fmt.Sprintf("marshal event: %v", err), http.StatusInternalServerError)
					return
				}
				if _, err := fmt.Fprintf(w, "event: %s\ndata: %s\n\n", event.Name, payload); err != nil {
					return
				}
				flusher.Flush()
			case err := <-errCh:
				if err == nil || errors.Is(err, context.Canceled) {
					return
				}
				payload, _ := json.Marshal(map[string]string{"message": err.Error()})
				if _, writeErr := fmt.Fprintf(w, "event: STREAM_ERROR\ndata: %s\n\n", payload); writeErr != nil {
					return
				}
				flusher.Flush()
				return
			case <-keepAlive.C:
				if _, err := fmt.Fprint(w, ": keepalive\n\n"); err != nil {
					return
				}
				flusher.Flush()
			}
		}
	})

	return &Server{handler: mux}, nil
}

func (s *Server) Handler() http.Handler {
	return s.handler
}

func suggestedTrackID(status playbackclient.Status, snapshot libraryclient.Snapshot) string {
	if status.CurrentTrack != "" {
		return status.CurrentTrack
	}
	if len(snapshot.Tracks) > 0 && snapshot.Tracks[0].TrackUID != "" {
		return snapshot.Tracks[0].TrackUID
	}

	return "demo-track-001"
}

func playbackBlocksScan(status playbackclient.Status) bool {
	return status.State == "pre_quiet" || status.State == "quiet_active"
}

func parseLogLines(r *http.Request) int {
	raw := r.URL.Query().Get("lines")
	if raw == "" {
		return defaultLogLines
	}

	lines, err := strconv.Atoi(raw)
	if err != nil {
		return defaultLogLines
	}
	if lines < 50 {
		return 50
	}
	if lines > 1000 {
		return 1000
	}

	return lines
}

type unavailableLogSource struct{}

func (unavailableLogSource) Recent(context.Context, int) (string, error) {
	return "", errors.New("log source is not configured")
}

type unavailableProvisioningSource struct{}

func (unavailableProvisioningSource) Snapshot(context.Context) provisioningclient.Snapshot {
	return provisioningclient.Snapshot{
		ReadError: "provisioning source is not configured",
	}
}

func buildQueueEntryViews(snapshot playbackclient.QueueSnapshot) []queueEntryView {
	views := make([]queueEntryView, 0, len(snapshot.Entries))
	for _, entry := range snapshot.Entries {
		title := entry.TrackUID
		if entry.Title != nil && *entry.Title != "" {
			title = *entry.Title
		}
		if title == "" {
			title = entry.TrackUID
		}

		views = append(views, queueEntryView{
			DisplayIndex: fmt.Sprintf("%02d", entry.OrderIndex+1),
			QueueEntryID: entry.QueueEntryID,
			TrackUID:     entry.TrackUID,
			RelativePath: entry.RelativePath,
			Title:        title,
			IsCurrent:    entry.IsCurrent,
		})
	}

	return views
}

func currentOrderLabel(snapshot playbackclient.QueueSnapshot) string {
	if snapshot.CurrentOrderIndex == nil {
		return "-"
	}

	return fmt.Sprintf("%d", *snapshot.CurrentOrderIndex)
}

func buildLibraryVolumeViews(snapshot libraryclient.Snapshot) []libraryVolumeView {
	views := make([]libraryVolumeView, 0, len(snapshot.Volumes))
	for _, volume := range snapshot.Volumes {
		views = append(views, libraryVolumeView{
			Label:       volume.Label,
			MountPath:   volume.MountPath,
			VolumeUUID:  volume.VolumeUUID,
			LastSeenAt:  fmt.Sprintf("%d", volume.LastSeenAt),
			IsAvailable: volume.IsAvailable,
		})
	}

	return views
}

func buildLibraryAlbumViews(snapshot libraryclient.Snapshot) []libraryAlbumView {
	views := make([]libraryAlbumView, 0, len(snapshot.Albums))
	for _, album := range snapshot.Albums {
		coverThumbLabel := fallback(album.CoverThumbRelPath, "-")
		coverThumbPath := ""
		if album.CoverThumbRelPath != "" {
			coverThumbPath = "/artwork/" + album.CoverThumbRelPath
		}
		views = append(views, libraryAlbumView{
			Title:           album.Title,
			AlbumArtist:     album.AlbumArtist,
			YearLabel:       intLabel(album.Year),
			TrackCount:      album.TrackCount,
			DurationLabel:   durationMSLabel(album.TotalDurationMS),
			RootDirHint:     fallback(album.RootDirHint, "-"),
			CoverThumbLabel: coverThumbLabel,
			CoverThumbPath:  coverThumbPath,
		})
	}

	return views
}

func buildLibraryTrackViews(snapshot libraryclient.Snapshot) []libraryTrackView {
	views := make([]libraryTrackView, 0, len(snapshot.Tracks))
	for _, track := range snapshot.Tracks {
		views = append(views, libraryTrackView{
			Title:         track.Title,
			Artist:        track.Artist,
			RelativePath:  track.RelativePath,
			FormatLabel:   formatTrackFormat(track),
			DurationLabel: pointerDurationMSLabel(track.DurationMS),
		})
	}

	return views
}

func durationMSLabel(durationMS int64) string {
	if durationMS <= 0 {
		return "-"
	}

	totalSeconds := durationMS / 1000
	minutes := totalSeconds / 60
	seconds := totalSeconds % 60
	return fmt.Sprintf("%d:%02d", minutes, seconds)
}

func pointerDurationMSLabel(durationMS *int64) string {
	if durationMS == nil {
		return "-"
	}

	return durationMSLabel(*durationMS)
}

func formatTrackFormat(track libraryclient.TrackSummary) string {
	if track.Format == "" && track.SampleRate == nil {
		return "-"
	}
	if track.SampleRate == nil {
		return track.Format
	}
	if track.Format == "" {
		return fmt.Sprintf("%d Hz", *track.SampleRate)
	}

	return fmt.Sprintf("%s · %d Hz", track.Format, *track.SampleRate)
}

func intLabel(value int) string {
	if value <= 0 {
		return "-"
	}

	return fmt.Sprintf("%d", value)
}

func fallback(value, fallbackValue string) string {
	if value == "" {
		return fallbackValue
	}

	return value
}
~~~

