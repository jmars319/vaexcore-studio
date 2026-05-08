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
  MarkersSnapshot,
  MediaPipelinePlan,
  MediaProfileInput,
  PreflightSnapshot,
  ProfilesSnapshot,
  RecentRecordingsSnapshot,
  SceneCollection,
  SceneCollectionBundle,
  SceneCollectionImportResult,
  SceneValidationResult,
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

export interface SceneCollectionBundleFileResult {
  path: string;
  backupPath: string | null;
  scenes: number;
  transitions: number;
}

export interface PermissionStatus {
  service: string;
  status: "authorized" | "denied" | "restricted" | "not_determined" | "unknown";
  detail: string;
}

export interface SuiteLaunchResult {
  appName: string;
  ok: boolean;
  detail: string;
}

export interface SuiteAppStatus {
  appId: string;
  appName: string;
  launchName: string;
  bundleIdentifier: string;
  installed: boolean;
  running: boolean;
  reachable: boolean;
  stale: boolean;
  discoveryFile: string;
  pid: number | null;
  apiUrl: string | null;
  healthUrl: string | null;
  updatedAt: string | null;
  capabilities: string[];
  suiteSessionId: string | null;
  activity: string | null;
  activityDetail: string | null;
  localRuntime: SuiteLocalRuntime | null;
  detail: string;
}

export interface SuiteLocalRuntime {
  contractVersion: 1;
  mode: "local-first";
  state: "ready" | "degraded" | "blocked";
  appStorageDir: string;
  suiteDir: string;
  secureStorage: string;
  secretStorageState: string;
  durableStorage: string[];
  networkPolicy: "localhost-only";
  dependencies: SuiteLocalRuntimeDependency[];
}

export interface SuiteLocalRuntimeDependency {
  name: string;
  kind: string;
  state: string;
  detail: string;
}

export interface SuiteSession {
  schemaVersion: number;
  sessionId: string;
  title: string;
  status: string;
  ownerApp: string;
  createdAt: string;
  updatedAt: string;
}

export interface SuiteCommand {
  schemaVersion: number;
  commandId: string;
  sourceApp: string;
  sourceAppName: string;
  targetApp: string;
  command: string;
  requestedAt: string;
  payload: Record<string, unknown>;
}

export interface SuiteTimelineEvent {
  schemaVersion: number;
  eventId: string;
  sourceApp: string;
  sourceAppName: string;
  kind: string;
  title: string;
  detail: string;
  createdAt: string;
  metadata: Record<string, unknown>;
}

export interface SuiteTimelineInput {
  kind: string;
  title: string;
  detail: string;
  metadata: Record<string, unknown>;
}

export interface SuiteCommandInput {
  targetApp: string;
  command: string;
  payload: Record<string, unknown>;
}

export interface PulseRecordingHandoffInput {
  sessionId: string;
  outputPath: string;
  profileId: string | null;
  profileName: string | null;
  stoppedAt: string;
}

export interface TwitchStreamKeyImport {
  streamKey: string;
  broadcasterLogin: string | null;
  broadcasterUserId: string | null;
}

export interface TwitchBroadcastReadiness {
  ok: boolean;
  status: "ready" | "attention" | "blocked";
  summary: string;
  nextAction: string;
  generatedAt: string;
  twitch: {
    broadcasterLogin: string | null;
    channelUrl: string | null;
    streamKeyScopeReady: boolean;
  };
  checks: Array<{
    name: string;
    ok: boolean;
    detail: string;
  }>;
}

export interface MarkerListOptions {
  sourceApp?: string;
  sourceEventId?: string;
  recordingSessionId?: string;
  limit?: number;
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

export async function launchVaexcoreSuite(): Promise<SuiteLaunchResult[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<SuiteLaunchResult[]>("launch_vaexcore_suite");
  } catch (error) {
    return [
      {
        appName: "vaexcore suite",
        ok: false,
        detail:
          error instanceof Error
            ? error.message
            : "Launch Suite is only available in the desktop app.",
      },
    ];
  }
}

export async function loadSuiteStatus(): Promise<SuiteAppStatus[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<SuiteAppStatus[]>("suite_status");
  } catch {
    return [];
  }
}

export async function loadSuiteSession(): Promise<SuiteSession | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<SuiteSession | null>("suite_session");
  } catch {
    return null;
  }
}

export async function startSuiteSession(
  title?: string,
): Promise<SuiteSession> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SuiteSession>("start_suite_session", { title });
}

