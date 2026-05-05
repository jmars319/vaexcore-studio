export type PlatformKind = "twitch" | "youtube" | "kick" | "custom_rtmp";

export type RecordingContainer = "mkv" | "mp4";

export type EncoderPreference = "auto" | "hardware" | "software" | { named: string };

export interface Resolution {
  width: number;
  height: number;
}

export interface SecretRef {
  provider: string;
  id: string;
}

export interface MediaProfile {
  id: string;
  name: string;
  output_folder: string;
  filename_pattern: string;
  container: RecordingContainer;
  resolution: Resolution;
  framerate: number;
  bitrate_kbps: number;
  encoder_preference: EncoderPreference;
  created_at: string;
  updated_at: string;
}

export interface MediaProfileInput {
  name: string;
  output_folder: string;
  filename_pattern: string;
  container: RecordingContainer;
  resolution: Resolution;
  framerate: number;
  bitrate_kbps: number;
  encoder_preference: EncoderPreference;
}

export interface AppSettings {
  api_host: string;
  api_port: number;
  api_token: string | null;
  dev_auth_bypass: boolean;
  log_level: "trace" | "debug" | "info" | "warn" | "error";
  default_recording_profile: MediaProfileInput;
  capture_sources: CaptureSourceSelection[];
}

export interface StreamDestination {
  id: string;
  name: string;
  platform: PlatformKind;
  ingest_url: string;
  stream_key_ref: SecretRef | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface StreamDestinationInput {
  name: string;
  platform: PlatformKind;
  ingest_url?: string | null;
  stream_key?: string | null;
  enabled?: boolean | null;
}

export interface RecordingSession {
  id: string;
  profile: MediaProfile;
  output_path: string;
  started_at: string;
}

export interface StreamSession {
  id: string;
  destination: StreamDestination;
  started_at: string;
}

export type EngineMode = "dry_run" | "g_streamer" | "external_sidecar";

export interface EngineStatus {
  engine: string;
  mode: EngineMode;
  recording: RecordingSession | null;
  stream: StreamSession | null;
  recording_active: boolean;
  stream_active: boolean;
  recording_path: string | null;
  active_destination: StreamDestination | null;
  updated_at: string;
}

export type StudioEventType =
  | "app.ready"
  | "media.engine.ready"
  | "recording.started"
  | "recording.stopped"
  | "stream.started"
  | "stream.stopped"
  | "marker.created"
  | "error";

export interface StudioEvent {
  id: string;
  type: StudioEventType;
  timestamp: string;
  payload: Record<string, unknown>;
}

export interface StudioStatus {
  status: EngineStatus;
  recent_events: StudioEvent[];
}

export interface ConnectedClient {
  id: string;
  name: string;
  kind: string;
  user_agent: string | null;
  last_request_id: string | null;
  last_path: string | null;
  request_count: number;
  connected_at: string;
  last_seen_at: string;
}

export interface ConnectedClientsSnapshot {
  clients: ConnectedClient[];
}

export interface AuditLogEntry {
  id: string;
  request_id: string;
  method: string;
  path: string;
  action: string;
  status_code: number;
  ok: boolean;
  client_id: string | null;
  client_name: string | null;
  created_at: string;
}

export interface AuditLogSnapshot {
  entries: AuditLogEntry[];
}

export interface StreamDestinationBundleItem {
  name: string;
  platform: PlatformKind;
  ingest_url: string;
  enabled: boolean;
  has_stream_key: boolean;
}

export interface ProfileBundle {
  version: number;
  exported_at: string;
  recording_profiles: MediaProfileInput[];
  stream_destinations: StreamDestinationBundleItem[];
}

export interface ProfileBundleImportResult {
  recording_profiles: number;
  stream_destinations: number;
}

export interface ProfilesSnapshot {
  recording_profiles: MediaProfile[];
  stream_destinations: StreamDestination[];
}

export interface CommandStatus {
  changed: boolean;
  message: string;
  status: EngineStatus;
}

export interface Marker {
  id: string;
  label: string | null;
  source_app: string | null;
  source_event_id: string | null;
  recording_session_id: string | null;
  media_path: string | null;
  start_seconds: number | null;
  end_seconds: number | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface MarkersSnapshot {
  markers: Marker[];
}

export interface CreateMarkerRequestInput {
  label?: string | null;
  source_app?: string | null;
  source_event_id?: string | null;
  recording_session_id?: string | null;
  media_path?: string | null;
  start_seconds?: number | null;
  end_seconds?: number | null;
  metadata?: Record<string, unknown> | null;
}

export interface RecordingHistoryEntry {
  session_id: string;
  output_path: string;
  profile_id: string;
  profile_name: string;
  started_at: string;
  stopped_at: string;
}

export interface RecentRecordingsSnapshot {
  recordings: RecordingHistoryEntry[];
}

export interface HealthResponse {
  service: string;
  version: string;
  ok: boolean;
  auth_required: boolean;
  dev_auth_bypass: boolean;
  local_runtime: LocalRuntimeHealth;
}

export interface LocalRuntimeHealth {
  contract_version: number;
  mode: "local-first";
  state: "ready" | "degraded" | "blocked";
  app_storage_dir: string;
  suite_dir: string;
  secure_storage: string;
  secret_storage_state: string;
  durable_storage: string[];
  network_policy: "localhost-only";
  dependencies: LocalRuntimeDependency[];
}

export interface LocalRuntimeDependency {
  name: string;
  kind: string;
  state: string;
  detail: string;
}

export type CaptureSourceKind =
  | "display"
  | "window"
  | "camera"
  | "microphone"
  | "system_audio";

export interface CaptureSourceSelection {
  id: string;
  kind: CaptureSourceKind;
  name: string;
  enabled: boolean;
}

export interface CaptureSourceCandidate {
  id: string;
  kind: CaptureSourceKind;
  name: string;
  available: boolean;
  notes: string | null;
}

export interface CaptureSourceInventory {
  candidates: CaptureSourceCandidate[];
  selected: CaptureSourceSelection[];
}

export type PreflightStatus =
  | "ready"
  | "warning"
  | "blocked"
  | "unknown"
  | "not_required";

export interface PreflightCheck {
  id: string;
  label: string;
  status: PreflightStatus;
  detail: string;
}

export interface PreflightSnapshot {
  overall: PreflightStatus;
  checked_at: string;
  checks: PreflightCheck[];
}

export interface ApiErrorBody {
  code: string;
  message: string;
}

export interface ApiResponse<T> {
  ok: boolean;
  data: T | null;
  error: ApiErrorBody | null;
}

export type CreateProfileRequest =
  | { kind: "recording_profile"; value: MediaProfileInput }
  | { kind: "stream_destination"; value: StreamDestinationInput };

export type CreatedProfile =
  | { kind: "recording_profile"; value: MediaProfile }
  | { kind: "stream_destination"; value: StreamDestination };

export type PipelineIntent =
  | "recording"
  | "stream"
  | "recording_and_stream";

export interface MediaPipelineConfig {
  version: number;
  dry_run: boolean;
  intent: PipelineIntent;
  capture_sources: CaptureSourceSelection[];
  recording_profile: MediaProfile | null;
  stream_destinations: StreamDestination[];
}

export interface MediaPipelinePlanRequest {
  dry_run: boolean;
  intent: PipelineIntent;
  capture_sources: CaptureSourceSelection[];
  recording_profile: MediaProfile | null;
  stream_destinations: StreamDestination[];
}

export type PipelineStepStatus = "ready" | "warning" | "blocked";

export interface MediaPipelineStep {
  id: string;
  label: string;
  status: PipelineStepStatus;
  detail: string;
}

export interface MediaPipelinePlan {
  pipeline_name: string;
  dry_run: boolean;
  ready: boolean;
  config: MediaPipelineConfig;
  steps: MediaPipelineStep[];
  warnings: string[];
  errors: string[];
}

export interface MediaPipelineValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export interface DeletedProfile {
  id: string;
  deleted: boolean;
}

export const platformLabels: Record<PlatformKind, string> = {
  twitch: "Twitch",
  youtube: "YouTube",
  kick: "Kick",
  custom_rtmp: "Custom RTMP",
};
