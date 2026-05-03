# Connected App Contract

vaexcore studio is the local control layer. Companion apps should discover it,
label themselves, and use the HTTP API plus event stream instead of reading
Studio internals.

## Discovery

Prefer the discovery file shown in Studio's Connected Apps screen:

```json
{
  "api_url": "http://127.0.0.1:51287",
  "ws_url": "ws://127.0.0.1:51287/events",
  "token": "optional-token"
}
```

Environment variables may override discovery when a tool is launched outside the
desktop app:

```text
VAEXCORE_STUDIO_API_URL=http://127.0.0.1:51287
VAEXCORE_STUDIO_API_TOKEN=optional-token
```

Browser/Tauri apps that use Vite may also accept:

```text
VITE_VAEXCORE_STUDIO_API_URL=http://127.0.0.1:51287
VITE_VAEXCORE_STUDIO_API_TOKEN=optional-token
```

## Client Labels

Every connected app should send stable labels on HTTP requests:

```http
x-vaexcore-client-id: vaexcore-pulse
x-vaexcore-client-name: vaexcore pulse
```

For WebSocket clients, pass the same labels as query parameters:

```text
ws://127.0.0.1:51287/events?client_id=vaexcore-pulse&client_name=vaexcore%20pulse
```

These labels feed Studio's recent client list and command audit log. They are
not authentication credentials.

## Recording Handoff

Apps that need media should listen to `recording.stopped` and should also poll
`GET /recordings/recent` during startup to recover from missed events.

The recent recording payload uses snake_case:

```json
{
  "session_id": "rec_...",
  "output_path": "/Users/me/Movies/vaexcore studio/clip.mkv",
  "profile_id": "rec_profile_...",
  "profile_name": "1080p60 Local",
  "started_at": "2026-05-02T12:00:00Z",
  "stopped_at": "2026-05-02T12:05:00Z"
}
```

## Result Handoff

Apps should send durable highlights, moments, or bot annotations back through
`POST /marker/create`:

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

`source_app` should be stable across versions. `source_event_id` should be
stable for a source result. Studio uses `source_app + source_event_id` as the
idempotency key and returns the existing marker on repeated submissions.

Apps can verify marker handoff through `GET /markers`:

```text
GET /markers?source_app=vaexcore-pulse
GET /markers?recording_session_id=rec_...
GET /markers?source_event_id=pulse:session:candidate
```
