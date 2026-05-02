# vaexcore studio

`vaexcore studio` is a macOS-first desktop control system for streaming, local recording, and localhost integrations. It is designed as the foundation layer that Twitch bots, highlight locators, stream deck tools, and future overlay systems can trust.

It is not a giveaway, moderation, highlight detection, scene editing, cloud, or plugin marketplace app.

## Stack

- Tauri v2 desktop shell
- React + TypeScript frontend
- Rust core/API/media crates
- SQLite for local profiles, markers, and secret references
- Local HTTP + WebSocket API
- Dry-run media engine behind a media abstraction
- Supervised, replaceable `media-runner` sidecar path with dry-run fallback

## Repository Layout

```text
apps/
  desktop/                 Tauri v2 + React app
crates/
  vaexcore-core/           shared Rust contracts, profiles, events, responses
  vaexcore-api/            localhost HTTP + WebSocket API
  vaexcore-media/          media traits and dry-run engine
  vaexcore-platforms/      Twitch, YouTube, Kick, custom RTMP definitions
packages/
  shared-types/            TypeScript API/event contracts
sidecars/
  media-runner/            replaceable media execution sidecar
docs/
  ARCHITECTURE.md
  API.md
  MEDIA_ENGINE.md
  ROADMAP.md
```

## Setup

Prerequisites:

- Node.js 20+
- Rust 1.82+
- Xcode command line tools on macOS

Install dependencies:

```bash
npm install
```

Run the desktop app in development:

```bash
npm run tauri -w apps/desktop -- dev
```

Run checks:

```bash
npm run typecheck
npm run build
cargo test --workspace
```

Build the sidecar executable for local desktop supervision:

```bash
cargo build -p vaexcore-media-runner
```

The desktop app auto-detects `target/debug/media-runner` or `target/release/media-runner` when present. You can also point directly at a sidecar executable:

```bash
VAEXCORE_MEDIA_RUNNER_PATH=/absolute/path/to/media-runner npm run tauri -w apps/desktop -- dev
```

Run the sidecar dry-run status service manually:

```bash
cargo run -p vaexcore-media-runner -- --status-addr 127.0.0.1:51387 --dry-run
```

## Local API

The desktop process starts the local API on:

```text
http://127.0.0.1:51287
ws://127.0.0.1:51287/events
```

In debug builds, auth bypass is enabled by default. For token-protected local clients, set:

```bash
VAEXCORE_DEV_AUTH_BYPASS=0
VAEXCORE_API_TOKEN=replace-with-a-local-token
```

Example:

```bash
curl http://127.0.0.1:51287/health
curl -H "x-vaexcore-token: replace-with-a-local-token" http://127.0.0.1:51287/status
```

## MVP Behavior

- Create Twitch, YouTube, Kick, and custom RTMP stream destinations.
- Create recording profiles with output path, filename pattern, container, resolution, framerate, bitrate, and encoder preference.
- Start/stop recording and streaming independently or together.
- Create manual markers.
- Stream lifecycle events over WebSocket.
- Simulate media execution with `DryRunMediaEngine`.
- Prefer supervised `media-runner` dry-run execution when the sidecar is available, and fall back to in-process dry-run when it is missing.

## Security Notes

- Stream keys are accepted as sensitive inputs and stored behind `SecretStore`.
- API responses expose secret references, not raw stream keys.
- Stream keys are never included in media events or status payloads.
- The API is localhost-only by default.
