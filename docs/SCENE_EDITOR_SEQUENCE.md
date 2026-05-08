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
19. Groups, nesting, and parent transforms: started with group preview rendering;
    nested transform evaluation is not started.
20. Full editor interactions: started with drag/resize, keyboard nudging,
    undo/redo, transform command buttons, align-to-canvas controls, and
    edge/center snapping guides; multi-select, rotate handles, distribute, and
    copy/paste are not started.
21. Crop, bounds modes, fit/fill/stretch/center controls: started with numeric
    crop fields, reset crop, fit-to-canvas, and center controls; source bounds
    modes and fit/fill/stretch policies are not started.
22. Source filters and effects: started with serializable per-source filter
    chains, supported filter kinds, validation, and compositor graph
    propagation plus first-pass Designer filter chain editing. Real video/audio
    filter rendering and detailed per-filter controls are not started.
23. Scene transitions and transition preview: started with persisted transition
    contracts, validation, and Designer controls; live transition preview and
    renderer application are not started.
24. Scene collection import/export/backup: started with versioned bundle
    contracts, local API export/import routes, SDK helpers, desktop bridge
    commands, store validation, and Designer import/export actions against the
    app data bundle path. Designer-facing file picker UX and automatic backup
    rotation are not started.
25. Hotkeys and workflow shortcuts: not started.
26. Active-scene recording and streaming integration: not started.
27. Performance tuning: frame pacing, latency, dropped frames, GPU/CPU load:
    started with per-target frame budget, latency, dropped-frame tolerance, and
    estimated throughput contracts. Live profiler sampling, GPU counters, and
    long-run tuning are not started.
28. Full validation matrix: automated, visual, Windows/macOS hardware, installer,
    and long-run soak tests: not started.

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
