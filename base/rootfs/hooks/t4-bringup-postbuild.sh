#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: t4-bringup-postbuild.sh <rootfs-dir>" >&2
  exit 1
fi

ROOTFS_DIR=$1
PROFILE=${LUMELO_IMAGE_PROFILE:-t4-bringup}
BOARD_SOURCE_IMAGE=${BOARD_SOURCE_IMAGE:-unknown}
ENABLE_SSH=${ENABLE_SSH:-0}
SSH_AUTHORIZED_KEYS_FILE=${SSH_AUTHORIZED_KEYS_FILE:-}
ROOT_PASSWORD=${ROOT_PASSWORD:-}
ssh_enabled_value=false
root_password_set=0
dev_sshd_config_path="${ROOTFS_DIR}/etc/ssh/sshd_config.d/90-lumelo-development.conf"

unit_dir=
for candidate in /usr/lib/systemd/system /lib/systemd/system; do
  if [ -d "${ROOTFS_DIR}${candidate}" ]; then
    unit_dir=$candidate
    break
  fi
done

mkdir -p "${ROOTFS_DIR}/etc/lumelo"
printf '%s\n' "lumelo" > "${ROOTFS_DIR}/etc/hostname"
cat > "${ROOTFS_DIR}/etc/hosts" <<'EOF'
127.0.0.1 localhost
127.0.1.1 lumelo

::1 localhost ip6-localhost ip6-loopback
ff02::1 ip6-allnodes
ff02::2 ip6-allrouters
EOF

cat > "${ROOTFS_DIR}/etc/fstab" <<'EOF'
# Lumelo-defined rootfs image.
# The board boot chain still provides the kernel command line for root=/dev/mmcblk?p8.
EOF

rm -f "${ROOTFS_DIR}/etc/resolv.conf"
ln -sf /run/systemd/resolve/resolv.conf "${ROOTFS_DIR}/etc/resolv.conf"

mkdir -p "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants"
rm -f "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/local-mode.target"
rm -f "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/bridge-mode.target"
ln -sf ../lumelo-mode-manager.service \
  "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/lumelo-mode-manager.service"

if [ -n "${unit_dir}" ]; then
  for unit in systemd-networkd.service systemd-resolved.service systemd-timesyncd.service; do
    if [ -f "${ROOTFS_DIR}${unit_dir}/${unit}" ]; then
      ln -sf "${unit_dir}/${unit}" \
        "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/${unit}"
    fi
  done

  if [ -f "${ROOTFS_DIR}/etc/systemd/system/lumelo-bluetooth-uart-attach.service" ]; then
    ln -sf ../lumelo-bluetooth-uart-attach.service \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/lumelo-bluetooth-uart-attach.service"
  fi

  if [ -f "${ROOTFS_DIR}${unit_dir}/bluetooth.service" ]; then
    ln -sf "${unit_dir}/bluetooth.service" \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/bluetooth.service"
  fi

  if [ -f "${ROOTFS_DIR}/etc/systemd/system/lumelo-bluetooth-provisioning.service" ]; then
    ln -sf ../lumelo-bluetooth-provisioning.service \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/lumelo-bluetooth-provisioning.service"
  fi

  if [ -f "${ROOTFS_DIR}/etc/systemd/system/lumelo-wifi-provisiond.service" ]; then
    ln -sf ../lumelo-wifi-provisiond.service \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/lumelo-wifi-provisiond.service"
  fi

  if [ "${ENABLE_SSH}" = "1" ] && [ -f "${ROOTFS_DIR}${unit_dir}/ssh.service" ]; then
    ln -sf "${unit_dir}/ssh.service" \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/ssh.service"
    ssh_enabled_value=true

    install -d -m 0755 "$(dirname "${dev_sshd_config_path}")"
    cat > "${dev_sshd_config_path}" <<'EOF'
# Lumelo development / bring-up images allow direct root SSH debugging.
PermitRootLogin yes
PasswordAuthentication yes
EOF

    if [ -n "${SSH_AUTHORIZED_KEYS_FILE}" ]; then
      install -d -m 0700 "${ROOTFS_DIR}/root/.ssh"
      install -m 0600 "${SSH_AUTHORIZED_KEYS_FILE}" \
        "${ROOTFS_DIR}/root/.ssh/authorized_keys"
    fi
  else
    rm -f "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/ssh.service"
    rm -f "${dev_sshd_config_path}"
    rm -rf "${ROOTFS_DIR}/root/.ssh"
  fi
fi

config_path="${ROOTFS_DIR}/etc/lumelo/config.toml"
if [ -f "${config_path}" ]; then
  sed -i "s/^ssh_enabled = .*/ssh_enabled = ${ssh_enabled_value}/" "${config_path}"
fi

rm -f "${ROOTFS_DIR}/etc/ssh/ssh_host_"*

if [ -n "${ROOT_PASSWORD}" ]; then
  if [ -x "${ROOTFS_DIR}/usr/sbin/chpasswd" ]; then
    printf 'root:%s\n' "${ROOT_PASSWORD}" | chroot "${ROOTFS_DIR}" /usr/sbin/chpasswd
    root_password_set=1
  else
    echo "ROOT_PASSWORD was provided, but chpasswd is not available in rootfs" >&2
    exit 1
  fi
fi

: > "${ROOTFS_DIR}/etc/machine-id"
mkdir -p "${ROOTFS_DIR}/var/lib/dbus"
ln -sf /etc/machine-id "${ROOTFS_DIR}/var/lib/dbus/machine-id"

rm -rf "${ROOTFS_DIR}/var/cache/apt/archives/"*.deb
rm -rf "${ROOTFS_DIR}/var/lib/apt/lists/"*

cat > "${ROOTFS_DIR}/etc/lumelo/image-build.txt" <<EOF
Lumelo-defined rootfs image profile: ${PROFILE}
Board support source: ${BOARD_SOURCE_IMAGE}
Built at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
SSH enabled in image: ${ENABLE_SSH}
SSH authorized_keys injected: $(if [ -n "${SSH_AUTHORIZED_KEYS_FILE}" ]; then printf '1'; else printf '0'; fi)
Root console password set: ${root_password_set}
EOF
