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
