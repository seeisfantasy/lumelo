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
