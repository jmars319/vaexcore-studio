# Media Engine

The MVP media layer is intentionally dry-run. It validates lifecycle behavior and API/event contracts without requiring GStreamer, FFmpeg, capture permissions, or stream credentials.

## Traits

`vaexcore-media` defines `MediaEngine`:

- `start_recording(profile)`
- `stop_recording()`
- `start_stream(destination)`
- `stop_stream()`
- `status()`

The shared contracts also define:

- `CaptureSourceSelection`
- `CaptureSourceInventory`
- `CaptureFramePlan`
- `AudioMixerPlan`
- `PerformanceTelemetryPlan`
- `SceneRuntimeCommand`
- `SceneActivationRequest` / `SceneActivationResponse`
- `PreviewFrameRequest` / `PreviewFrameResponse`
- `CompositorRenderRequest` / `CompositorRenderResponse`
- `RuntimeCaptureSourceBindingContract`
- `RuntimeAudioSourceBindingContract`
- `TransitionExecutionRequest` / `TransitionExecutionResponse`
- `MediaPipelinePlanRequest`
- `MediaPipelinePlan`
- `MediaPipelineValidation`

These contracts let the UI, local API, sidecar, and future external tools agree on source selection and pipeline readiness before any real capture backend is started.
Scene sources also carry ordered filter chains for effects such as color
correction, chroma key, crop/pad, blur, LUTs, noise gates, and compressors.
The software preview compositor applies visual filters for color correction,
chroma key, crop/pad alpha crop, blur, sharpen, still-image mask/blend, and
`.cube` LUT transforms to source input pixels. The simulated audio graph applies
audio gain, noise gate, and compressor filters to audio meter runtime levels;
real captured audio mixing remains deferred.

The software preview compositor can decode local still-image `image_media`
sources when `media_type = "image"`. It supports PNG, JPEG, WebP, and the first
frame of GIF files, caches decoded pixels by normalized path plus file modified
time, and reports asset readiness metadata to Designer. It also rasterizes
single-line `text` sources with the bundled Inter font and reports font fallback,
color fallback, rendered bounds, and checksum metadata. Filter diagnostics report
applied, skipped, deferred, or failed runtime state plus filtered checksums.
Mask images use the same still-image decode/cache path, and LUT files are parsed
and cached by normalized path plus modified time. Video media, stinger video
playback, live capture, browser capture, recording, and streaming output remain
outside this path.

`CaptureFramePlan` maps visible capture-backed scene sources to the video or
audio frame stream the compositor will eventually consume. Each binding records
the scene source id, capture source id, media kind, expected format, dimensions
or audio shape, planned transport, and permission/availability status. This is a
contract and validation layer only; it does not start platform capture.

`AudioMixerPlan` maps visible audio meter sources to the master, monitor,
recording, and stream buses. It carries gain, mute, monitoring, meter, and sync
offset fields so the UI and future mixer engine agree on routing before real
audio mixing is implemented. `AudioGraphRuntimeSnapshot` reports deterministic
pre-filter and post-filter meter levels plus ordered audio filter diagnostics for
`audio_gain`, `noise_gate`, and `compressor`.

`PerformanceTelemetryPlan` maps enabled compositor render targets to frame
budget, render budget, encode budget, dropped-frame tolerance, latency ceiling,
and estimated RGBA throughput. It is a contract and validation layer for frame
pacing and hardware-readiness reporting; it does not start runtime profiling.

The Scene Runtime contracts define the payloads that the future runtime API will
use for scene activation, runtime state patching, preview polling, compositor
render requests, source binding readiness, audio binding readiness, and
transition execution. They are intentionally serializable and backend-friendly,
but this pass does not start the backend runtime or real capture/compositor
execution.

## macOS Source Inventory

The desktop backend now enumerates macOS capture inputs before a real capture backend is selected:

- displays through CoreGraphics, preserving `display:main` for the default main display selection
- visible windows through CoreGraphics window metadata when Screen Recording access allows useful names
- cameras through `system_profiler SPCameraDataType`
- microphone-capable CoreAudio devices through `system_profiler SPAudioDataType`

Camera and microphone preflight checks use AVFoundation authorization status. The Settings window exposes privacy shortcuts for Camera, Microphone, and Screen Recording so operators can resolve blocked permissions before starting a real pipeline.

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
- accepts planning and validation commands:
  - `POST /plan`
  - `POST /validate`
- can later wrap GStreamer, FFmpeg, or native capture pipelines

The desktop process writes two files in the app data directory:

- `pipeline-plan.json`: full `MediaPipelinePlan` for diagnostics and UI/external inspection
- `pipeline-config.json`: runner config shape consumed by `media-runner --config`

The config file includes the dry-run flag, sidecar status address when known, pipeline name, and resolved `MediaPipelineConfig`. It contains stream secret references only, not raw stream keys.
When an active scene is present, `MediaPipelineConfig` includes
`active_scene`, `capture_frame_plan`, `audio_mixer_plan`, `compositor_graph`,
`compositor_render_plan`, and `performance_telemetry_plan`.

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

The API asks the sidecar for `/plan` when it is available and falls back to the in-process dry-run planner if planning cannot reach the sidecar. This keeps the UI and external apps able to inspect readiness even when media execution is degraded.

Quit App and app-level exit events call the supervisor shutdown path before exiting so a managed `media-runner` process is not left running.

## Future Real Pipeline Requirements

- Encoder capability detection.
- Container-specific recovery strategy.
- Per-platform ingest validation.
- Backpressure-aware event reporting.
- Crash-safe sidecar restart behavior.
