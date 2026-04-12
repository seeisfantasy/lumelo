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
