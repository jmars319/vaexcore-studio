# Local API

Base URL:

```text
http://127.0.0.1:51287
```

WebSocket URL:

```text
ws://127.0.0.1:51287/events
```

The desktop app writes an API discovery document to its app data directory at
`api-discovery.json`. External tools should prefer that file when present,
because vaexcore studio can fall back to an available port if the configured
port is already occupied.

The same app data directory also contains media planning files:

- `pipeline-plan.json`
- `pipeline-config.json`

External tools may read these files for diagnostics, but should use the HTTP API as the source of truth while Studio is running.
When a scene is active, the generated pipeline config includes
`active_scene`, `capture_frame_plan`, `audio_mixer_plan`, `compositor_graph`,
`compositor_render_plan`, and `performance_telemetry_plan`.
Compositor graph nodes include optional `parent_source_id` and `group_depth`
fields so group/nesting transforms can be resolved consistently by preview and
program renderers.
Scene sources may include a serializable `filters` chain; the compositor graph
preserves that chain and software preview diagnostics report applied, skipped,
deferred, or failed filter runtime state.
Scene sources also include `bounds_mode` (`stretch`, `fit`, `fill`, `center`,
or `original_size`), which maps into compositor node `scale_mode` evaluation.
Scene transition helpers expose frame-count, easing sample plans, and
deterministic placeholder preview frames for `cut`, `fade`, `swipe`, and
`stinger` transition review before renderer handoff.
Desktop scene bundle imports validate the selected bundle first, then create
timestamped backups under `scene-backups` in the app data directory before
replacing the active collection. The app-data bundle path remains available as a
fallback, while the desktop UI can import/export an explicit user-selected JSON
bundle path.
Shared Scene Runtime contracts now define scene activation, runtime state
updates, preview-frame requests/responses, compositor render requests/responses,
capture/audio binding readiness, and transition execution payloads. These are
available through local runtime API routes. The routes return contract frames and
binding readiness only; they do not start real capture or recording/streaming
output.

## Auth

Debug builds enable dev auth bypass by default. When auth is required, pass either:

```http
Authorization: Bearer <token>
```

or:

```http
x-vaexcore-token: <token>
```

For WebSocket clients, pass `?token=<token>` or the token header. Browser clients that cannot send headers may pass `?client_id=<id>&client_name=<name>` for the recent client registry.

HTTP responses include `x-vaexcore-request-id`. Clients may send this header on requests to correlate their own logs with Studio logs; otherwise the API generates one.

Clients may also send:

```http
x-vaexcore-client-id: stable-client-id
x-vaexcore-client-name: Human Friendly Client Name
```

These headers feed the recent client registry. They are labels only, not auth credentials.

## TypeScript Client SDK

The workspace includes `@vaexcore/client-sdk` for local tools and bots:

```ts
import { VaexcoreStudioClient } from "@vaexcore/client-sdk";

const client = new VaexcoreStudioClient({
  apiUrl: "http://127.0.0.1:51287",
  token: process.env.VAEXCORE_API_TOKEN,
  clientId: "my-control-tool",
  clientName: "My Control Tool",
});

await client.createMarker({
  label: "manual-marker",
  source_app: "my-control-tool",
});
const status = await client.status();
```

## Response Envelope

Success:

```json
{
  "ok": true,
  "data": {},
  "error": null
}
```

Failure:

```json
{
  "ok": false,
  "data": null,
  "error": {
    "code": "media_error",
    "message": "stream destination requires an ingest URL"
  }
}
```

## Routes

### `GET /health`

Returns service health and auth mode.

### `GET /status`

Returns `StudioStatus`:

- engine status
- recording state
- stream state
- active destination
- recording path
- recent events

### `GET /clients`

Returns recent localhost clients observed through HTTP and WebSocket traffic:

```json
{
  "clients": []
}
```

### `GET /audit-log`

Returns recent command audit entries:

```json
{
  "entries": []
}
```

Audit entries include method, path, action, status, request ID, client label, and timestamp. Request bodies are not stored.

### `GET /recordings/recent`

Returns completed recording sessions, newest first:

```json
{
  "recordings": [
    {
      "session_id": "rec_...",
      "output_path": "/Users/me/Movies/vaexcore studio/clip.mkv",
      "profile_id": "rec_profile_...",
      "profile_name": "1080p60 Local",
      "started_at": "2026-05-02T12:00:00Z",
      "stopped_at": "2026-05-02T12:05:00Z"
    }
  ]
}
```

### `GET /markers`

