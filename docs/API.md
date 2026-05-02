# Local API

Base URL:

```text
http://127.0.0.1:51287
```

WebSocket URL:

```text
ws://127.0.0.1:51287/events
```

## Auth

Debug builds enable dev auth bypass by default. When auth is required, pass either:

```http
Authorization: Bearer <token>
```

or:

```http
x-vaexcore-token: <token>
```

For WebSocket clients, pass `?token=<token>` or the token header.

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
  "label": "manual-marker"
}
```

Emits `marker.created`.

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

`WS /events` sends JSON event objects:

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
- `recording.started`
- `recording.stopped`
- `stream.started`
- `stream.stopped`
- `marker.created`
- `error`
