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
- app resource directory
- executable directory
- `target/debug/media-runner`
- `target/release/media-runner`

If a runner is found, `vaexcore-api` wraps the dry-run lifecycle with `SidecarMediaEngine` and polls the runner `/status` endpoint. The command lifecycle remains idempotent and MVP-safe while the sidecar contract is still status-only.

If the runner is missing, fails to start, or becomes unavailable, Studio stays usable with `DryRunMediaEngine`. Missing sidecars are logged, not surfaced as fatal startup errors.

Quit App calls the supervisor shutdown path before exiting so a managed `media-runner` process is not left running.

## Future Real Pipeline Requirements

- Capture permission diagnostics on macOS.
- Encoder capability detection.
- Container-specific recovery strategy.
- Per-platform ingest validation.
- Backpressure-aware event reporting.
- Crash-safe sidecar restart behavior.
