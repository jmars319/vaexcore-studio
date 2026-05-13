# Scene Editor Sequence

This is the implementation sequence for taking Studio from the current Designer
foundation to OBS-class scene editing and output.

## Status

1. Scene contracts, defaults, and validation: done.
2. Scene persistence, API, SDK, and desktop bridge: done.
3. Designer shell with scene/source panels, preview, and inspector: done.
4. Compositor render-graph contract: done for the serializable graph contract.
5. Preview renderer using the compositor graph: done for the placeholder graph
   canvas.
6. Program/output renderer for recording and streaming: started with a dedicated
   program-preview frame API that renders the active saved scene as a software
   `program` target at program resolution/FPS. Real capture-backed
   recording/streaming output remains Phase 2+.
7. Render runtime and frame evaluation: done for deterministic contract-frame
   evaluation and a deterministic RGBA software probe; GPU-backed pixel
   rendering is not started.
8. GPU-backed preview renderer: not started.
9. Program/output renderer producing real frames: started with deterministic
   software RGBA program-preview frame buffers, encoded image data, and
   checksums; encoder output is not started.
10. Frame timing, resolution, FPS, scaling, and color contracts: started with
    render target timing/scaling evaluation.
11. Display capture binding on macOS and Windows: started with inventory-backed
    source availability, capture-frame plan bindings, and mocked provider
    lifecycle diagnostics. macOS Designer preview can now request one-shot
    native `screencapture` display snapshots when the source is bound and Screen
    Recording permission is available; persistent capture and Windows live
    capture remain later work.
12. Window capture binding on macOS and Windows: started with inventory-backed
    source availability, capture-frame plan bindings, and mocked provider
    lifecycle diagnostics. macOS Designer preview can now request one-shot
    native `screencapture` window snapshots for bound window ids when permission
    is available; persistent capture and Windows live capture remain later work.
13. Camera source engine: started with inventory-backed source availability plus
    capture-frame plan bindings, mocked provider lifecycle diagnostics, and
    optional macOS FFmpeg/AVFoundation one-shot Designer preview frames when a
    camera is bound, FFmpeg is installed, and Camera permission is available.
    Persistent camera sessions and Windows live camera capture remain later
    work.
14. Microphone and system audio capture: started with inventory-backed source
    availability, capture-frame plan bindings, mocked provider lifecycle
    diagnostics, and audio source routing fields; real audio capture is not
    started.
15. Audio mixer model, meters, and routing: started with serializable mixer
    buses, gain/mute/monitor/sync controls, simulated pre/post-filter levels,
    and pipeline validation; real audio mixing and live meters are not started.
16. Image and media source engine: started with real local still-image decode
    for PNG, JPEG, WebP, and first-frame GIF preview rendering plus optional
    FFmpeg-backed local video preview frame extraction for MP4, MOV, WebM, and
    MKV assets. Video audio playback and long-running media timelines are not
    started.
17. Browser/web overlay source engine: started with optional local
    Chrome/Chromium/Edge DevTools rendering for HTTP, HTTPS, and file URLs.
    Full Scene Designer Pass 7 adds managed preview sessions, refresh interval,
    reload-token handling, viewport/CSS reinjection, process reuse, cache, and
    cleanup diagnostics. Browser audio, interactive overlay input, and output
    parity are not started.
18. Text render engine with font controls: started with backend software
    rasterization for current single-line text source fields. Multiline layout,
    rich text, and platform font discovery are not started.
19. Groups, nesting, and parent transforms: done for offline editor V1 with
    group preview rendering, structured child management, group child
    validation, compositor parent/depth metadata, and first-pass nested
    position/rotation/opacity evaluation. Full group clipping and scaling
    inheritance remain renderer parity work.
20. Full editor interactions: started with drag/resize, keyboard nudging,
    undo/redo, transform command buttons, align-to-canvas controls,
    edge/center snapping guides, multi-select, rotate handles, distribute,
    source duplicate, and multi-source copy/paste. Advanced constraints,
    guides, and layer operations remain future polish.
21. Crop, bounds modes, fit/fill/stretch/center controls: started with numeric
    crop fields, reset crop, fit-to-canvas, center controls, serializable source
    bounds modes, Designer bounds controls, and first-pass compositor
    fit/fill/stretch/center/original-size evaluation. Real source clipping and
    full renderer policy parity are not started.
