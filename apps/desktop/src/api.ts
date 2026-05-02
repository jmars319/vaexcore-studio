import type {
  ApiResponse,
  AppSettings,
  CommandStatus,
  CreatedProfile,
  CreateProfileRequest,
  HealthResponse,
  Marker,
  ProfilesSnapshot,
  StudioStatus,
} from "@vaexcore/shared-types";

export interface RuntimeApiConfig {
  apiUrl: string;
  wsUrl: string;
  token: string | null;
  devAuthBypass: boolean;
}

export interface LocalAppSettingsSnapshot {
  settings: AppSettings;
  apiUrl: string;
  wsUrl: string;
  dataDir: string;
  databasePath: string;
  restartRequired: boolean;
}

export async function loadRuntimeConfig(): Promise<RuntimeApiConfig> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<RuntimeApiConfig>("api_config");
  } catch {
    const apiUrl =
      import.meta.env.VITE_VAEXCORE_API_URL ?? "http://127.0.0.1:51287";
    const wsUrl = apiUrl.replace(/^http/, "ws") + "/events";
    return {
      apiUrl,
      wsUrl,
      token: import.meta.env.VITE_VAEXCORE_API_TOKEN ?? null,
      devAuthBypass: true,
    };
  }
}

export async function loadAppSettings(): Promise<LocalAppSettingsSnapshot> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<LocalAppSettingsSnapshot>("app_settings");
}

export async function saveAppSettings(
  settings: AppSettings,
): Promise<LocalAppSettingsSnapshot> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<LocalAppSettingsSnapshot>("save_app_settings", { settings });
}

export async function regenerateApiToken(): Promise<LocalAppSettingsSnapshot> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<LocalAppSettingsSnapshot>("regenerate_api_token");
}

export async function openDataDirectory(): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("open_data_directory");
}

export async function apiRequest<T>(
  config: RuntimeApiConfig,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body && !headers.has("content-type")) {
    headers.set("content-type", "application/json");
  }
  if (config.token) {
    headers.set("x-vaexcore-token", config.token);
  }

  const response = await fetch(`${config.apiUrl}${path}`, {
    ...init,
    headers,
  });
  const body = (await response.json()) as ApiResponse<T>;

  if (!response.ok || !body.ok || body.data === null) {
    throw new Error(body.error?.message ?? `API request failed: ${path}`);
  }

  return body.data;
}

export const StudioApi = {
  health: (config: RuntimeApiConfig) =>
    apiRequest<HealthResponse>(config, "/health"),
  status: (config: RuntimeApiConfig) =>
    apiRequest<StudioStatus>(config, "/status"),
  profiles: (config: RuntimeApiConfig) =>
    apiRequest<ProfilesSnapshot>(config, "/profiles"),
  createProfile: (config: RuntimeApiConfig, request: CreateProfileRequest) =>
    apiRequest<CreatedProfile>(config, "/profiles", {
      method: "POST",
      body: JSON.stringify(request),
    }),
  startRecording: (config: RuntimeApiConfig, profileId?: string) =>
    apiRequest<CommandStatus>(config, "/recording/start", {
      method: "POST",
      body: JSON.stringify({ profile_id: profileId }),
    }),
  stopRecording: (config: RuntimeApiConfig) =>
    apiRequest<CommandStatus>(config, "/recording/stop", { method: "POST" }),
  startStream: (config: RuntimeApiConfig, destinationId?: string) =>
    apiRequest<CommandStatus>(config, "/stream/start", {
      method: "POST",
      body: JSON.stringify({ destination_id: destinationId }),
    }),
  stopStream: (config: RuntimeApiConfig) =>
    apiRequest<CommandStatus>(config, "/stream/stop", { method: "POST" }),
  createMarker: (config: RuntimeApiConfig, label?: string) =>
    apiRequest<Marker>(config, "/marker/create", {
      method: "POST",
      body: JSON.stringify({ label }),
    }),
};

export function eventSocketUrl(config: RuntimeApiConfig): string {
  if (!config.token || config.devAuthBypass) {
    return config.wsUrl;
  }

  const url = new URL(config.wsUrl);
  url.searchParams.set("token", config.token);
  return url.toString();
}
