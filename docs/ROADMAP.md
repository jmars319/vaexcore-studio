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
- Sidecar startup supervision with dry-run fallback.
- Sidecar dry-run command transport.
- SQLite schema migration tracking.
- API request ID headers and bounded WebSocket replay.
- Recent client registry.
- Bounded command audit log.

## Next Milestone: Runtime Hardening

- API port fallback and frontend discovery. Done.
- Structured app log file rotation. Done.
- Import/export profile bundle. Done.
- Sidecar restart policy after crashes. Done.
- macOS preflight checks. Done for API, token, output folder, sidecar, screen recording, and source-gated placeholders.
- Capture source model. Done with display, window, camera, microphone, and system-audio source kinds.
- Dry-run media pipeline planning and validation. Done through API and sidecar contracts.

## Real Media Engine Milestone

- GStreamer pipeline builder behind `MediaEngine`.
- Camera and microphone authorization checks through AVFoundation.
- Audio/video source enumeration.
- Encoder and muxer capability discovery.
- Recording container safety checks.
- RTMP/RTMPS connection status events.
- Real pipeline execution behind the existing `media-runner` command transport.
- System audio capture strategy for macOS.

## Platform Milestone

- Better ingest presets.
- Per-platform validation rules.
- Optional OAuth helpers outside the media core.
- macOS Keychain storage for Studio stream keys. Done.
- Windows Credential Manager storage for Studio stream keys.

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
