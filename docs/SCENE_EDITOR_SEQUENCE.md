# Scene Editor Sequence

This is the implementation sequence for taking Studio from the current Designer
foundation to OBS-class scene editing and output.

## Status

1. Scene contracts, defaults, and validation: done.
2. Scene persistence, API, SDK, and desktop bridge: done.
3. Designer shell with scene/source panels, preview, and inspector: done.
4. Compositor render-graph contract: in progress, graph contract added.
5. Preview renderer using the compositor graph: in progress, placeholder graph
   canvas added.
6. Program/output renderer for recording and streaming: not started.
7. Display capture binding on macOS and Windows: not started.
8. Window capture binding on macOS and Windows: not started.
9. Camera source engine: not started.
10. Microphone and system audio capture: not started.
11. Audio mixer model, meters, and routing: not started.
12. Image and media source engine: not started.
13. Browser/web overlay source engine: not started.
14. Text render engine with font controls: not started.
15. Groups, nesting, and parent transforms: not started.
16. Full editor interactions: multi-select, snapping, rotate, align, copy/paste,
    undo/redo: not started.
17. Crop, bounds modes, fit/fill/stretch/center controls: not started.
18. Source filters and effects: not started.
19. Scene transitions and transition preview: not started.
20. Scene collection import/export/backup: not started.
21. Hotkeys and workflow shortcuts: not started.
22. Active-scene recording and streaming integration: not started.
23. Performance tuning: frame pacing, latency, dropped frames, GPU/CPU load: not
    started.
24. Full validation matrix: automated, visual, Windows/macOS hardware, installer,
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

The generated `pipeline-config.json` must include both `active_scene` and
`compositor_graph` before the program/output renderer is considered wired.
