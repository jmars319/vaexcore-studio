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

await client.createMarker("manual-marker");
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
- `recording.started`
- `recording.stopped`
- `stream.started`
- `stream.stopped`
- `marker.created`
- `error`
