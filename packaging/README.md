# Packaging

`packaging/` is reserved for build and release artifacts:

- `image/`: image assembly inputs and helpers
- `recovery/`: password reset and recovery-media flow assets
- `update/`: offline update packaging
- `systemd/`: packaging notes for installing/enabling units from the rootfs

The authoritative unit files currently live under
`base/rootfs/overlay/etc/systemd/system/`.
