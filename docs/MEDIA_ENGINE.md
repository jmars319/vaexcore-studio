# Media Engine

The MVP media layer is intentionally dry-run. It validates lifecycle behavior and API/event contracts without requiring GStreamer, FFmpeg, capture permissions, or stream credentials.

## Traits

`vaexcore-media` defines `MediaEngine`:

- `start_recording(profile)`
- `stop_recording()`
- `start_stream(destination)`
- `stop_stream()`
- `status()`

Start and stop operations are idempotent:

- Starting an already active recording returns the existing session.
- Stopping an idle recording returns success with `changed: false`.
- Starting an already active stream returns the existing session.
- Stopping an idle stream returns success with `changed: false`.

## DryRunMediaEngine

`DryRunMediaEngine` simulates:

- recording session creation
- output path generation from filename patterns
- stream session creation
- simultaneous stream and recording
- lifecycle event emission

It emits:

- `recording.started`
- `recording.stopped`
- `stream.started`
- `stream.stopped`

It does not log stream keys and does not include raw secrets in events or status.

## GStreamer Placeholder

`GStreamerMediaEngine` is feature-gated behind:

```bash
cargo check -p vaexcore-media --features gstreamer
```

The placeholder compiles without requiring GStreamer to be installed. Real GStreamer work should add a separate pipeline builder, capability checks, and install diagnostics before changing the default engine.

## Sidecar Contract

`sidecars/media-runner` is a replaceable execution process:

- accepts JSON config via stdin or `--config`
- supports `--dry-run`
- can expose `/health` and `/status`
- accepts lifecycle commands over localhost HTTP:
  - `POST /recording/start`
  - `POST /recording/stop`
  - `POST /stream/start`
  - `POST /stream/stop`
- can later wrap GStreamer, FFmpeg, or native capture pipelines

Example:

```bash
echo '{"dry_run":true,"pipeline_name":"local-test"}' | cargo run -p vaexcore-media-runner
```

Long-running status mode:

```bash
cargo run -p vaexcore-media-runner -- --status-addr 127.0.0.1:51387 --dry-run
```

## Sidecar Supervision

The desktop runtime now attempts to start `media-runner` on launch. Discovery order:

- `VAEXCORE_MEDIA_RUNNER_PATH`
- app resource directory and bundled Tauri sidecar locations
- executable directory
- `target/debug/media-runner`
- `target/release/media-runner`

Release builds stage the sidecar with:

```bash
npm run prepare:sidecars -w apps/desktop
```

Tauri bundles the staged binary through `bundle.externalBin`, which expects the `media-runner-<target-triple>` filename shape.

If a runner is found, `vaexcore-api` uses `SidecarMediaEngine` to forward recording and stream lifecycle commands to the runner over localhost HTTP. The runner owns dry-run lifecycle state, exposes `/status`, and preserves the same idempotent command semantics as `DryRunMediaEngine`.

If the runner is missing or fails to start during desktop startup, Studio stays usable with `DryRunMediaEngine`. Missing sidecars are logged, not surfaced as fatal startup errors. If a managed runner exits after startup, command calls return explicit unavailable errors instead of silently switching engines mid-session.

Quit App and app-level exit events call the supervisor shutdown path before exiting so a managed `media-runner` process is not left running.

## Future Real Pipeline Requirements

- Capture permission diagnostics on macOS.
- Encoder capability detection.
- Container-specific recovery strategy.
- Per-platform ingest validation.
- Backpressure-aware event reporting.
- Crash-safe sidecar restart behavior.