Returns recent markers created by Studio or connected apps:

```json
{
  "markers": []
}
```

Supported query parameters:

- `source_app`
- `source_event_id`
- `recording_session_id`
- `limit`

### `GET /scenes`

Returns the saved `SceneCollection` used by Designer and media planning.

### `PUT /scenes`

Saves a complete `SceneCollection`, validates it, updates `updated_at`, and refreshes the default pipeline contract files.

### `GET /scenes/export`

Returns a versioned `SceneCollectionBundle`:

```json
{
  "version": 1,
  "exported_at": "2026-05-08T12:00:00Z",
  "collection": {}
}
```

### `POST /scenes/import`

Accepts a `SceneCollectionBundle`, validates and saves the contained collection, refreshes the default pipeline contract files, and returns imported scene and transition counts.

### `POST /scenes/validate`

Accepts a `SceneCollection` and returns validation issues without saving.

### `GET /scene-runtime`

Returns the in-process scene runtime snapshot loaded from the saved scene collection.

### `POST /scene-runtime/activate`

Accepts a `SceneActivationRequest`, updates the active saved scene/transition when valid, refreshes media planning files, and returns a `SceneActivationResponse`.

### `PUT /scene-runtime/state`

Accepts a `SceneRuntimeStateUpdateRequest`, applies active-scene, active-transition, preview, and status patches, and returns a `SceneRuntimeStateUpdateResponse`.

### `POST /scene-runtime/preview-frame`

Accepts a `PreviewFrameRequest` and returns a `PreviewFrameResponse` with software-rendered preview pixels, frame metadata, checksum, and optional encoded image data. Local `image_media` sources with `media_type = "image"` decode PNG, JPEG, WebP, and first-frame GIF assets into the software compositor. Local `image_media` sources with `media_type = "video"` can extract preview frames from MP4, MOV, WebM, and MKV assets through optional FFmpeg image-pipe decoding. `display` and `window` sources can render macOS preview snapshots through the native `screencapture` path when the source is bound and Screen Recording permission is available. `camera` sources can render macOS preview snapshots through optional FFmpeg/AVFoundation when the selected camera is bound and Camera permission is available. Capture failures remain explicit placeholders. `browser_overlay` sources can render preview snapshots through optional local Chrome, Chromium, or Edge DevTools capture for HTTP, HTTPS, and file URLs. Single-line `text` sources rasterize with the bundled Inter font. Enabled visual source filters apply to software input pixels for color correction, chroma key, crop/pad alpha crop, blur, sharpen, still-image mask/blend, and `.cube` LUT transforms; audio filter families remain non-pixel filters and are evaluated by the audio graph runtime instead. Output pipelines remain placeholder-backed. Image, video, live capture, browser, text, and filter diagnostics report source readiness, fallback behavior, dimensions or bounds, format/font/decoder/browser/provider metadata, checksum, capture/cache state, sampled media time, and filter runtime state where applicable.

### `POST /scene-runtime/program-preview-frame`

Accepts a `ProgramPreviewFrameRequest` and returns a `ProgramPreviewFrameResponse` with software-rendered pixels for the saved active scene at program target resolution/FPS. This uses the same compositor and source runtime as runtime preview, but marks the rendered target as `program` and keeps recording/streaming encoder execution disabled.

### `POST /scene-runtime/transition-preview-frame`

Accepts a `TransitionPreviewFrameRequest` for `cut`, `fade`, `swipe`, and `stinger` transitions and returns a `TransitionPreviewFrameResponse` with a software-rendered preview image, frame timing, progress metadata, checksum, and transition diagnostics. Cut, fade, and swipe render from/to scene pixels directly in the backend transition path. Stinger video frames are extracted from local MP4, MOV, WebM, and MKV assets through optional FFmpeg using the same cache and fallback policy as local video media sources. Missing assets, unsupported extensions, missing FFmpeg, and decode failures remain explicit placeholder states.

### `POST /scene-runtime/validate-graph`

Accepts a `CompositorRenderRequest`, evaluates the render graph contract, and returns a `CompositorRenderResponse`.

### `GET /scene-runtime/bindings`

Returns capture and audio binding contracts for the active saved scene.

### `GET /scene-runtime/capture-providers`

Returns the active scene capture provider runtime snapshot. V1 uses deterministic mocked providers that mirror display, window, camera, microphone, and system-audio bindings, report lifecycle/readiness state, frame shape, latency, dropped-frame counters, and validation warnings, but do not start platform capture.

