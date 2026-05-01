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
expect_text /etc/systemd/network/20-wired-dhcp.network "MulticastDNS=yes" "wired DHCP mDNS policy"
expect_text /etc/systemd/network/20-wired-dhcp.network "ClientIdentifier=mac" "wired DHCP client id"
expect_text /etc/systemd/network/30-wireless-dhcp.network "LinkLocalAddressing=no" "wireless DHCP link-local policy"
expect_text /etc/systemd/network/30-wireless-dhcp.network "LLMNR=no" "wireless DHCP LLMNR policy"
expect_text /etc/systemd/network/30-wireless-dhcp.network "MulticastDNS=yes" "wireless DHCP mDNS policy"
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
expect_text /etc/systemd/resolved.conf.d/lumelo.conf "MulticastDNS=yes" "resolved mDNS policy"
expect_file /etc/systemd/dnssd/lumelo-http.dnssd "Lumelo HTTP DNS-SD service"
expect_text /etc/systemd/dnssd/lumelo-http.dnssd "Type=_http._tcp" "Lumelo HTTP DNS-SD type"
expect_text /etc/systemd/dnssd/lumelo-http.dnssd "Port=80" "Lumelo HTTP DNS-SD port"
expect_text /etc/systemd/system/controld.service "CONTROLD_LISTEN_ADDR=0.0.0.0:80" "controld bring-up listen address"
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
expect_file /etc/systemd/system/bluetooth.service.d/30-lumelo-compat.conf "bluetooth compat drop-in"
expect_text /etc/systemd/system/bluetooth.service.d/30-lumelo-compat.conf "ExecStart=/usr/libexec/bluetooth/bluetoothd -C" "bluetooth compat ExecStart override"
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
  expect_text \
    /usr/bin/lumelo-bluetooth-provisioning-mode \
    'ACTION=${1:-enable}' \
    "Bluetooth provisioning helper action selector"
  expect_text \
    /usr/bin/lumelo-bluetooth-provisioning-mode \
    'btmgmt discov no' \
    "Bluetooth provisioning helper disable path"
  expect_text \
    /usr/bin/lumelo-bluetooth-provisioning-mode \
    'btmgmt pairable off' \
    "Bluetooth provisioning helper pairable disable path"
  expect_text \
    /usr/bin/lumelo-bluetooth-provisioning-mode \
    'btmgmt pairable on' \
    "Bluetooth provisioning helper pairable enable path"
  expect_text \
    /usr/bin/lumelo-bluetooth-provisioning-mode \
    'btmgmt discov yes 0' \
    "Bluetooth provisioning helper discoverable enable path"
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
  expect_text \
    /etc/systemd/system/lumelo-bluetooth-provisioning.service \
    "PartOf=lumelo-wifi-provisiond.service" \
    "Bluetooth provisioning lifecycle coupling"
  expect_text \
    /etc/systemd/system/lumelo-bluetooth-provisioning.service \
    "ExecStop=/usr/bin/lumelo-bluetooth-provisioning-mode disable" \
    "Bluetooth provisioning disable stop path"
else
  warn "Bluetooth provisioning unit not present; expected only in images rebuilt after 2026-04-08 02:55"
fi

if [ -x "${ROOTFS_MOUNT}/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond" ]; then
  pass "Classic Bluetooth Wi-Fi provisioning daemon: /usr/libexec/lumelo/classic-bluetooth-wifi-provisiond"
  expect_text \
    /usr/libexec/lumelo/classic-bluetooth-wifi-provisiond \
    "def cleanup_sdp_records_main():" \
    "Classic provisioning SDP cleanup entrypoint"
  expect_text \
    /usr/libexec/lumelo/classic-bluetooth-wifi-provisiond \
    "classic_bluetooth_socket_bind_failed" \
    "Classic provisioning bind failure code"
  expect_text \
    /usr/libexec/lumelo/classic-bluetooth-wifi-provisiond \
    "classic_bluetooth_sdp_registration_failed" \
    "Classic provisioning SDP failure code"
else
  warn "Classic Bluetooth Wi-Fi provisioning daemon not present; expected only in images rebuilt after 2026-04-12 16:30"
fi

if [ -f "${ROOTFS_MOUNT}/etc/systemd/system/lumelo-wifi-provisiond.service" ]; then
  pass "Bluetooth Wi-Fi provisioning unit: /etc/systemd/system/lumelo-wifi-provisiond.service"
  expect_text \
    /etc/systemd/system/lumelo-wifi-provisiond.service \
    "BindsTo=lumelo-bluetooth-provisioning.service" \
    "Classic provisioning service lifecycle binding"
  expect_text \
    /etc/systemd/system/lumelo-wifi-provisiond.service \
    "ExecStopPost=/usr/libexec/lumelo/classic-bluetooth-wifi-provisiond --cleanup-sdp" \
    "Classic provisioning SDP cleanup stop hook"
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
