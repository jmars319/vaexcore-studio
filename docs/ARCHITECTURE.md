# Architecture

`vaexcore studio` is structured around strict ownership boundaries:

- UI renders state and sends commands.
- Local API owns command routing, auth, persistence, and events.
- Media engine owns recording/stream lifecycle.
- Platform crate owns destination defaults and platform metadata.
- Sidecar owns replaceable media execution.

## Process Model

The Tauri desktop process starts:

1. React UI in the WebView.
2. Rust local API on `127.0.0.1:51287`.
3. App-data discovery and media planning files.
4. A supervised bundled `media-runner` sidecar when available.
5. `DryRunMediaEngine` in-process as the fallback and MVP simulation layer.

The UI does not depend on the sidecar being present. If `media-runner` is missing or unhealthy, the app remains usable through dry-run media execution. Release builds stage `media-runner` as a Tauri external binary.

The local API refreshes `pipeline-plan.json` and `pipeline-config.json` when the default media plan is requested or profile data changes. The sidecar is launched with the generated config path so future media backends can move from dry-run planning to real execution without changing the UI contract.

## Crate Responsibilities

### `vaexcore-core`

Shared Rust types:

- Recording profiles
- Stream destinations
- Sensitive string and secret references
- Engine status
- Recording and stream sessions
- API response envelopes
- Event contracts

### `vaexcore-api`

Local API:

- HTTP routes
- WebSocket event stream
- Token auth with dev bypass
- SQLite persistence
- SecretStore implementation
- Command audit persistence
- Recent client registry
- Dry-run engine wiring
- Optional sidecar supervision, command transport, and health events
- Pipeline plan/config file contract

### `vaexcore-media`

Media abstraction:

- `MediaEngine`
- `RecordingSession`
- `StreamSession`
- `MediaProfile`
- `StreamDestination`
- `EngineStatus`
- `DryRunMediaEngine`
- `SidecarMediaEngine`
- `MediaRunnerSupervisor`
- feature-gated `GStreamerMediaEngine` placeholder

### `vaexcore-platforms`

Platform profiles:

- Twitch RTMP
- YouTube RTMPS
- Kick as named custom RTMP/RTMPS for MVP
- Custom RTMP/RTMPS

### `packages/shared-types`

TypeScript contracts mirrored from the API. External clients can use this package as the API/event contract source.

## Data Flow

```text
React UI
  -> HTTP command
  -> vaexcore-api
  -> request ID/client registry/audit middleware
  -> SQLite profile/secret lookup
  -> MediaEngine trait
  -> SidecarMediaEngine when media-runner is available
  -> media-runner HTTP command transport
  -> dry-run lifecycle state in sidecar
  -> DryRunMediaEngine fallback
  -> StudioEvent
  -> EventBus
  -> WebSocket clients
```

## Stability Rules

- Start/stop commands are idempotent.
- Events emit only on actual lifecycle transitions.
- Secrets do not appear in logs, status, or events.
- Command audit entries never store request bodies.
- API responses use structured JSON envelopes.
- Partial failures return explicit errors and do not mutate unrelated state.
