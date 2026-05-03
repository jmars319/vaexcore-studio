import type {
  ApiResponse,
  AppSettings,
  AuditLogSnapshot,
  CaptureSourceInventory,
  CommandStatus,
  ConnectedClientsSnapshot,
  CreatedProfile,
  CreateMarkerRequestInput,
  CreateProfileRequest,
  DeletedProfile,
  HealthResponse,
  Marker,
  MediaPipelinePlan,
  MediaProfileInput,
  PreflightSnapshot,
  ProfilesSnapshot,
  RecentRecordingsSnapshot,
  StudioStatus,
  StreamDestinationInput,
} from "@vaexcore/shared-types";

export interface RuntimeApiConfig {
  apiUrl: string;
  wsUrl: string;
  configuredApiUrl: string;
  configuredWsUrl: string;
  bindAddr: string;
  configuredBindAddr: string;
  portFallbackActive: boolean;
  discoveryFile: string;
  token: string | null;
  devAuthBypass: boolean;
}

export interface LocalAppSettingsSnapshot {
  settings: AppSettings;
  apiUrl: string;
  wsUrl: string;
  configuredApiUrl: string;
  configuredWsUrl: string;
  portFallbackActive: boolean;
  dataDir: string;
  databasePath: string;
  discoveryFile: string;
  logDir: string;
  pipelinePlanPath: string;
  pipelineConfigPath: string;
  restartRequired: boolean;
}

export interface MediaRunnerInfo {
  bundled: boolean;
  running: boolean;
  fallbackDryRun: boolean;
  statusAddr: string | null;
  executablePath: string | null;
}

export interface ProfileBundleFileResult {
  path: string;
  recordingProfiles: number;
  streamDestinations: number;
}

export interface PermissionStatus {
  service: string;
  status: "authorized" | "denied" | "restricted" | "not_determined" | "unknown";
  detail: string;
}

const UI_CLIENT_ID = "vaexcore-studio-ui";
const UI_CLIENT_NAME = "vaexcore studio UI";

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
      configuredApiUrl: apiUrl,
      configuredWsUrl: wsUrl,
      bindAddr: new URL(apiUrl).host,
      configuredBindAddr: new URL(apiUrl).host,
      portFallbackActive: false,
      discoveryFile: "",
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

export async function loadCaptureSourceInventory(): Promise<CaptureSourceInventory> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<CaptureSourceInventory>("capture_source_inventory");
}

export async function loadPreflightSnapshot(): Promise<PreflightSnapshot> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PreflightSnapshot>("preflight_snapshot");
}

export async function loadCameraPermissionStatus(): Promise<PermissionStatus> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PermissionStatus>("camera_permission_status");
}

export async function loadMicrophonePermissionStatus(): Promise<PermissionStatus> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<PermissionStatus>("microphone_permission_status");
}

export async function openCameraPrivacySettings(): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("open_camera_privacy_settings");
}

export async function openMicrophonePrivacySettings(): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("open_microphone_privacy_settings");
}

export async function openScreenRecordingPrivacySettings(): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<void>("open_screen_recording_privacy_settings");
}

export async function exportProfileBundle(): Promise<ProfileBundleFileResult> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ProfileBundleFileResult>("export_profile_bundle");
}

export async function importProfileBundle(): Promise<ProfileBundleFileResult> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<ProfileBundleFileResult>("import_profile_bundle");
}

export async function loadMediaRunnerInfo(): Promise<MediaRunnerInfo> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<MediaRunnerInfo>("media_runner_info");
  } catch {
    return {
      bundled: false,
      running: false,
      fallbackDryRun: true,
      statusAddr: null,
      executablePath: null,
    };
  }
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
  headers.set("x-vaexcore-client-id", UI_CLIENT_ID);
  headers.set("x-vaexcore-client-name", UI_CLIENT_NAME);

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
  clients: (config: RuntimeApiConfig) =>
    apiRequest<ConnectedClientsSnapshot>(config, "/clients"),
  auditLog: (config: RuntimeApiConfig) =>
    apiRequest<AuditLogSnapshot>(config, "/audit-log"),
  recentRecordings: (config: RuntimeApiConfig) =>
    apiRequest<RecentRecordingsSnapshot>(config, "/recordings/recent"),
  mediaPlan: (config: RuntimeApiConfig) =>
    apiRequest<MediaPipelinePlan>(config, "/media/plan"),
  profiles: (config: RuntimeApiConfig) =>
    apiRequest<ProfilesSnapshot>(config, "/profiles"),
  createProfile: (config: RuntimeApiConfig, request: CreateProfileRequest) =>
    apiRequest<CreatedProfile>(config, "/profiles", {
      method: "POST",
      body: JSON.stringify(request),
    }),
  updateRecordingProfile: (
    config: RuntimeApiConfig,
    id: string,
    value: MediaProfileInput,
  ) =>
    apiRequest<CreatedProfile>(
      config,
      `/profiles/recording/${encodeURIComponent(id)}`,
      {
        method: "PUT",
        body: JSON.stringify(value),
      },
    ),
  deleteRecordingProfile: (config: RuntimeApiConfig, id: string) =>
    apiRequest<DeletedProfile>(
      config,
      `/profiles/recording/${encodeURIComponent(id)}`,
      { method: "DELETE" },
    ),
  updateStreamDestination: (
    config: RuntimeApiConfig,
    id: string,
    value: StreamDestinationInput,
  ) =>
    apiRequest<CreatedProfile>(
      config,
      `/profiles/destinations/${encodeURIComponent(id)}`,
      {
        method: "PUT",
        body: JSON.stringify(value),
      },
    ),
  deleteStreamDestination: (config: RuntimeApiConfig, id: string) =>
    apiRequest<DeletedProfile>(
      config,
      `/profiles/destinations/${encodeURIComponent(id)}`,
      { method: "DELETE" },
    ),
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
  createMarker: (
    config: RuntimeApiConfig,
    request?: string | CreateMarkerRequestInput,
  ) =>
    apiRequest<Marker>(config, "/marker/create", {
      method: "POST",
      body: JSON.stringify(
        typeof request === "string" ? { label: request } : (request ?? {}),
      ),
    }),
};

export function eventSocketUrl(config: RuntimeApiConfig): string {
  const url = new URL(config.wsUrl);
  url.searchParams.set("client_id", `${UI_CLIENT_ID}-events`);
  url.searchParams.set("client_name", `${UI_CLIENT_NAME} events`);
  if (config.token && !config.devAuthBypass) {
    url.searchParams.set("token", config.token);
  }
  return url.toString();
}
