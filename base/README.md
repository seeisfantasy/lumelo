# Base Layer

`base/` holds the system-facing part of Lumelo:

- board support placeholders for the FriendlyELEC T4 line
- rootfs overlay files
- systemd units and targets
- boot/build hooks and manifests

This layer should stay focused on image construction and runtime integration,
not application logic.
