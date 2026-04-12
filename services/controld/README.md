# Controld

`controld` is the Go control-plane service for V1.

This skeleton keeps the scope narrow:

- embedded SSR templates and static files
- a tiny HTTP server entrypoint
- placeholder internal packages for auth, settings, SSH, playback IPC, and
  library access

Real playback commands, auth storage, and settings persistence will be layered
in on top of this structure.
