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
