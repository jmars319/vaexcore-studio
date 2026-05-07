# vaexcore studio

vaexcore studio is a local-first desktop control system for streaming, recording, and localhost integrations. It is the foundation layer that other vaexcore tools can use for local API discovery, recording control, marker capture, and connected-app coordination.

Studio is not a cloud control plane, giveaway bot, highlight detector, scene editor, or plugin marketplace. Its role is local infrastructure.

## Operational Purpose

- Provide a trusted local API for creator tooling.
- Coordinate local recording, stream control, markers, profiles, and connected clients.
- Keep media execution behind a replaceable sidecar boundary.
- Give companion apps a durable discovery and command surface.

## Design Posture

- Localhost integration over hosted dependency.
- Desktop app owns supervision and operator visibility.
- Rust crates own core contracts, API, media, and platform boundaries.
- Dry-run media behavior remains available for safe development.
- Connected-client registry and audit logs are part of the operational model.

## Architecture

```text
apps/
  desktop/                 Tauri v2 + React desktop app

crates/
  vaexcore-core/           Shared Rust contracts, profiles, events, responses
  vaexcore-api/            Local HTTP and WebSocket API
  vaexcore-media/          Media traits, dry-run engine, sidecar control
  vaexcore-platforms/      Streaming platform definitions

packages/
  shared-types/            TypeScript API and event contracts
  client-sdk/              TypeScript client for localhost integrations

sidecars/
  media-runner/            Replaceable media execution sidecar
```

## Current State

- The macOS-first Tauri desktop app is the active product surface.
- A local HTTP/WebSocket API starts from the desktop process.
- API discovery is written locally when the default port changes.
- The media runner can run as a supervised dry-run sidecar.
- Client SDK and smoke examples exist for companion integration testing.
- Windows launcher material is present, but platform maturity still centers on local desktop validation.

## Deployment Posture

Studio is currently a local desktop infrastructure app. It supports local packaging and release staging, but it should be evaluated as operator-controlled local software, not a hosted service.

## Working Locally

```bash
npm install
npm run tauri -w apps/desktop -- dev
npm run typecheck
npm run build
npm run prepare:sidecars
npm run check:sidecars
cargo test --workspace
```

The local API defaults to `http://127.0.0.1:51287` and `ws://127.0.0.1:51287/events`.

## Direction

- Continue hardening the local API and discovery contract.
- Expand sidecar supervision without coupling Studio to one media backend.
- Keep companion app integration explicit through the SDK and suite protocol.
- Preserve dry-run behavior as a safety path for development and testing.

## Related Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [API](docs/API.md)
- [Media Engine](docs/MEDIA_ENGINE.md)
- [Suite Protocol](docs/SUITE_PROTOCOL.md)
- [Roadmap](docs/ROADMAP.md)