### `GET /scene-runtime/audio-graph`

Returns the active scene audio graph runtime snapshot with simulated pre-filter and post-filter meter levels, ordered `audio_gain` / `noise_gate` / `compressor` diagnostics, gain, mute, monitor, sync offset, bus state, and validation metadata. This endpoint does not start live audio capture.

### `GET /media/plan`

Returns the current dry-run pipeline plan using the saved capture sources, first recording profile, active scene, and enabled stream destinations.

Also refreshes `pipeline-plan.json` and `pipeline-config.json` in the app data directory. The generated config includes compositor render targets, performance telemetry, and an `output_preflight_plan` with recording/streaming target contracts for dry-run readiness checks.

### `POST /media/plan`

Body:

```json
{
  "dry_run": true,
  "intent": "recording_and_stream",
  "capture_sources": [
    {
      "id": "display:main",
      "kind": "display",
      "name": "Main Display",
      "enabled": true
    }
  ],
  "recording_profile": null,
  "stream_destinations": []
}
```

Returns a `MediaPipelinePlan` with resolved config, ordered steps, warnings, and blocking errors. Stream keys are never included; only secret references may appear inside destination objects.

Also refreshes the media planning files with the supplied request.

### `GET /media/validate`

Returns validation for the current default plan.

### `POST /media/validate`

Accepts the same body as `/media/plan` and returns only `ready`, `warnings`, and `errors`.

### `POST /recording/start`

Body:

```json
{
  "profile_id": "rec_profile_optional"
}
```

If `profile_id` is omitted, the first profile is used.

### `POST /recording/stop`

Stops the active recording if one exists. Repeated calls are safe.

### `POST /stream/start`

Body:

```json
{
  "destination_id": "stream_dest_optional"
}
```

If `destination_id` is omitted, the first enabled destination is used.

### `POST /stream/stop`

Stops the active stream if one exists. Repeated calls are safe.

### `POST /marker/create`

Body:

```json
{
  "label": "Pulse keep: opener",
  "source_app": "vaexcore-pulse",
  "source_event_id": "pulse:session:candidate",
  "recording_session_id": "rec_...",
  "media_path": "/Users/me/Movies/vaexcore studio/clip.mkv",
  "start_seconds": 12.5,
  "end_seconds": 24,
  "metadata": {
    "confidenceBand": "high"
  }
}
```

All fields except `label` are optional. External apps should set `source_app` to a stable app identifier and `source_event_id` to an idempotency-friendly event reference when one exists. If a marker already exists for the same `source_app + source_event_id`, Studio returns the existing marker instead of creating a duplicate. New markers emit `marker.created` with the saved marker payload.

### `GET /profiles`

Returns:

```json
{
  "recording_profiles": [],
  "stream_destinations": []
}
```

### `POST /profiles`

Create recording profile:

```json
{
  "kind": "recording_profile",
  "value": {
    "name": "1080p60 Local",
    "output_folder": "~/Movies/vaexcore studio",
    "filename_pattern": "{date}-{time}-{profile}",
    "container": "mkv",
    "resolution": { "width": 1920, "height": 1080 },
    "framerate": 60,
    "bitrate_kbps": 12000,
    "encoder_preference": "auto"
  }
}
```

Create stream destination:

```json
{
  "kind": "stream_destination",
  "value": {
    "name": "Twitch Primary",
    "platform": "twitch",
    "ingest_url": "rtmp://live.twitch.tv/app",
    "stream_key": "sensitive",
    "enabled": true
  }
}
```

## Events

`WS /events` sends JSON event objects. New connections receive recent events before live events. Pass `?limit=<count>` to control replay count, capped at 100.

```json
{
  "id": "evt_...",
  "type": "recording.started",
  "timestamp": "2026-05-02T12:00:00Z",
  "payload": {}
}
```

Supported event types:

- `app.ready`
- `media.engine.ready`
- `recording.started` with `session_id`, `output_path`, `profile_id`, and the
  active `scene_id`/`scene_name` when a scene collection is available
- `recording.stopped`
- `stream.started` with `session_id`, destination identity, platform, and the
  active `scene_id`/`scene_name` when a scene collection is available
- `stream.stopped`
- `marker.created`
- `error`

## Desktop Preflight

The desktop shell exposes Tauri commands for macOS-first readiness checks:

- `preflight_snapshot`
- `capture_source_inventory`

Those include local API reachability, token mode, writable recording output folder, media-runner health, configured capture sources, and macOS capture permission readiness where it can be safely checked before the real media backend exists.