22. Source filters and effects: started with serializable per-source filter
    chains, supported filter kinds, duplicate/order/config validation, and
    compositor graph propagation, first-pass Designer filter chain editing, and
    software preview execution for color correction, chroma key, crop/pad alpha
    crop, blur, sharpen, still-image mask/blend, and `.cube` LUT transforms.
    Audio graph execution is done for audio gain, noise gate, and compressor
    filters over deterministic simulation and live Designer level probes where
    available; output pipeline filter parity remains deferred.
23. Scene transitions and transition preview: started for OBS-class runtime V1
    with persisted transition contracts, validation, Designer controls, from/to
    scene selection, scrub/playback controls, and backend software pixel preview
    frames for `cut`, `fade`, `swipe`, and `stinger`. Stinger transitions can
    request optional FFmpeg video-frame extraction, trigger-time scene switching,
    cache diagnostics, and explicit placeholder states when no asset or decoder
    is available. Live program/output transition execution, stinger audio, and
    encoder parity are not started.
24. Scene collection import/export/backup: done for offline editor V1 with
    versioned bundle contracts, local API export/import routes, SDK helpers,
    desktop bridge commands, explicit user-selected JSON import/export, app-data
    fallback import/export, store validation, and timestamped backups after
    successful import validation. Desktop retains the newest 10 backups.
25. Hotkeys and workflow shortcuts: started with Designer-level save,
    undo/redo, selected-source delete, copy, paste, duplicate, grouping,
    visibility/lock, z-order, nudge, and rotate shortcuts. Designer shortcuts
    now have a local configurable shortcut panel and an external committed
    shortcut reference; app-wide shortcut routing is not started.
26. Active-scene recording and streaming integration: started with API launch
    requests carrying the active scene into recording/streaming engines and
    start events reporting active scene identity. Real capture-backed
    program/output frames remain Phase 2+.
27. Performance tuning: frame pacing, latency, dropped frames, GPU/CPU load:
    started with per-target frame budget, latency, dropped-frame tolerance, and
    estimated throughput contracts. Live profiler sampling, GPU counters, and
    long-run tuning are not started.
28. Full validation matrix: automated, visual, Windows/macOS hardware, installer,
    and long-run soak tests: started with repeatable macOS app build validation
    a Windows validation runner/guide, and Chrome screenshot smoke checks with
    PNG size/dimension assertions. Real Windows hardware results, installer QA,
    and long-run soak tests are not started.

## 100-Step Progress

Phase B runtime contracts, covering steps 11-20 from the 100-step plan, are now
in the shared type package. This includes serializable command, preview-frame,
compositor render, capture binding, audio binding, scene activation, runtime
state update, and transition execution contracts with validation/default helper
coverage. These are contract-only foundations; backend runtime state, API routes,
preview polling, and live compositor execution begin in Phase C and later.

Phase C backend scene runtime, covering steps 21-30, is now started with an
in-process runtime snapshot, active scene/transition state, scene activation,
runtime state patching, contract preview-frame requests, runtime graph
validation, capture/audio binding snapshots, SDK helpers, and API smoke coverage.
It still returns contract metadata only; real compositor execution and capture
frames remain later phases.

Phase D preview plumbing, covering steps 31-40, is now started in Designer with
runtime preview-frame polling, manual frame refresh, loading/error state, frame
metadata, diagnostics, and runtime-frame canvas drawing from backend contract
frames. The preview still uses deterministic contract frames, not real captured
pixels.

Phase E capture binding, covering steps 41-50, is now started in Designer with
source-to-inventory candidate matching for display, window, camera, microphone,
and system-audio sources. The Inspector surfaces runtime binding status, target,
media shape, candidate availability, mocked provider lifecycle diagnostics,
refresh, and auto-bind controls. The API now exposes a serializable capture
provider runtime snapshot with deterministic mock video/audio packet contracts,
latency, dropped-frame, and readiness metadata. This still does not start real
capture; it prepares the saved scene graph and runtime contracts for
capture-backed frames in later phases.

Full Scene Designer Pass 4 is now started for macOS display/window capture V1:
software preview frames can use real one-shot macOS `screencapture` pixels for
bound display/window sources when Screen Recording permission is available, and
Designer shows live capture frame diagnostics. This is not yet a persistent
ScreenCaptureKit provider, and it does not cover Windows live capture,
recording, streaming, or encoder output.

