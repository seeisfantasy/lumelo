# Image Packaging

Current first target:

- `T4 smoke image`
- boot from `TF` on `FriendlyELEC NanoPC-T4`
- validate `network + SSH + Lumelo services + placeholder WebUI + library scan`
- do **not** block on real audio output before first flash

Locked smoke base:

- official `FriendlyELEC NanoPC-T4` SD image family:
  - `rk3399-sd-debian-trixie-core-4.19-arm64-YYYYMMDD.img.gz`
- lock metadata:
  - [t4-smoke-base.toml](/Volumes/SeeDisk/Codex/Lumelo/packaging/image/t4-smoke-base.toml)

Build entrypoint:

- [build-t4-smoke-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/build-t4-smoke-image.sh)

Current smoke builder strategy:

- reuse the official FriendlyELEC boot chain and partition layout
- remaster only the `rootfs` partition of the selected SD image
- inject `Lumelo` binaries plus `base/rootfs/overlay`
- enable `lumelo-mode-manager.service`
- let mode manager start `local-mode.target` or the V1 `bridge-mode.target` placeholder from `/etc/lumelo/config.toml`
- enable wired DHCP via `systemd-networkd` when available

This is intentionally a bring-up shortcut for the first TF image.
The long-term V1 direction remains:

- `FriendlyELEC` board support for `kernel / dtb / u-boot`
- `Lumelo`-owned rootfs and upper-layer services

First `Lumelo-defined rootfs` builder:

- lock metadata:
  - [t4-lumelo-rootfs-base.toml](/Volumes/SeeDisk/Codex/Lumelo/packaging/image/t4-lumelo-rootfs-base.toml)
- package manifest:
  - [t4-bringup-packages.txt](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/manifests/t4-bringup-packages.txt)
- post-build hook:
  - [t4-bringup-postbuild.sh](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/hooks/t4-bringup-postbuild.sh)
- bring-up report tool:
  - [lumelo-t4-report](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-t4-report)
- manual ALSA smoke helper:
  - [lumelo-audio-smoke](/Volumes/SeeDisk/Codex/Lumelo/base/rootfs/overlay/usr/bin/lumelo-audio-smoke)
- build entrypoint:
  - [build-t4-lumelo-rootfs-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/build-t4-lumelo-rootfs-image.sh)
- SSH bring-up wrapper:
  - [build-t4-ssh-bringup-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/build-t4-ssh-bringup-image.sh)
- offline verifier:
  - [verify-t4-lumelo-rootfs-image.sh](/Volumes/SeeDisk/Codex/Lumelo/scripts/verify-t4-lumelo-rootfs-image.sh)

Current custom-rootfs builder strategy:

- recreate the GPT layout instead of remastering the official rootfs
- copy the FriendlyELEC pre-partition RK3399 loader area before `p1`
- copy FriendlyELEC `p1-p7` board-support partitions into a new image
- create a smaller `p8 rootfs` and `p9 userdata`
- bootstrap `Debian trixie` with `mmdebstrap`
- inject matching FriendlyELEC kernel modules and firmware from the official image
- inject `Lumelo` binaries plus `base/rootfs/overlay`
- enable SSH by default for development / bring-up images
- allow direct `root/root` SSH login during the current debug phase
- keep `SSH_AUTHORIZED_KEYS_FILE=/path/to/id.pub` as an optional key-injection path
- require release-like images to set `ENABLE_SSH=0` explicitly
- provide manual ALSA diagnostics only; do not add an automatic audio smoke
  service
- validate each generated image with the read-only `p8` verifier before T4
  hardware bring-up
- write a sibling `.sha256` file after each successful image build
