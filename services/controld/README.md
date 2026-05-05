# Controld

`controld` is the Go control-plane service for `Lumelo V1`.

Current scope:

- embedded SSR templates and static files
- browser-facing JSON / SSE endpoints
- playback command / event bridge
- direct `library.db` read model
- provisioning status and logs
- thin auth / settings / SSH summary

Current boundary:

- `playbackd` remains the playback and queue authority
- `sessiond` remains the Quiet Mode authority
- `controld` is the WebUI / API adapter
- `controld` should not own queue semantics

Current Web direction:

- keep `SSR + small native JS`
- stabilize `/api/v1/...` in the same process and port
- support repeated UI redesigns without changing Rust service semantics
- do not introduce a separate frontend daemon or SPA toolchain

Phase 1 first batch:

- `GET /api/v1/system/summary`
- `GET /api/v1/system/health`
- `GET /api/v1/system/audio-output`
- `GET /api/v1/provisioning/status`
- `GET /api/v1/playback/status`
- `GET /api/v1/playback/queue`
- `GET /api/v1/playback/events`
- `GET /api/v1/library/snapshot`

Phase 2 current batch:

- `POST /api/v1/playback/commands`
- `POST /api/v1/library/commands`
- legacy form routes still exist, but now share the same command execution path
- home and `Library` forms now prefer the new command API and keep form actions only as fallback
- `Library` list sections now also prefer browser-side render from `GET /api/v1/library/snapshot`
- `Library` browse links now also prefer browser-side navigation with `pushState / popstate` and API refresh
- `Library` page now receives a stable snapshot API base path; current query comes from browser URL / history
- SSR first paint and no-JS fallback still remain in place

Current detailed contract:

- [docs/Product_Development_Manual.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Product_Development_Manual.md), especially `controld` boundary and `API 与服务 Contract`
- [docs/Development_Progress_Log.md](/Volumes/SeeDisk/Codex/Lumelo/docs/Development_Progress_Log.md) for current implementation status
