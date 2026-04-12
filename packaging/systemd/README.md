# Systemd Packaging Notes

The first-pass service and target units live in:

- `base/rootfs/overlay/etc/systemd/system/`

Keep packaging-specific enablement logic here so the source-of-truth unit files
do not drift from the rootfs overlay.
