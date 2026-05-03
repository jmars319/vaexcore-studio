import type {
  ApiErrorBody,
  ApiResponse,
  AuditLogSnapshot,
  CommandStatus,
  ConnectedClientsSnapshot,
  CreatedProfile,
  HealthResponse,
  Marker,
  MediaProfileInput,
  ProfilesSnapshot,
  StudioStatus,
  StreamDestinationInput,
} from "@vaexcore/shared-types";

export interface VaexcoreStudioClientOptions {
  apiUrl?: string;
  token?: string | null;
  clientId?: string;
  clientName?: string;
  fetchImpl?: typeof fetch;
}

export interface EventSocketUrlOptions {
  clientId?: string;
  clientName?: string;
  includeToken?: boolean;
  limit?: number;
}

export class VaexcoreApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly response: ApiErrorBody | null;

  constructor(status: number, code: string, message: string, response: ApiErrorBody | null) {
    super(message);
    this.name = "VaexcoreApiError";
    this.status = status;
    this.code = code;
    this.response = response;
  }
}

export class VaexcoreStudioClient {
  readonly apiUrl: string;
  readonly token: string | null;
  readonly clientId: string;
  readonly clientName: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: VaexcoreStudioClientOptions = {}) {
    this.apiUrl = normalizeApiUrl(options.apiUrl ?? "http://127.0.0.1:51287");
    this.token = options.token ?? null;
    this.clientId = options.clientId ?? "vaexcore-client-sdk";
    this.clientName = options.clientName ?? "Vaexcore Client SDK";
    this.fetchImpl = options.fetchImpl ?? globalThis.fetch;

    if (!this.fetchImpl) {
      throw new Error("VaexcoreStudioClient requires a fetch implementation");
    }
  }

  health(): Promise<HealthResponse> {
    return this.request<HealthResponse>("/health");
  }

  status(): Promise<StudioStatus> {
    return this.request<StudioStatus>("/status");
  }

  profiles(): Promise<ProfilesSnapshot> {
    return this.request<ProfilesSnapshot>("/profiles");
  }

  clients(): Promise<ConnectedClientsSnapshot> {
    return this.request<ConnectedClientsSnapshot>("/clients");
  }

  auditLog(): Promise<AuditLogSnapshot> {
    return this.request<AuditLogSnapshot>("/audit-log");
  }

  createRecordingProfile(value: MediaProfileInput): Promise<CreatedProfile> {
    return this.request<CreatedProfile>("/profiles", {
      method: "POST",
      body: JSON.stringify({ kind: "recording_profile", value }),
    });
  }

  createStreamDestination(value: StreamDestinationInput): Promise<CreatedProfile> {
    return this.request<CreatedProfile>("/profiles", {
      method: "POST",
      body: JSON.stringify({ kind: "stream_destination", value }),
    });
  }

  startRecording(profileId?: string): Promise<CommandStatus> {
    return this.request<CommandStatus>("/recording/start", {
      method: "POST",
      body: JSON.stringify({ profile_id: profileId }),
    });
  }

  stopRecording(): Promise<CommandStatus> {
    return this.request<CommandStatus>("/recording/stop", { method: "POST" });
  }

  startStream(destinationId?: string): Promise<CommandStatus> {
    return this.request<CommandStatus>("/stream/start", {
      method: "POST",
      body: JSON.stringify({ destination_id: destinationId }),
    });
  }

  stopStream(): Promise<CommandStatus> {
    return this.request<CommandStatus>("/stream/stop", { method: "POST" });
  }

  createMarker(label?: string): Promise<Marker> {
    return this.request<Marker>("/marker/create", {
      method: "POST",
      body: JSON.stringify({ label }),
    });
  }

  eventSocketUrl(options: EventSocketUrlOptions = {}): string {
    const url = new URL(this.apiUrl.replace(/^http/, "ws"));
    url.pathname = "/events";
    url.searchParams.set("client_id", options.clientId ?? `${this.clientId}-events`);
    url.searchParams.set("client_name", options.clientName ?? `${this.clientName} Events`);

    if (typeof options.limit === "number") {
      url.searchParams.set("limit", String(options.limit));
    }
    if ((options.includeToken ?? true) && this.token) {
      url.searchParams.set("token", this.token);
    }

    return url.toString();
  }

  async request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const headers = new Headers(init.headers);
    if (init.body && !headers.has("content-type")) {
      headers.set("content-type", "application/json");
    }
    if (this.token) {
      headers.set("x-vaexcore-token", this.token);
    }
    headers.set("x-vaexcore-client-id", this.clientId);
    headers.set("x-vaexcore-client-name", this.clientName);

    const response = await this.fetchImpl(`${this.apiUrl}${path}`, {
      ...init,
      headers,
    });
    const body = await parseResponse<T>(response, path);

    if (!response.ok || !body.ok || body.data === null) {
      const error = body.error;
      throw new VaexcoreApiError(
        response.status,
        error?.code ?? "api_error",
        error?.message ?? `Vaexcore Studio API request failed: ${path}`,
        error,
      );
    }

    return body.data;
  }
}

export function createVaexcoreStudioClient(
  options?: VaexcoreStudioClientOptions,
): VaexcoreStudioClient {
  return new VaexcoreStudioClient(options);
}

async function parseResponse<T>(response: Response, path: string): Promise<ApiResponse<T>> {
  const text = await response.text();
  try {
    return JSON.parse(text) as ApiResponse<T>;
  } catch {
    throw new VaexcoreApiError(
      response.status,
      "invalid_response",
      `Vaexcore Studio API returned invalid JSON for ${path}`,
      null,
    );
  }
}

function normalizeApiUrl(apiUrl: string): string {
  return apiUrl.replace(/\/+$/, "");
}
