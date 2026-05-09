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
6. Program/output renderer for recording and streaming: started with a compact
   software frame probe; real capture-backed recording/streaming output remains
   Phase 2+.
7. Render runtime and frame evaluation: done for deterministic contract-frame
   evaluation and a deterministic RGBA software probe; GPU-backed pixel
   rendering is not started.
8. GPU-backed preview renderer: not started.
9. Program/output renderer producing real frames: started with deterministic
   software RGBA frame buffers and checksums; encoder output is not started.
10. Frame timing, resolution, FPS, scaling, and color contracts: started with
    render target timing/scaling evaluation.
11. Display capture binding on macOS and Windows: started with inventory-backed
    source availability plus capture-frame plan bindings; real capture is not
    started.
12. Window capture binding on macOS and Windows: started with inventory-backed
    source availability plus capture-frame plan bindings; real capture is not
    started.
13. Camera source engine: started with inventory-backed source availability plus
    capture-frame plan bindings; real capture is not started.
14. Microphone and system audio capture: started with inventory-backed source
    availability, capture-frame plan bindings, and audio source routing fields;
    real audio capture is not started.
15. Audio mixer model, meters, and routing: started with serializable mixer
    buses, gain/mute/monitor/sync controls, and pipeline validation; real audio
    mixing and live meters are not started.
16. Image and media source engine: started with source-specific preview
    rendering; real media decode/playback is not started.
17. Browser/web overlay source engine: started with source-specific preview
    rendering; real browser capture is not started.
18. Text render engine with font controls: started with canvas preview text
    rendering; backend text rasterization is not started.
19. Groups, nesting, and parent transforms: started with group preview
    rendering, group child validation, compositor parent/depth metadata, and
    first-pass nested position/rotation/opacity evaluation. Full group
    clipping, scaling inheritance, and group editor actions are not started.
20. Full editor interactions: started with drag/resize, keyboard nudging,
    undo/redo, transform command buttons, align-to-canvas controls, and
    edge/center snapping guides, source duplicate, and source copy/paste;
    multi-select, rotate handles, and distribute are not started.
21. Crop, bounds modes, fit/fill/stretch/center controls: started with numeric
    crop fields, reset crop, fit-to-canvas, center controls, serializable source
    bounds modes, Designer bounds controls, and first-pass compositor
    fit/fill/stretch/center/original-size evaluation. Real source clipping and
    full renderer policy parity are not started.
22. Source filters and effects: started with serializable per-source filter
    chains, supported filter kinds, duplicate/order/config validation, and
    compositor graph propagation plus first-pass Designer filter chain editing.
    Real video/audio filter rendering and detailed per-filter controls are not
    started.
23. Scene transitions and transition preview: started with persisted transition
    contracts, validation, Designer controls, and frame/easing preview plans;
    live pixel transition playback and renderer application are not started.
24. Scene collection import/export/backup: started with versioned bundle
    contracts, local API export/import routes, SDK helpers, desktop bridge
    commands, store validation, and Designer import/export actions against the
    app data bundle path. Desktop imports write a timestamped backup of the
    current scene collection and retain the newest 10 backups. Designer-facing
    file picker UX is not started.
25. Hotkeys and workflow shortcuts: started with Designer-level save,
    undo/redo, selected-source delete, copy, paste, duplicate, grouping,
    visibility/lock, z-order, nudge, and rotate shortcuts. Designer shortcuts
    now have a local configurable shortcut panel; app-wide shortcut routing is
    not started.
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
media shape, candidate availability, refresh, and auto-bind controls. This still
does not start real capture; it prepares the saved scene graph and runtime
contracts for capture-backed frames in later phases.

Phase F software compositor, covering steps 51-60, now has serializable input
frame contracts, per-source placeholder providers, crop/opacity/rotation-aware
software drawing, z-order compositing, and compositor tests. The inputs are still
deterministic placeholders, not real capture frames.

Phase G live preview, covering steps 61-70, now returns encoded software preview
image data through the runtime preview API and Designer draws it as the preview
base frame. Designer also has preview pause/resume, quality selection, FPS limit
metadata, dropped-frame accounting, render timing, transport size, and visual
smoke coverage. It remains a software placeholder preview until real capture
providers are connected.

Phase H audio foundation, covering steps 71-80, now has an audio graph runtime
snapshot, simulated meter levels, gain/mute/monitor/sync metadata, runtime
validation, API/SDK/Desktop client access, and Designer meter displays in the
preview and Inspector. The meters are deterministic simulation, not live device
audio capture.

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
`capture_frame_plan`, `audio_mixer_plan`, `compositor_graph`, and
`compositor_render_plan`, and `performance_telemetry_plan` before Phase 1 is
considered wired.
