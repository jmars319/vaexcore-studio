# Roadmap

## MVP

- Tauri v2 desktop shell.
- React + TypeScript control surface.
- SQLite-backed profiles and destinations.
- Local HTTP API.
- WebSocket event stream.
- Dry-run recording and streaming lifecycle.
- Manual marker events.
- Replaceable `media-runner` sidecar scaffold.

## Next Milestone: Runtime Hardening

- API port fallback and frontend discovery.
- More detailed command audit log.
- Recent client registry.
- Per-command request IDs.
- Structured app log file rotation.
- Import/export profile bundle.

## Real Media Engine Milestone

- GStreamer pipeline builder behind `MediaEngine`.
- macOS capture permission checks.
- Audio/video source enumeration.
- Encoder and muxer capability discovery.
- Recording container safety checks.
- RTMP/RTMPS connection status events.
- Sidecar health supervision from the API process.

## Platform Milestone

- Better ingest presets.
- Per-platform validation rules.
- Optional OAuth helpers outside the media core.
- Secret migration to OS keychain where available.

## External App Milestone

- Typed client SDK.
- WebSocket reconnect examples.
- Stream deck bridge sample.
- Bot integration sample.
- Overlay event sample.

## Explicit Non-Goals

- Twitch chat integration.
- Giveaways.
- Moderation.
- Highlight detection.
- OBS-style scene editor.
- Plugin marketplace.
- Cloud dependency.
- Mobile app.

