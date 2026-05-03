import { VaexcoreStudioClient } from "../dist/index.js";

const client = new VaexcoreStudioClient({
  apiUrl: process.env.VAEXCORE_API_URL ?? "http://127.0.0.1:51287",
  token: process.env.VAEXCORE_API_TOKEN ?? null,
  clientId: "client-sdk-smoke",
  clientName: "Client SDK Smoke",
});

const health = await client.health();
const status = await client.status();

console.log({
  service: health.service,
  ok: health.ok,
  recordingActive: status.status.recording_active,
  streamActive: status.status.stream_active,
});
