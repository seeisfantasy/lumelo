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
