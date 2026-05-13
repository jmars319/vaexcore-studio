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
- `CaptureProviderRuntimeSnapshot`
- `AudioMixerPlan`
- `PerformanceTelemetryPlan`
- `SceneRuntimeCommand`
- `SceneActivationRequest` / `SceneActivationResponse`
- `PreviewFrameRequest` / `PreviewFrameResponse`
- `ProgramPreviewFrameRequest` / `ProgramPreviewFrameResponse`
- `CompositorRenderRequest` / `CompositorRenderResponse`
- `RuntimeCaptureSourceBindingContract`
- `RuntimeAudioSourceBindingContract`
- `TransitionExecutionRequest` / `TransitionExecutionResponse`
- `MediaPipelinePlanRequest`
- `MediaPipelinePlan`
- `MediaPipelineValidation`

These contracts let the UI, local API, sidecar, and future external tools agree on source selection and pipeline readiness before any real capture backend is started.
The program-preview contract renders the active saved scene through the same
software source runtime as the editor preview, but targets a `program` frame for
output-pipeline readiness without starting recording, streaming, or encoders.
Scene sources also carry ordered filter chains for effects such as color
correction, chroma key, crop/pad, blur, LUTs, noise gates, and compressors.
The software preview compositor applies visual filters for color correction,
chroma key, crop/pad alpha crop, blur, sharpen, still-image mask/blend, and
`.cube` LUT transforms to source input pixels. The audio graph applies audio
gain, noise gate, and compressor filters to audio meter runtime levels from
deterministic simulation or live Designer probes where available; real output
audio mixing remains deferred.

The software preview compositor can decode local still-image `image_media`
sources when `media_type = "image"`. It supports PNG, JPEG, WebP, and the first
frame of GIF files, caches decoded pixels by normalized path plus file modified
time, and reports asset readiness metadata to Designer. It can also extract
local video preview frames for `image_media` sources when `media_type = "video"`
and FFmpeg is available. Video V1 supports MP4, MOV, WebM, and MKV files,
samples a deterministic media timeline at a conservative interval, supports
play/pause/stop, playback rate, looped extraction, and restart-on-scene-activate
metadata, and caches decoded frames by normalized path, file modified time,
sampled time, and loop mode. Missing FFmpeg, missing files, unsupported
extensions, and extraction failures remain explicit placeholder states.
Transition previews render `cut`, `fade`, and `swipe` pixels in the backend
software path, and stinger previews reuse the same timeline-aware optional video
extraction to composite sampled local transition video over still from/to scene
software frames and report trigger, decoder, cache, timeline, and fallback
metadata.
The compositor also rasterizes single-line `text` sources with the
bundled Inter font and reports font fallback, color fallback, rendered bounds,
and checksum metadata. Browser overlay sources can render preview frames through
managed optional local Chrome, Chromium, or Edge DevTools sessions for HTTP,
HTTPS, and file URLs. Browser sessions use isolated temporary profiles, apply
configured viewport and custom CSS, support refresh interval and reload-token
changes, cache by source, URL, viewport, CSS hash, sampled time, and reload
token, and report lifecycle, process reuse, cleanup, cache, and fallback
diagnostics when no compatible browser is available or capture fails. Filter
diagnostics report applied, skipped, deferred, or failed runtime state plus
filtered checksums.
Mask images use the same still-image decode/cache path, and LUT files are
parsed and cached by normalized path plus modified time. Video audio playback,
recording, and streaming output remain outside this path.

`CaptureFramePlan` maps visible capture-backed scene sources to the video or
audio frame stream the compositor will eventually consume. Each binding records
the scene source id, capture source id, media kind, expected format, dimensions
or audio shape, planned transport, and permission/availability status. This is a
contract and validation layer only; it does not start platform capture.

`CaptureProviderRuntimeSnapshot` mirrors the active scene's capture bindings into
provider lifecycle state. V1 uses deterministic mocked providers so the UI,
API, and tests can validate display, window, camera, microphone, and
system-audio readiness, frame/audio packet contracts, latency, dropped-frame
counters, and lifecycle transitions before any native capture backend is
started.

The software preview compositor can now request one-shot macOS display/window
snapshot frames through the native `screencapture` path when a display or window
source is bound and Screen Recording permission is available. Camera sources can
request one-shot macOS FFmpeg/AVFoundation preview frames when a camera is bound,
FFmpeg is installed, and Camera permission is available. Captured pixels enter
the same transform, crop, opacity, z-order, group, and visual-filter path as
image/text/browser pixels. Missing permissions, missing FFmpeg, unsupported
source ids, unsupported platforms, and capture failures remain explicit
placeholder states with provider, frame, duration, latency, dropped-frame, and
checksum diagnostics. This is Designer preview capture only; persistent native
capture sessions, camera capability negotiation, and recording/streaming output
capture remain later work.

`AudioMixerPlan` maps visible audio meter sources to the master, monitor,
recording, and stream buses. It carries gain, mute, monitoring, meter, and sync
offset fields so the UI and future mixer engine agree on routing before output
audio mixing is implemented. The runtime audio graph can now prefer one-shot
macOS FFmpeg/AVFoundation microphone or system-audio level probes for assigned
audio meter sources, then feeds those levels through the same gain, mute, sync,
bus, and audio-filter path. If FFmpeg, permissions, platform support, or source
availability are missing, the graph reports explicit silent/fallback diagnostics
rather than starting playback or output. `AudioGraphRuntimeSnapshot` reports
input mode, provider, sample count, capture duration, latency, pre/post-filter
levels, meter decay, peak hold, bus source counts, and ordered diagnostics for
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
