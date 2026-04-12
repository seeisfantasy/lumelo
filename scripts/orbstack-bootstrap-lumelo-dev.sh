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