export async function sendSuiteCommand(
  input: SuiteCommandInput,
): Promise<SuiteCommand> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SuiteCommand>("send_suite_command", { input });
}

export async function loadSuiteTimeline(limit = 50): Promise<SuiteTimelineEvent[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<SuiteTimelineEvent[]>("suite_timeline", { limit });
  } catch {
    return [];
  }
}

export async function recordSuiteTimelineEvent(
  input: SuiteTimelineInput,
): Promise<void> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke<void>("append_suite_timeline", { input });
  } catch {
    // Shared suite timeline events are best-effort.
  }
}

export async function handoffRecordingToPulse(
  recording: PulseRecordingHandoffInput,
): Promise<SuiteLaunchResult[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<SuiteLaunchResult[]>("handoff_recording_to_pulse", {
      recording,
    });
  } catch (error) {
    return [
      {
        appName: "vaexcore pulse",
        ok: false,
        detail:
          error instanceof Error
            ? error.message
            : "Pulse handoff is only available in the desktop app.",
      },
    ];
  }
}

export async function fetchTwitchStreamKeyFromConsole(): Promise<TwitchStreamKeyImport> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<TwitchStreamKeyImport>("twitch_stream_key_from_console");
}

export async function fetchTwitchBroadcastReadinessFromConsole(): Promise<TwitchBroadcastReadiness | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<TwitchBroadcastReadiness>(
      "twitch_broadcast_readiness_from_console",
    );
  } catch {
    return null;
  }
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

export async function exportSceneCollectionBundle(): Promise<SceneCollectionBundleFileResult> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SceneCollectionBundleFileResult>("export_scene_collection_bundle");
}

export async function importSceneCollectionBundle(): Promise<SceneCollectionBundleFileResult> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<SceneCollectionBundleFileResult>("import_scene_collection_bundle");
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
  markers: (config: RuntimeApiConfig, options?: MarkerListOptions) =>
    apiRequest<MarkersSnapshot>(config, markerListPath(options)),
  mediaPlan: (config: RuntimeApiConfig) =>
    apiRequest<MediaPipelinePlan>(config, "/media/plan"),
  profiles: (config: RuntimeApiConfig) =>
    apiRequest<ProfilesSnapshot>(config, "/profiles"),
  sceneCollection: (config: RuntimeApiConfig) =>
    apiRequest<SceneCollection>(config, "/scenes"),
  saveSceneCollection: (
    config: RuntimeApiConfig,
    collection: SceneCollection,
  ) =>
    apiRequest<SceneCollection>(config, "/scenes", {
      method: "PUT",
      body: JSON.stringify(collection),
    }),
  exportSceneCollection: (config: RuntimeApiConfig) =>
    apiRequest<SceneCollectionBundle>(config, "/scenes/export"),
  importSceneCollection: (
    config: RuntimeApiConfig,
    bundle: SceneCollectionBundle,
  ) =>
    apiRequest<SceneCollectionImportResult>(config, "/scenes/import", {
      method: "POST",
      body: JSON.stringify(bundle),
    }),
  validateSceneCollection: (
    config: RuntimeApiConfig,
    collection: SceneCollection,
  ) =>
    apiRequest<SceneValidationResult>(config, "/scenes/validate", {
      method: "POST",
      body: JSON.stringify(collection),
    }),
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
  startStream: (
    config: RuntimeApiConfig,
    destinationId?: string,
    bandwidthTest = false,
  ) =>
    apiRequest<CommandStatus>(config, "/stream/start", {
      method: "POST",
      body: JSON.stringify({
        destination_id: destinationId,
        bandwidth_test: bandwidthTest,
      }),
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

function markerListPath(options: MarkerListOptions = {}): string {
  const params = new URLSearchParams();
  if (options.sourceApp) params.set("source_app", options.sourceApp);
  if (options.sourceEventId) params.set("source_event_id", options.sourceEventId);
  if (options.recordingSessionId) {
    params.set("recording_session_id", options.recordingSessionId);
  }
  if (typeof options.limit === "number") params.set("limit", String(options.limit));
  const query = params.toString();
  return query ? `/markers?${query}` : "/markers";
}

export function eventSocketUrl(config: RuntimeApiConfig): string {
  const url = new URL(config.wsUrl);
  url.searchParams.set("client_id", `${UI_CLIENT_ID}-events`);
  url.searchParams.set("client_name", `${UI_CLIENT_NAME} events`);
  if (config.token && !config.devAuthBypass) {
    url.searchParams.set("token", config.token);
  }
  return url.toString();
}
