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
- Supervised, replaceable `media-runner` sidecar with HTTP command transport
- Recent client registry and command audit log for localhost integrations

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
  client-sdk/              TypeScript client for localhost integrations
sidecars/
  media-runner/            replaceable media execution sidecar
docs/
  ARCHITECTURE.md
  API.md
  MEDIA_ENGINE.md
  RELEASE.md
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
npm run test:scripts
npm run prepare:sidecars
npm run check:sidecars
npm run typecheck
npm run build
cargo test --workspace
```

Build and run the TypeScript client SDK smoke example after Studio is running:

```bash
npm run build -w @vaexcore/client-sdk
node packages/client-sdk/examples/node-smoke.mjs
```

Build and stage the sidecar executable for local desktop supervision and release bundling:

```bash
npm run prepare:sidecars -w apps/desktop
npm run check:sidecars -w apps/desktop
```

The desktop app first checks bundled sidecar locations, then falls back to local build artifacts like `target/debug/media-runner` or `target/release/media-runner`. You can also point directly at a sidecar executable:

```bash
VAEXCORE_MEDIA_RUNNER_PATH=/absolute/path/to/media-runner npm run tauri -w apps/desktop -- dev
```

Run the sidecar dry-run status service manually:

```bash
cargo run -p vaexcore-media-runner -- --status-addr 127.0.0.1:51387 --dry-run
```

When running as a service, `media-runner` exposes `/health`, `/status`, and dry-run recording/stream command endpoints.

## Local API

The desktop process starts the local API on:

```text
http://127.0.0.1:51287
ws://127.0.0.1:51287/events
```

If that port is occupied, the app binds a fallback localhost port and writes the active URLs to `api-discovery.json` in the app data directory. Connected tools should use that discovery file when available instead of assuming the default port.

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

HTTP responses include an `x-vaexcore-request-id` header. Local clients may send their own request ID with the same header for log correlation.
External clients can identify themselves with `x-vaexcore-client-id` and `x-vaexcore-client-name`; Studio shows recent clients on the Connected Apps page.

TypeScript integrations can use `@vaexcore/client-sdk`:

```ts
import { VaexcoreStudioClient } from "@vaexcore/client-sdk";

const client = new VaexcoreStudioClient({
  apiUrl: "http://127.0.0.1:51287",
  token: process.env.VAEXCORE_API_TOKEN,
});

await client.createMarker("manual-marker");
```

## MVP Behavior

- Create Twitch, YouTube, Kick, and custom RTMP stream destinations.
- Create recording profiles with output path, filename pattern, container, resolution, framerate, bitrate, and encoder preference.
- Start/stop recording and streaming independently or together.
- Create manual markers.
- Stream lifecycle events over WebSocket.
- Track recent localhost clients.
- Record a bounded command audit log without storing request bodies.
- Export/import profile bundles without raw stream keys.
- Write structured JSONL app logs under the local app data directory.
- Run macOS-first preflight checks for API, token, output folder, capture readiness, and sidecar health.
- Build and validate dry-run media pipeline plans through the API and sidecar.
- Simulate media execution with `DryRunMediaEngine`.
- Prefer supervised `media-runner` dry-run execution when the sidecar is available, with in-process dry-run fallback when it is missing during startup.

## Security Notes

- Stream keys are accepted as sensitive inputs and stored behind `SecretStore`.
- API responses expose secret references, not raw stream keys.
- Stream keys are never included in media events or status payloads.
- The API is localhost-only by default.
