# @vaexcore/client-sdk

Typed TypeScript client for localhost integrations that talk to vaexcore studio.

```ts
import { VaexcoreStudioClient } from "@vaexcore/client-sdk";

const client = new VaexcoreStudioClient({
  apiUrl: "http://127.0.0.1:51287",
  token: process.env.VAEXCORE_API_TOKEN,
  clientId: "my-control-tool",
  clientName: "My Control Tool",
});

const status = await client.status();
const plan = await client.mediaPlan();
await client.createMarker("manual-marker");
console.log(status.status.recording_active, plan.ready);
```

For WebSocket clients, use `client.eventSocketUrl()` and pass the returned URL to your WebSocket implementation.