Full Scene Designer Pass 5 is now started for camera capture V1: software
preview frames can request one-shot macOS FFmpeg/AVFoundation camera snapshots
for bound camera sources when FFmpeg and Camera permission are available, and
Designer shows camera frame, latency, decoder, and fallback diagnostics. This is
not yet a persistent camera provider, does not negotiate camera capabilities,
and does not cover Windows live camera capture, recording, streaming, or encoder
output.

Phase F software compositor, covering steps 51-60, now has serializable input
frame contracts, per-source placeholder providers, local still-image decode,
local video preview frame extraction through optional FFmpeg, backend text
rasterization with bundled Inter, managed browser overlay preview sessions
through optional Chrome/Chromium/Edge, asset cache invalidation by path/source
identity and sampled time, crop/opacity/rotation-aware software drawing,
z-order compositing, and compositor tests. Persistent display/window/camera
capture and output browser parity remain later work.

Phase G live preview, covering steps 61-70, now returns encoded software preview
image data through the runtime preview API and Designer draws it as the preview
base frame. Designer also has preview pause/resume, quality selection, FPS limit
metadata, dropped-frame accounting, render timing, transport size, and visual
smoke coverage. It remains a software placeholder preview until real capture
providers are connected.

Phase H audio foundation, covering steps 71-80, now has an audio graph runtime
snapshot, pre/post-filter meter levels, gain/mute/monitor/sync metadata, audio
gain/noise gate/compressor filter diagnostics, runtime validation,
API/SDK/Desktop client access, and Designer meter displays in the preview and
Inspector. Full Scene Designer Pass 6 now lets the API prefer one-shot macOS
FFmpeg/AVFoundation microphone or system-audio level probes for assigned audio
meter sources, with input mode, provider, sample count, capture duration,
latency, decay, peak hold, and bus source-count diagnostics. Missing FFmpeg,
permissions, unsupported platforms, or unavailable sources stay explicit
silent/fallback states. Output audio mixing, monitoring playback, recording, and
streaming audio remain later work.

Phase I recording/streaming prep, covering steps 81-90, now has render target
profiles, recording target contracts, streaming target contracts, dry-run
encoder/output-path/destination readiness checks, and a Designer runtime
preflight panel. It prepares outputs but still does not start real recording,
streaming, capture, or encoder work.

Phase J editor polish, covering steps 91-100, now has a searchable/filterable
Add Source modal, inline source-stack renaming, multi-source copy/paste and
duplicate actions, grouped command-bar history for selection visibility/lock
commands, an external Designer shortcut reference, scene import runtime refresh,
and an updated Windows validation runner. This cuts the runtime-preview-ready
milestone; it is still not a real capture, recording, or streaming backend.

Offline Editor V1 is now complete for local authoring scope: scene/source
editing, grouping, numeric transforms, local asset URI selection, explicit
bundle import/export, app-data fallback bundles, validation, runtime refresh,
deterministic placeholder transition preview, real local still-image preview
pixels, FFmpeg-backed local video preview frames when FFmpeg is available,
backend-rendered single-line text pixels, optional browser overlay preview
snapshots, software visual filter preview pixels, one-shot macOS display/window
capture snapshots, and one-shot macOS camera snapshots through optional FFmpeg
when source bindings and permissions are available. The remaining Scene Designer
work starts persistent capture sessions, interactive browser lifecycle, live
audio, timeline playback, and the output handoff path; offline editor completion
does not imply OBS-level capture, encoder, plugin, or live-output parity.

## Validation Contract

Every chunk from step 4 onward must keep these gates green unless a platform
blocker is explicitly documented:

```sh
npm run test:scripts
npm run typecheck --workspaces --if-present
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

macOS packaging validation uses:

```sh
npm run app:build:mac
```

Windows packaging validation must be run on a Windows machine:

```sh
npm run app:build:windows
```

The generated `pipeline-config.json` must include `active_scene`,
`capture_frame_plan`, `audio_mixer_plan`, `compositor_graph`,
`compositor_render_plan`, `performance_telemetry_plan`, and
`output_preflight_plan` before Phase 1 is considered wired.
