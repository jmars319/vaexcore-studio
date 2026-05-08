export type PlatformKind = "twitch" | "youtube" | "kick" | "custom_rtmp";

export type RecordingContainer = "mkv" | "mp4";

export type EncoderPreference = "auto" | "hardware" | "software" | { named: string };

export interface Resolution {
  width: number;
  height: number;
}

export interface ScenePoint {
  x: number;
  y: number;
}

export interface SceneSize {
  width: number;
  height: number;
}

export interface SceneCrop {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export type SceneSourceKind =
  | "display"
  | "window"
  | "camera"
  | "audio_meter"
  | "image_media"
  | "browser_overlay"
  | "text"
  | "group";

export type SceneSourceFilterKind =
  | "color_correction"
  | "chroma_key"
  | "crop_pad"
  | "mask_blend"
  | "blur"
  | "sharpen"
  | "lut"
  | "audio_gain"
  | "noise_gate"
  | "compressor";

export interface SceneSourceFilter {
  id: string;
  name: string;
  kind: SceneSourceFilterKind;
  enabled: boolean;
  order: number;
  config: Record<string, unknown>;
}

export type SceneSourceAvailabilityState =
  | "available"
  | "permission_required"
  | "unavailable"
  | "unknown";

export interface SceneSourceAvailability {
  state: SceneSourceAvailabilityState;
  detail: string;
}

export interface DisplaySceneSourceConfig {
  display_id: string | null;
  resolution: Resolution | null;
  capture_cursor: boolean;
  availability: SceneSourceAvailability;
}

export interface WindowSceneSourceConfig {
  window_id: string | null;
  application_name: string | null;
  title: string | null;
  resolution: Resolution | null;
  availability: SceneSourceAvailability;
}

export interface CameraSceneSourceConfig {
  device_id: string | null;
  resolution: Resolution | null;
  framerate: number | null;
  availability: SceneSourceAvailability;
}

export interface AudioMeterSceneSourceConfig {
  device_id: string | null;
  channel: "microphone" | "system" | "mixed";
  meter_style: "bar" | "waveform";
  gain_db: number;
  muted: boolean;
  monitor_enabled: boolean;
  meter_enabled: boolean;
  sync_offset_ms: number;
  availability: SceneSourceAvailability;
}

export interface ImageMediaSceneSourceConfig {
  asset_uri: string | null;
  media_type: "image" | "video";
  loop: boolean;
  availability: SceneSourceAvailability;
}

export interface BrowserOverlaySceneSourceConfig {
  url: string | null;
  viewport: Resolution;
  custom_css: string | null;
  availability: SceneSourceAvailability;
}

export interface TextSceneSourceConfig {
  text: string;
  font_family: string;
  font_size: number;
  color: string;
  align: "left" | "center" | "right";
}

export interface GroupSceneSourceConfig {
  child_source_ids: string[];
}

export type SceneSourceConfig =
  | DisplaySceneSourceConfig
  | WindowSceneSourceConfig
  | CameraSceneSourceConfig
  | AudioMeterSceneSourceConfig
  | ImageMediaSceneSourceConfig
  | BrowserOverlaySceneSourceConfig
  | TextSceneSourceConfig
  | GroupSceneSourceConfig;

export interface SceneSourceBase<
  Kind extends SceneSourceKind,
  Config extends SceneSourceConfig,
> {
  id: string;
  name: string;
  kind: Kind;
  position: ScenePoint;
  size: SceneSize;
  crop: SceneCrop;
  rotation_degrees: number;
  opacity: number;
  visible: boolean;
  locked: boolean;
  z_index: number;
  filters: SceneSourceFilter[];
  config: Config;
}

export type SceneSource =
  | SceneSourceBase<"display", DisplaySceneSourceConfig>
  | SceneSourceBase<"window", WindowSceneSourceConfig>
  | SceneSourceBase<"camera", CameraSceneSourceConfig>
  | SceneSourceBase<"audio_meter", AudioMeterSceneSourceConfig>
  | SceneSourceBase<"image_media", ImageMediaSceneSourceConfig>
  | SceneSourceBase<"browser_overlay", BrowserOverlaySceneSourceConfig>
  | SceneSourceBase<"text", TextSceneSourceConfig>
  | SceneSourceBase<"group", GroupSceneSourceConfig>;

export interface SceneCanvas {
  width: number;
  height: number;
  background_color: string;
}

export interface Scene {
  id: string;
  name: string;
  canvas: SceneCanvas;
  sources: SceneSource[];
}

export type SceneTransitionKind = "cut" | "fade" | "swipe" | "stinger";

export type SceneTransitionEasing = "linear" | "ease_in" | "ease_out" | "ease_in_out";

export interface SceneTransition {
  id: string;
  name: string;
  kind: SceneTransitionKind;
  duration_ms: number;
  easing: SceneTransitionEasing;
  config: Record<string, unknown>;
}

export interface SceneTransitionPreviewSample {
  frame_index: number;
  elapsed_ms: number;
  linear_progress: number;
  eased_progress: number;
}

export interface SceneTransitionPreviewPlan {
  version: number;
  transition: SceneTransition;
  from_scene_id: string;
  from_scene_name: string;
  to_scene_id: string;
  to_scene_name: string;
  framerate: number;
  duration_ms: number;
  frame_count: number;
  sample_frames: SceneTransitionPreviewSample[];
  validation: SceneTransitionPreviewValidation;
}

export interface SceneTransitionPreviewValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export interface SceneCollection {
  id: string;
  name: string;
  version: number;
  active_scene_id: string;
  active_transition_id: string;
  transitions: SceneTransition[];
  scenes: Scene[];
  created_at: string;
  updated_at: string;
}

export interface SceneCollectionBundle {
  version: number;
  exported_at: string;
  collection: SceneCollection;
}

export interface SceneCollectionImportResult {
  imported_scenes: number;
  imported_transitions: number;
  collection: SceneCollection;
}

export type SceneCollectionBundleInput = Partial<
  Omit<SceneCollectionBundle, "collection">
> & {
  collection?: Partial<SceneCollection> | null;
};

export interface SceneValidationIssue {
  path: string;
  message: string;
}

export interface SceneValidationResult {
  ok: boolean;
  issues: SceneValidationIssue[];
}

export type CompositorNodeRole = "video" | "audio" | "overlay" | "text" | "group";

export type CompositorNodeStatus =
  | "ready"
  | "placeholder"
  | "permission_required"
  | "unavailable"
  | "hidden";

export type CompositorBlendMode = "normal";

export type CompositorScaleMode = "stretch" | "fit" | "fill" | "original_size";

export interface CompositorOutput {
  width: number;
  height: number;
  background_color: string;
}

export interface CompositorTransform {
  position: ScenePoint;
  size: SceneSize;
  crop: SceneCrop;
  rotation_degrees: number;
  opacity: number;
}

export interface CompositorNode {
  id: string;
  source_id: string;
  name: string;
  source_kind: SceneSourceKind;
  role: CompositorNodeRole;
  parent_source_id?: string | null;
  group_depth: number;
  transform: CompositorTransform;
  visible: boolean;
  locked: boolean;
  z_index: number;
  blend_mode: CompositorBlendMode;
  scale_mode: CompositorScaleMode;
  status: CompositorNodeStatus;
  status_detail: string;
  filters: SceneSourceFilter[];
  config: SceneSourceConfig;
}

export interface CompositorGraph {
  version: number;
  scene_id: string;
  scene_name: string;
  output: CompositorOutput;
  nodes: CompositorNode[];
}

export type CompositorRendererKind = "contract" | "software" | "gpu";

export type CompositorRenderTargetKind =
  | "preview"
  | "program"
  | "recording"
  | "stream";

export type CompositorFrameFormat = "rgba8" | "bgra8" | "nv12";

export interface CompositorRenderTarget {
  id: string;
  name: string;
  kind: CompositorRenderTargetKind;
  width: number;
  height: number;
  framerate: number;
  frame_format: CompositorFrameFormat;
  scale_mode: CompositorScaleMode;
  enabled: boolean;
}

export interface CompositorRenderPlan {
  version: number;
  renderer: CompositorRendererKind;
  graph: CompositorGraph;
  targets: CompositorRenderTarget[];
}

export interface PerformanceTargetBudget {
  target_id: string;
  target_name: string;
  target_kind: CompositorRenderTargetKind;
  width: number;
  height: number;
  framerate: number;
  frame_budget_nanos: number;
  render_budget_nanos: number;
  encode_budget_nanos: number;
  max_latency_ms: number;
  max_dropped_frames_per_minute: number;
  pixel_count: number;
  estimated_rgba_bytes_per_frame: number;
  estimated_rgba_bytes_per_second: number;
}

export interface PerformanceTelemetryPlan {
  version: number;
  scene_id: string;
  scene_name: string;
  sample_window_seconds: number;
  cpu_warning_percent: number;
  gpu_warning_percent: number;
  targets: PerformanceTargetBudget[];
  validation: PerformanceTelemetryValidation;
}

export interface PerformanceTelemetryValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export interface CompositorFrameClock {
  frame_index: number;
  framerate: number;
  pts_nanos: number;
  duration_nanos: number;
}

export interface CompositorRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface CompositorEvaluatedNode {
  node_id: string;
  source_id: string;
  name: string;
  role: CompositorNodeRole;
  status: CompositorNodeStatus;
  rect: CompositorRect;
  crop: SceneCrop;
  rotation_degrees: number;
  opacity: number;
  z_index: number;
}

export interface CompositorRenderedTarget {
  target_id: string;
  target_kind: CompositorRenderTargetKind;
  width: number;
  height: number;
  frame_format: CompositorFrameFormat;
  nodes: CompositorEvaluatedNode[];
}

export interface CompositorRenderedFrame {
  renderer: CompositorRendererKind;
  scene_id: string;
  scene_name: string;
  clock: CompositorFrameClock;
  targets: CompositorRenderedTarget[];
  validation: CompositorValidation;
}

export interface CompositorValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export interface SceneSourceDefaults {
  id?: string;
  name?: string;
  position?: Partial<ScenePoint>;
  size?: Partial<SceneSize>;
  crop?: Partial<SceneCrop>;
  rotation_degrees?: number;
  opacity?: number;
  visible?: boolean;
  locked?: boolean;
  z_index?: number;
  filters?: SceneSourceFilter[];
  config?: Partial<SceneSourceConfig>;
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

export type CaptureFrameMediaKind = "video" | "audio";

export type CaptureFrameFormat = "rgba8" | "bgra8" | "nv12" | "pcm_f32" | "pcm_s16";

export type CaptureFrameTransport =
  | "unavailable"
  | "shared_memory"
  | "texture_handle"
  | "external_process";

export type CaptureFrameBindingStatus =
  | "ready"
  | "placeholder"
  | "permission_required"
  | "unavailable";

export interface CaptureFrameBinding {
  scene_source_id: string;
  scene_source_name: string;
  capture_source_id: string | null;
  capture_kind: CaptureSourceKind;
  media_kind: CaptureFrameMediaKind;
  width: number | null;
  height: number | null;
  framerate: number | null;
  sample_rate: number | null;
  channels: number | null;
  format: CaptureFrameFormat;
  transport: CaptureFrameTransport;
  status: CaptureFrameBindingStatus;
  status_detail: string;
}

export interface CaptureFramePlan {
  version: number;
  scene_id: string;
  scene_name: string;
  bindings: CaptureFrameBinding[];
  validation: CaptureFrameValidation;
}

export interface CaptureFrameValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export type AudioMixBusKind = "master" | "monitor" | "recording" | "stream";

export type AudioMixSourceStatus =
  | "ready"
  | "placeholder"
  | "permission_required"
  | "unavailable";

export interface AudioMixBus {
  id: string;
  name: string;
  kind: AudioMixBusKind;
  sample_rate: number;
  channels: number;
  gain_db: number;
  muted: boolean;
}

export interface AudioMixSource {
  scene_source_id: string;
  name: string;
  capture_source_id: string | null;
  capture_kind: CaptureSourceKind;
  gain_db: number;
  muted: boolean;
  monitor_enabled: boolean;
  meter_enabled: boolean;
  sync_offset_ms: number;
  status: AudioMixSourceStatus;
  status_detail: string;
}

export interface AudioMixerPlan {
  version: number;
  scene_id: string;
  scene_name: string;
  sample_rate: number;
  channels: number;
  sources: AudioMixSource[];
  buses: AudioMixBus[];
  validation: AudioMixerValidation;
}

export interface AudioMixerValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
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
  active_scene?: Scene | null;
  capture_frame_plan?: CaptureFramePlan | null;
  audio_mixer_plan?: AudioMixerPlan | null;
  compositor_graph?: CompositorGraph | null;
  compositor_render_plan?: CompositorRenderPlan | null;
  performance_telemetry_plan?: PerformanceTelemetryPlan | null;
  recording_profile: MediaProfile | null;
  stream_destinations: StreamDestination[];
}

export interface MediaPipelinePlanRequest {
  dry_run: boolean;
  intent: PipelineIntent;
  capture_sources: CaptureSourceSelection[];
  active_scene?: Scene | null;
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

export const sceneSourceKindLabels: Record<SceneSourceKind, string> = {
  display: "Display",
  window: "Window",
  camera: "Camera",
  audio_meter: "Microphone / Audio Meter",
  image_media: "Image / Media",
  browser_overlay: "Browser Overlay",
  text: "Text",
  group: "Group",
};

export const defaultSceneCanvas: SceneCanvas = {
  width: 1920,
  height: 1080,
  background_color: "#050711",
};

const defaultSceneTransitions: SceneTransition[] = [
  {
    id: "transition-cut",
    name: "Cut",
    kind: "cut",
    duration_ms: 0,
    easing: "linear",
    config: {},
  },
  {
    id: "transition-fade",
    name: "Fade",
    kind: "fade",
    duration_ms: 300,
    easing: "ease_in_out",
    config: { color: "#000000" },
  },
];

const emptyCrop: SceneCrop = {
  top: 0,
  right: 0,
  bottom: 0,
  left: 0,
};

function cloneJson<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function defaultAvailability(
  state: SceneSourceAvailabilityState,
  detail: string,
): SceneSourceAvailability {
  return { state, detail };
}

export function defaultSceneSourceConfig(
  kind: SceneSourceKind,
): SceneSourceConfig {
  switch (kind) {
    case "display":
      return {
        display_id: null,
        resolution: { width: 1920, height: 1080 },
        capture_cursor: true,
        availability: defaultAvailability(
          "permission_required",
          "Screen Recording permission has not been verified.",
        ),
      };
    case "window":
      return {
        window_id: null,
        application_name: null,
        title: null,
        resolution: null,
        availability: defaultAvailability(
          "unknown",
          "Window inventory has not been loaded.",
        ),
      };
    case "camera":
      return {
        device_id: null,
        resolution: { width: 1280, height: 720 },
        framerate: 30,
        availability: defaultAvailability(
          "permission_required",
          "Camera permission has not been verified.",
        ),
      };
    case "audio_meter":
      return {
        device_id: null,
        channel: "microphone",
        meter_style: "bar",
        gain_db: 0,
        muted: false,
        monitor_enabled: false,
        meter_enabled: true,
        sync_offset_ms: 0,
        availability: defaultAvailability(
          "permission_required",
          "Microphone permission has not been verified.",
        ),
      };
    case "image_media":
      return {
        asset_uri: null,
        media_type: "image",
        loop: true,
        availability: defaultAvailability(
          "unavailable",
          "No local media asset has been selected.",
        ),
      };
    case "browser_overlay":
      return {
        url: null,
        viewport: { width: 1280, height: 720 },
        custom_css: null,
        availability: defaultAvailability(
          "unavailable",
          "No browser overlay URL has been configured.",
        ),
      };
    case "text":
      return {
        text: "Starting Soon",
        font_family: "Inter",
        font_size: 72,
        color: "#f4f8ff",
        align: "center",
      };
    case "group":
      return {
        child_source_ids: [],
      };
  }
}

export function createDefaultSceneSource(
  kind: SceneSourceKind,
  defaults: SceneSourceDefaults = {},
): SceneSource {
  const config = {
    ...(defaultSceneSourceConfig(kind) as object),
    ...((defaults.config ?? {}) as object),
  } as SceneSourceConfig;

  return {
    id: defaults.id ?? `source-${kind}-${Date.now()}`,
    name: defaults.name ?? sceneSourceKindLabels[kind],
    kind,
    position: { x: 0, y: 0, ...defaults.position },
    size: { width: 640, height: 360, ...defaults.size },
    crop: { ...emptyCrop, ...defaults.crop },
    rotation_degrees: defaults.rotation_degrees ?? 0,
    opacity: defaults.opacity ?? 1,
    visible: defaults.visible ?? true,
    locked: defaults.locked ?? false,
    z_index: defaults.z_index ?? 0,
    filters: cloneJson(defaults.filters ?? []),
    config,
  } as SceneSource;
}

export function createDefaultSceneCollection(now = new Date().toISOString()): SceneCollection {
  const scene: Scene = {
    id: "scene-main",
    name: "Main Scene",
    canvas: { ...defaultSceneCanvas },
    sources: [
      createDefaultSceneSource("display", {
        id: "source-main-display",
        name: "Main Display Placeholder",
        position: { x: 0, y: 0 },
        size: { width: 1920, height: 1080 },
        z_index: 0,
        config: {
          display_id: "display:main",
          resolution: { width: 1920, height: 1080 },
        },
      }),
      createDefaultSceneSource("camera", {
        id: "source-camera-placeholder",
        name: "Camera Placeholder",
        position: { x: 1460, y: 700 },
        size: { width: 380, height: 214 },
        z_index: 10,
      }),
      createDefaultSceneSource("audio_meter", {
        id: "source-mic-meter",
        name: "Microphone Meter",
        position: { x: 80, y: 900 },
        size: { width: 420, height: 72 },
        z_index: 20,
      }),
      createDefaultSceneSource("browser_overlay", {
        id: "source-alert-overlay",
        name: "Alerts Browser Overlay",
        position: { x: 1240, y: 72 },
        size: { width: 560, height: 170 },
        z_index: 30,
      }),
      createDefaultSceneSource("text", {
        id: "source-title-text",
        name: "Scene Title",
        position: { x: 640, y: 84 },
        size: { width: 640, height: 110 },
        z_index: 40,
        config: {
          text: "vaexcore studio",
          font_size: 64,
        },
      }),
    ],
  };

  return {
    id: "collection-default",
    name: "Default Studio Scenes",
    version: 1,
    active_scene_id: scene.id,
    active_transition_id: "transition-fade",
    transitions: cloneJson(defaultSceneTransitions),
    scenes: [scene],
    created_at: now,
    updated_at: now,
  };
}

export function normalizeSceneCollection(
  collection: Partial<SceneCollection> | null | undefined,
): SceneCollection {
  if (!collection) {
    return createDefaultSceneCollection();
  }

  const fallback = createDefaultSceneCollection(collection.updated_at);
  const scenes = (collection.scenes?.length ? collection.scenes : fallback.scenes).map(
    (scene, sceneIndex) => ({
      id: scene.id || `scene-${sceneIndex + 1}`,
      name: scene.name || `Scene ${sceneIndex + 1}`,
      canvas: {
        ...defaultSceneCanvas,
        ...scene.canvas,
      },
      sources: (scene.sources ?? []).map((source, sourceIndex) =>
        createDefaultSceneSource(source.kind, {
          ...source,
          id: source.id || `source-${sceneIndex + 1}-${sourceIndex + 1}`,
          name: source.name || sceneSourceKindLabels[source.kind],
          config: source.config,
        }),
      ),
    }),
  );

  const activeSceneId =
    collection.active_scene_id && scenes.some((scene) => scene.id === collection.active_scene_id)
      ? collection.active_scene_id
      : scenes[0].id;
  const transitions = (collection.transitions?.length
    ? collection.transitions
    : fallback.transitions
  ).map((transition, transitionIndex) => ({
    id: transition.id || `transition-${transitionIndex + 1}`,
    name: transition.name || `Transition ${transitionIndex + 1}`,
    kind: transition.kind || "fade",
    duration_ms: transition.duration_ms ?? 300,
    easing: transition.easing || "ease_in_out",
    config: transition.config ?? {},
  })) satisfies SceneTransition[];
  const activeTransitionId =
    collection.active_transition_id &&
    transitions.some((transition) => transition.id === collection.active_transition_id)
      ? collection.active_transition_id
      : transitions[0].id;

  return {
    id: collection.id || fallback.id,
    name: collection.name || fallback.name,
    version: collection.version || fallback.version,
    active_scene_id: activeSceneId,
    active_transition_id: activeTransitionId,
    transitions,
    scenes,
    created_at: collection.created_at || fallback.created_at,
    updated_at: collection.updated_at || fallback.updated_at,
  };
}

export function createSceneCollectionBundle(
  collection: SceneCollection,
  exportedAt = new Date().toISOString(),
): SceneCollectionBundle {
  return {
    version: 1,
    exported_at: exportedAt,
    collection: cloneJson(collection),
  };
}

export function normalizeSceneCollectionBundle(
  bundle: SceneCollectionBundleInput | null | undefined,
): SceneCollectionBundle {
  return {
    version: bundle?.version || 1,
    exported_at: bundle?.exported_at || new Date().toISOString(),
    collection: normalizeSceneCollection(bundle?.collection),
  };
}

export function buildSceneTransitionPreviewPlan(
  collection: SceneCollection,
  fromSceneId: string | null = null,
  toSceneId: string | null = null,
  framerate = 60,
): SceneTransitionPreviewPlan {
  const fallbackScene =
    collection.scenes.find((scene) => scene.id === collection.active_scene_id) ??
    collection.scenes[0];
  const fromScene =
    collection.scenes.find((scene) => scene.id === fromSceneId) ?? fallbackScene;
  const toScene =
    collection.scenes.find((scene) => scene.id === toSceneId) ?? fallbackScene;
  const transition =
    collection.transitions.find(
      (item) => item.id === collection.active_transition_id,
    ) ?? collection.transitions[0] ?? defaultSceneTransitions[0];
  const frameCount = transitionFrameCount(transition.duration_ms, framerate);
  const plan: SceneTransitionPreviewPlan = {
    version: 1,
    transition: cloneJson(transition),
    from_scene_id: fromScene?.id ?? "",
    from_scene_name: fromScene?.name ?? "",
    to_scene_id: toScene?.id ?? "",
    to_scene_name: toScene?.name ?? "",
    framerate,
    duration_ms: transition.duration_ms,
    frame_count: frameCount,
    sample_frames: transitionSampleFrames(transition, frameCount, framerate),
    validation: {
      ready: true,
      warnings: [],
      errors: [],
    },
  };
  plan.validation = validateSceneTransitionPreviewPlan(plan);
  return plan;
}

export function validateSceneTransitionPreviewPlan(
  plan: SceneTransitionPreviewPlan,
): SceneTransitionPreviewValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  if (!Number.isInteger(plan.version) || plan.version < 1) {
    errors.push("Transition preview plan version must be a positive integer.");
  }
  if (!plan.transition.id.trim()) {
    errors.push("Transition preview transition id is required.");
  }
  if (!plan.from_scene_id.trim()) {
    errors.push("Transition preview from scene id is required.");
  }
  if (!plan.to_scene_id.trim()) {
    errors.push("Transition preview to scene id is required.");
  }
  if (!Number.isInteger(plan.framerate) || plan.framerate < 1) {
    errors.push("Transition preview framerate must be greater than zero.");
  }
  if (!Number.isInteger(plan.frame_count) || plan.frame_count < 1) {
    errors.push("Transition preview frame count must be greater than zero.");
  }
  if (plan.duration_ms > 60_000) {
    errors.push("Transition preview duration must be 60 seconds or less.");
  }
  if (plan.transition.kind === "cut" && plan.duration_ms !== 0) {
    errors.push("Cut transition preview duration must be zero.");
  }
  if (plan.from_scene_id === plan.to_scene_id) {
    warnings.push("Transition preview uses the same from and to scene.");
  }

  for (const sample of plan.sample_frames) {
    if (sample.linear_progress < 0 || sample.linear_progress > 1) {
      errors.push("Transition preview linear progress must be 0-1.");
    }
    if (sample.eased_progress < 0 || sample.eased_progress > 1) {
      errors.push("Transition preview eased progress must be 0-1.");
    }
  }

  return {
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

function transitionFrameCount(durationMs: number, framerate: number): number {
  if (durationMs === 0 || framerate <= 0) return 1;
  return Math.max(1, Math.ceil((durationMs * framerate) / 1000));
}

function transitionSampleFrames(
  transition: SceneTransition,
  frameCount: number,
  framerate: number,
): SceneTransitionPreviewSample[] {
  const indices = [
    ...new Set([0, Math.floor(frameCount / 2), Math.max(0, frameCount - 1)]),
  ].sort((left, right) => left - right);
  return indices.map((frameIndex) => {
    const linearProgress = frameCount <= 1 ? 1 : frameIndex / (frameCount - 1);
    return {
      frame_index: frameIndex,
      elapsed_ms: framerate <= 0 ? 0 : Math.floor((frameIndex * 1000) / framerate),
      linear_progress: linearProgress,
      eased_progress: transitionEasedProgress(linearProgress, transition.easing),
    };
  });
}

function transitionEasedProgress(
  progress: number,
  easing: SceneTransitionEasing,
): number {
  const value = Math.min(1, Math.max(0, progress));
  switch (easing) {
    case "linear":
      return value;
    case "ease_in":
      return value * value;
    case "ease_out":
      return 1 - (1 - value) * (1 - value);
    case "ease_in_out":
      return value < 0.5
        ? 2 * value * value
        : 1 - Math.pow(-2 * value + 2, 2) / 2;
  }
}

export function bindSceneCollectionCaptureInventory(
  collection: SceneCollection,
  inventory: CaptureSourceInventory | null | undefined,
): SceneCollection {
  if (!inventory) {
    return collection;
  }

  return {
    ...collection,
    scenes: collection.scenes.map((scene) => ({
      ...scene,
      sources: scene.sources.map((source) => bindSceneSourceCaptureInventory(source, inventory)),
    })),
  };
}

function bindSceneSourceCaptureInventory(
  source: SceneSource,
  inventory: CaptureSourceInventory,
): SceneSource {
  switch (source.kind) {
    case "display":
      return bindCaptureCandidate(source, inventory, ["display"], "display_id", "display");
    case "window":
      return bindCaptureCandidate(source, inventory, ["window"], "window_id", "window");
    case "camera":
      return bindCaptureCandidate(source, inventory, ["camera"], "device_id", "camera");
    case "audio_meter":
      return bindCaptureCandidate(
        source,
        inventory,
        ["microphone", "system_audio"],
        "device_id",
        "audio device",
      );
    default:
      return source;
  }
}

function bindCaptureCandidate<Source extends SceneSource>(
  source: Source,
  inventory: CaptureSourceInventory,
  candidateKinds: CaptureSourceKind[],
  configKey: string,
  label: string,
): Source {
  const config = source.config as unknown as Record<string, unknown>;
  const configuredId = typeof config[configKey] === "string" ? String(config[configKey]) : "";
  const candidates = inventory.candidates.filter((candidate) =>
    candidateKinds.includes(candidate.kind),
  );
  const candidate = configuredId
    ? candidates.find((item) => item.id === configuredId)
    : undefined;
  const availability = candidate
    ? {
        state: candidate.available ? "available" : "unavailable",
        detail: candidate.available
          ? `${candidate.name} is available.`
          : (candidate.notes ?? `${candidate.name} is not available.`),
      }
    : {
        state: candidates.some((item) => item.available) ? "unknown" : "unavailable",
        detail: configuredId
          ? `Configured ${label} "${configuredId}" was not found in the current inventory.`
          : `No ${label} has been assigned.`,
      };

  return {
    ...source,
    config: {
      ...source.config,
      availability,
    },
  } as Source;
}

export function buildCaptureFramePlan(scene: Scene): CaptureFramePlan {
  const bindings = scene.sources
    .filter((source) => source.visible)
    .map(captureFrameBinding)
    .filter((binding): binding is CaptureFrameBinding => Boolean(binding));
  const plan: CaptureFramePlan = {
    version: 1,
    scene_id: scene.id,
    scene_name: scene.name,
    bindings,
    validation: {
      ready: true,
      warnings: [],
      errors: [],
    },
  };
  plan.validation = validateCaptureFramePlan(plan);
  return plan;
}

export function validateCaptureFramePlan(
  plan: CaptureFramePlan,
): CaptureFrameValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  if (!Number.isInteger(plan.version) || plan.version < 1) {
    errors.push("Capture frame plan version must be a positive integer.");
  }
  if (!plan.scene_id.trim()) {
    errors.push("Capture frame plan scene id is required.");
  }
  if (!plan.scene_name.trim()) {
    errors.push("Capture frame plan scene name is required.");
  }
  if (plan.bindings.length === 0) {
    warnings.push("Capture frame plan has no capture-backed scene sources.");
  }

  plan.bindings.forEach((binding) => {
    if (!binding.scene_source_id.trim()) {
      errors.push("Capture frame binding scene source id is required.");
    }
    if (!binding.scene_source_name.trim()) {
      errors.push(`Capture frame binding "${binding.scene_source_id}" name is required.`);
    }
    if (!binding.capture_source_id) {
      warnings.push(`${binding.scene_source_name} has no assigned capture source.`);
    }
    if (binding.media_kind === "video") {
      validateNullablePositiveNumber(binding.width, `${binding.scene_source_id}.width`, errors);
      validateNullablePositiveNumber(binding.height, `${binding.scene_source_id}.height`, errors);
      validateNullablePositiveNumber(
        binding.framerate,
        `${binding.scene_source_id}.framerate`,
        errors,
      );
    } else {
      validateNullablePositiveNumber(
        binding.sample_rate,
        `${binding.scene_source_id}.sample_rate`,
        errors,
      );
      validateNullablePositiveNumber(binding.channels, `${binding.scene_source_id}.channels`, errors);
    }

    if (binding.status === "placeholder") {
      warnings.push(
        `${binding.scene_source_name} is waiting for capture assignment: ${binding.status_detail}`,
      );
    } else if (binding.status === "permission_required") {
      warnings.push(
        `${binding.scene_source_name} requires capture permission: ${binding.status_detail}`,
      );
    } else if (binding.status === "unavailable") {
      warnings.push(
        `${binding.scene_source_name} capture is unavailable: ${binding.status_detail}`,
      );
    }
  });

  return {
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

function captureFrameBinding(source: SceneSource): CaptureFrameBinding | null {
  const captureKind = sceneCaptureKind(source);
  if (!captureKind) return null;

  const mediaKind: CaptureFrameMediaKind =
    captureKind === "display" || captureKind === "window" || captureKind === "camera"
      ? "video"
      : "audio";
  const captureSourceId = sceneSourceCaptureIdentity(source);
  const { status, detail } = captureBindingStatus(source, captureSourceId);
  const videoShape = sourceVideoShape(source);
  const audioShape = sourceAudioShape(source);

  return {
    scene_source_id: source.id,
    scene_source_name: source.name,
    capture_source_id: captureSourceId,
    capture_kind: captureKind,
    media_kind: mediaKind,
    width: mediaKind === "video" ? videoShape.width : null,
    height: mediaKind === "video" ? videoShape.height : null,
    framerate: mediaKind === "video" ? videoShape.framerate : null,
    sample_rate: mediaKind === "audio" ? audioShape.sampleRate : null,
    channels: mediaKind === "audio" ? audioShape.channels : null,
    format: mediaKind === "video" ? "bgra8" : "pcm_f32",
    transport: status === "ready" ? "shared_memory" : "unavailable",
    status,
    status_detail: detail,
  };
}

function sceneCaptureKind(source: SceneSource): CaptureSourceKind | null {
  switch (source.kind) {
    case "display":
      return "display";
    case "window":
      return "window";
    case "camera":
      return "camera";
    case "audio_meter":
      return source.config.channel === "system" ? "system_audio" : "microphone";
    default:
      return null;
  }
}

function sceneSourceCaptureIdentity(source: SceneSource): string | null {
  switch (source.kind) {
    case "display":
      return source.config.display_id;
    case "window":
      return source.config.window_id;
    case "camera":
    case "audio_meter":
      return source.config.device_id;
    default:
      return null;
  }
}

function captureBindingStatus(
  source: SceneSource,
  captureSourceId: string | null,
): { status: CaptureFrameBindingStatus; detail: string } {
  if (!captureSourceId) {
    return { status: "placeholder", detail: "No capture source has been assigned." };
  }

  const availability =
    "availability" in source.config ? source.config.availability : null;
  if (!availability) {
    return { status: "ready", detail: "Capture source is configured." };
  }
  if (availability.state === "available") {
    return { status: "ready", detail: availability.detail };
  }
  if (availability.state === "permission_required") {
    return { status: "permission_required", detail: availability.detail };
  }
  if (availability.state === "unavailable") {
    return { status: "unavailable", detail: availability.detail };
  }
  return { status: "placeholder", detail: availability.detail };
}

function sourceVideoShape(source: SceneSource): {
  width: number;
  height: number;
  framerate: number;
} {
  const config = source.config as
    | DisplaySceneSourceConfig
    | WindowSceneSourceConfig
    | CameraSceneSourceConfig;
  return {
    width: config.resolution?.width ?? Math.max(1, Math.round(source.size.width)),
    height: config.resolution?.height ?? Math.max(1, Math.round(source.size.height)),
    framerate: "framerate" in config && config.framerate ? config.framerate : 60,
  };
}

function sourceAudioShape(source: SceneSource): {
  sampleRate: number;
  channels: number;
} {
  const config = source.config as AudioMeterSceneSourceConfig & {
    sample_rate?: number;
    channels?: number;
  };
  return {
    sampleRate: config.sample_rate ?? 48_000,
    channels: config.channels ?? 2,
  };
}

export function buildAudioMixerPlan(scene: Scene): AudioMixerPlan {
  const sources = scene.sources
    .filter(
      (source): source is Extract<SceneSource, { kind: "audio_meter" }> =>
        source.visible && source.kind === "audio_meter",
    )
    .map(audioMixSource);
  const plan: AudioMixerPlan = {
    version: 1,
    scene_id: scene.id,
    scene_name: scene.name,
    sample_rate: 48_000,
    channels: 2,
    sources,
    buses: defaultAudioBuses(),
    validation: {
      ready: true,
      warnings: [],
      errors: [],
    },
  };
  plan.validation = validateAudioMixerPlan(plan);
  return plan;
}

export function validateAudioMixerPlan(plan: AudioMixerPlan): AudioMixerValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  if (!Number.isInteger(plan.version) || plan.version < 1) {
    errors.push("Audio mixer plan version must be a positive integer.");
  }
  if (!plan.scene_id.trim()) {
    errors.push("Audio mixer plan scene id is required.");
  }
  if (!plan.scene_name.trim()) {
    errors.push("Audio mixer plan scene name is required.");
  }
  validateNullablePositiveNumber(plan.sample_rate, "audio.sample_rate", errors);
  validateNullablePositiveNumber(plan.channels, "audio.channels", errors);
  if (plan.sources.length === 0) {
    warnings.push("Audio mixer has no audio scene sources.");
  }
  if (!plan.buses.some((bus) => bus.kind === "master")) {
    errors.push("Audio mixer requires a master bus.");
  }

  plan.sources.forEach((source) => {
    if (!source.scene_source_id.trim()) {
      errors.push("Audio mix source scene source id is required.");
    }
    if (!source.name.trim()) {
      errors.push(`Audio mix source "${source.scene_source_id}" name is required.`);
    }
    validateGain(source.gain_db, source.name, errors);
    if (source.status === "placeholder") {
      warnings.push(`${source.name} is waiting for an audio input: ${source.status_detail}`);
    } else if (source.status === "permission_required") {
      warnings.push(`${source.name} requires audio permission: ${source.status_detail}`);
    } else if (source.status === "unavailable") {
      warnings.push(`${source.name} audio is unavailable: ${source.status_detail}`);
    }
  });

  plan.buses.forEach((bus) => {
    if (!bus.id.trim()) {
      errors.push("Audio mix bus id is required.");
    }
    if (!bus.name.trim()) {
      errors.push(`Audio mix bus "${bus.id}" name is required.`);
    }
    validateNullablePositiveNumber(bus.sample_rate, `${bus.id}.sample_rate`, errors);
    validateNullablePositiveNumber(bus.channels, `${bus.id}.channels`, errors);
    validateGain(bus.gain_db, bus.name, errors);
  });

  return {
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

function audioMixSource(source: Extract<SceneSource, { kind: "audio_meter" }>): AudioMixSource {
  const captureSourceId = source.config.device_id;
  const { status, detail } = captureBindingStatus(source, captureSourceId);
  return {
    scene_source_id: source.id,
    name: source.name,
    capture_source_id: captureSourceId,
    capture_kind: source.config.channel === "system" ? "system_audio" : "microphone",
    gain_db: source.config.gain_db ?? 0,
    muted: source.config.muted ?? false,
    monitor_enabled: source.config.monitor_enabled ?? false,
    meter_enabled: source.config.meter_enabled ?? true,
    sync_offset_ms: Math.round(source.config.sync_offset_ms ?? 0),
    status,
    status_detail: detail,
  };
}

function defaultAudioBuses(): AudioMixBus[] {
  return [
    audioBus("bus-master", "Master", "master"),
    audioBus("bus-monitor", "Monitor", "monitor"),
    audioBus("bus-recording", "Recording", "recording"),
    audioBus("bus-stream", "Stream", "stream"),
  ];
}

function audioBus(id: string, name: string, kind: AudioMixBusKind): AudioMixBus {
  return {
    id,
    name,
    kind,
    sample_rate: 48_000,
    channels: 2,
    gain_db: 0,
    muted: false,
  };
}

function validateGain(value: number, label: string, errors: string[]) {
  if (!Number.isFinite(value) || value < -60 || value > 24) {
    errors.push(`${label} gain must be between -60 dB and 24 dB.`);
  }
}

export function buildCompositorGraph(scene: Scene): CompositorGraph {
  const parentBySourceId = buildGroupParentMap(scene.sources);
  const nodes = [...scene.sources]
    .sort((left, right) => left.z_index - right.z_index || left.id.localeCompare(right.id))
    .map((source) => {
      const { status, detail } = compositorNodeStatus(source);
      const parentSourceId = parentBySourceId.get(source.id) ?? null;
      return {
        id: `node-${source.id}`,
        source_id: source.id,
        name: source.name,
        source_kind: source.kind,
        role: compositorNodeRole(source.kind),
        parent_source_id: parentSourceId,
        group_depth: groupDepth(source.id, parentBySourceId),
        transform: {
          position: { ...source.position },
          size: { ...source.size },
          crop: { ...source.crop },
          rotation_degrees: source.rotation_degrees,
          opacity: source.opacity,
        },
        visible: source.visible,
        locked: source.locked,
        z_index: source.z_index,
        blend_mode: "normal",
        scale_mode: "stretch",
        status,
        status_detail: detail,
        filters: cloneJson(source.filters ?? []),
        config: source.config,
      } satisfies CompositorNode;
    });

  return {
    version: 1,
    scene_id: scene.id,
    scene_name: scene.name,
    output: {
      width: scene.canvas.width,
      height: scene.canvas.height,
      background_color: scene.canvas.background_color,
    },
    nodes,
  };
}

function buildGroupParentMap(sources: SceneSource[]): Map<string, string> {
  const parentBySourceId = new Map<string, string>();
  sources
    .filter((source) => source.kind === "group")
    .forEach((source) => {
      source.config.child_source_ids
        .map((childSourceId) => childSourceId.trim())
        .filter(Boolean)
        .forEach((childSourceId) => {
          if (!parentBySourceId.has(childSourceId)) {
            parentBySourceId.set(childSourceId, source.id);
          }
        });
    });
  return parentBySourceId;
}

function groupDepth(sourceId: string, parentBySourceId: Map<string, string>): number {
  let depth = 0;
  let cursor = sourceId;
  const visited = new Set<string>();

  while (parentBySourceId.has(cursor)) {
    const parentSourceId = parentBySourceId.get(cursor);
    if (!parentSourceId || visited.has(parentSourceId)) break;
    visited.add(parentSourceId);
    depth += 1;
    cursor = parentSourceId;
  }

  return depth;
}

function compositorNodeRole(kind: SceneSourceKind): CompositorNodeRole {
  switch (kind) {
    case "display":
    case "window":
    case "camera":
      return "video";
    case "audio_meter":
      return "audio";
    case "image_media":
    case "browser_overlay":
      return "overlay";
    case "text":
      return "text";
    case "group":
      return "group";
  }
}

function compositorNodeStatus(source: SceneSource): {
  status: CompositorNodeStatus;
  detail: string;
} {
  if (!source.visible) {
    return { status: "hidden", detail: "Source is hidden in the active scene." };
  }

  const availability =
    "availability" in source.config ? source.config.availability : null;
  if (availability) {
    if (availability.state === "available") {
      return { status: "ready", detail: availability.detail };
    }
    if (availability.state === "permission_required") {
      return { status: "permission_required", detail: availability.detail };
    }
    if (availability.state === "unavailable") {
      return { status: "unavailable", detail: availability.detail };
    }
    return { status: "placeholder", detail: availability.detail };
  }

  switch (source.kind) {
    case "display":
      return source.config.display_id
        ? { status: "ready", detail: "Display capture target configured." }
        : { status: "placeholder", detail: "No display capture target has been assigned." };
    case "window":
      return source.config.window_id
        ? { status: "ready", detail: "Window capture target configured." }
        : { status: "placeholder", detail: "No window capture target has been assigned." };
    case "camera":
      return source.config.device_id
        ? { status: "ready", detail: "Camera capture target configured." }
        : { status: "placeholder", detail: "No camera capture target has been assigned." };
    case "audio_meter":
      return source.config.device_id
        ? { status: "ready", detail: "Audio device configured." }
        : { status: "placeholder", detail: "No audio device has been assigned." };
    case "image_media":
      return source.config.asset_uri
        ? { status: "ready", detail: "Media asset configured." }
        : { status: "placeholder", detail: "No media asset has been selected." };
    case "browser_overlay":
      return source.config.url
        ? { status: "ready", detail: "Browser overlay URL configured." }
        : { status: "placeholder", detail: "No browser overlay URL has been configured." };
    case "text":
      return source.config.text.trim()
        ? { status: "ready", detail: "Text content configured." }
        : { status: "placeholder", detail: "Text source is empty." };
    case "group":
      return source.config.child_source_ids.length
        ? {
            status: "ready",
            detail: `${source.config.child_source_ids.length} child source(s) grouped.`,
          }
        : { status: "placeholder", detail: "Group has no child sources." };
  }
}

export function validateCompositorGraph(graph: CompositorGraph): CompositorValidation {
  const warnings: string[] = [];
  const errors: string[] = [];
  const nodeIds = new Set<string>();
  const visibleNodes = graph.nodes.filter((node) => node.visible);

  if (!Number.isInteger(graph.version) || graph.version < 1) {
    errors.push("Compositor graph version must be a positive integer.");
  }
  if (!graph.scene_id.trim()) {
    errors.push("Compositor graph scene id is required.");
  }
  if (!graph.scene_name.trim()) {
    errors.push("Compositor graph scene name is required.");
  }
  validateGraphPositiveNumber(graph.output.width, "output.width", errors);
  validateGraphPositiveNumber(graph.output.height, "output.height", errors);
  if (graph.nodes.length === 0) {
    errors.push("Compositor graph must contain at least one node.");
  }
  if (visibleNodes.length === 0) {
    errors.push("Compositor graph must contain at least one visible node.");
  }

  graph.nodes.forEach((node) => {
    if (nodeIds.has(node.id)) {
      errors.push(`Duplicate compositor node id "${node.id}".`);
    }
    nodeIds.add(node.id);
    if (!node.source_id.trim()) {
      errors.push(`Compositor node "${node.id}" has no source id.`);
    }
    if (!node.name.trim()) {
      errors.push(`Compositor node "${node.id}" has no name.`);
    }
    if (node.parent_source_id) {
      if (node.parent_source_id === node.source_id) {
        errors.push(`Compositor node "${node.id}" cannot parent itself.`);
      }
      if (!graph.nodes.some((candidate) => candidate.source_id === node.parent_source_id)) {
        errors.push(
          `Compositor node "${node.id}" references missing parent source "${node.parent_source_id}".`,
        );
      }
    }
    validateGraphFiniteNumber(node.transform.position.x, `${node.id}.position.x`, errors);
    validateGraphFiniteNumber(node.transform.position.y, `${node.id}.position.y`, errors);
    validateGraphPositiveNumber(node.transform.size.width, `${node.id}.size.width`, errors);
    validateGraphPositiveNumber(node.transform.size.height, `${node.id}.size.height`, errors);
    validateGraphNonNegativeNumber(node.transform.crop.top, `${node.id}.crop.top`, errors);
    validateGraphNonNegativeNumber(node.transform.crop.right, `${node.id}.crop.right`, errors);
    validateGraphNonNegativeNumber(node.transform.crop.bottom, `${node.id}.crop.bottom`, errors);
    validateGraphNonNegativeNumber(node.transform.crop.left, `${node.id}.crop.left`, errors);
    validateGraphFiniteNumber(node.transform.rotation_degrees, `${node.id}.rotation`, errors);
    if (
      !Number.isFinite(node.transform.opacity) ||
      node.transform.opacity < 0 ||
      node.transform.opacity > 1
    ) {
      errors.push(`${node.id}.opacity must be between 0 and 1.`);
    }

    if (node.status === "placeholder") {
      warnings.push(`${node.name} is using a placeholder: ${node.status_detail}`);
    } else if (node.status === "permission_required") {
      warnings.push(`${node.name} requires permission: ${node.status_detail}`);
    } else if (node.status === "unavailable") {
      warnings.push(`${node.name} is unavailable: ${node.status_detail}`);
    }
  });

  return {
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

export function buildCompositorRenderPlan(
  graph: CompositorGraph,
  targets: CompositorRenderTarget[],
): CompositorRenderPlan {
  return {
    version: 1,
    renderer: "contract",
    graph,
    targets,
  };
}

export function buildDefaultCompositorRenderTargets(
  intent: PipelineIntent,
  graph: CompositorGraph,
  recordingProfile: MediaProfile | null | undefined,
  streamDestinations: StreamDestination[] = [],
): CompositorRenderTarget[] {
  const framerate = recordingProfile?.framerate ?? 60;
  const targets: CompositorRenderTarget[] = [
    compositorRenderTarget(
      "target-preview",
      "Preview",
      "preview",
      graph.output.width,
      graph.output.height,
      framerate,
    ),
    compositorRenderTarget(
      "target-program",
      "Program",
      "program",
      graph.output.width,
      graph.output.height,
      framerate,
    ),
  ];

  if (intent === "recording" || intent === "recording_and_stream") {
    targets.push(
      compositorRenderTarget(
        "target-recording",
        "Recording Output",
        "recording",
        recordingProfile?.resolution.width ?? graph.output.width,
        recordingProfile?.resolution.height ?? graph.output.height,
        framerate,
      ),
    );
  }

  if (intent === "stream" || intent === "recording_and_stream") {
    if (streamDestinations.length === 0) {
      targets.push(
        compositorRenderTarget(
          "target-stream",
          "Stream Output",
          "stream",
          graph.output.width,
          graph.output.height,
          framerate,
        ),
      );
    } else {
      targets.push(
        ...streamDestinations.map((destination) =>
          compositorRenderTarget(
            `target-stream-${destination.id}`,
            `Stream Output: ${destination.name}`,
            "stream",
            graph.output.width,
            graph.output.height,
            framerate,
          ),
        ),
      );
    }
  }

  return targets;
}

export function validateCompositorRenderPlan(
  plan: CompositorRenderPlan,
): CompositorValidation {
  const validation = validateCompositorGraph(plan.graph);
  const targetIds = new Set<string>();
  const enabledTargets = plan.targets.filter((target) => target.enabled);

  if (!Number.isInteger(plan.version) || plan.version < 1) {
    validation.errors.push("Compositor render plan version must be a positive integer.");
  }
  if (plan.targets.length === 0) {
    validation.errors.push("Compositor render plan must contain at least one target.");
  }
  if (enabledTargets.length === 0) {
    validation.errors.push("Compositor render plan must contain at least one enabled target.");
  }
  if (!enabledTargets.some((target) => target.kind === "program")) {
    validation.warnings.push("Compositor render plan has no enabled program target.");
  }

  plan.targets.forEach((target) => {
    if (targetIds.has(target.id)) {
      validation.errors.push(`Duplicate compositor render target id "${target.id}".`);
    }
    targetIds.add(target.id);
    if (!target.id.trim()) {
      validation.errors.push("Compositor render target id is required.");
    }
    if (!target.name.trim()) {
      validation.errors.push(`Compositor render target "${target.id}" name is required.`);
    }
    validateGraphPositiveNumber(target.width, `${target.id}.width`, validation.errors);
    validateGraphPositiveNumber(target.height, `${target.id}.height`, validation.errors);
    validateGraphPositiveNumber(target.framerate, `${target.id}.framerate`, validation.errors);
  });

  return {
    ...validation,
    ready: validation.errors.length === 0,
  };
}

export function buildPerformanceTelemetryPlan(
  renderPlan: CompositorRenderPlan,
): PerformanceTelemetryPlan {
  const plan: PerformanceTelemetryPlan = {
    version: 1,
    scene_id: renderPlan.graph.scene_id,
    scene_name: renderPlan.graph.scene_name,
    sample_window_seconds: 10,
    cpu_warning_percent: 85,
    gpu_warning_percent: 85,
    targets: renderPlan.targets
      .filter((target) => target.enabled)
      .map(performanceTargetBudget),
    validation: {
      ready: true,
      warnings: [],
      errors: [],
    },
  };
  plan.validation = validatePerformanceTelemetryPlan(plan);
  return plan;
}

export function validatePerformanceTelemetryPlan(
  plan: PerformanceTelemetryPlan,
): PerformanceTelemetryValidation {
  const warnings: string[] = [];
  const errors: string[] = [];
  const targetIds = new Set<string>();

  if (!Number.isInteger(plan.version) || plan.version < 1) {
    errors.push("Performance telemetry plan version must be a positive integer.");
  }
  if (!plan.scene_id.trim()) {
    errors.push("Performance telemetry scene id is required.");
  }
  if (!plan.scene_name.trim()) {
    errors.push("Performance telemetry scene name is required.");
  }
  if (!Number.isInteger(plan.sample_window_seconds) || plan.sample_window_seconds < 1) {
    errors.push("Performance telemetry sample window must be greater than zero.");
  }
  validatePercent(plan.cpu_warning_percent, "CPU warning percent", errors);
  validatePercent(plan.gpu_warning_percent, "GPU warning percent", errors);
  if (plan.targets.length === 0) {
    warnings.push("Performance telemetry has no enabled render targets.");
  }

  for (const target of plan.targets) {
    if (targetIds.has(target.target_id)) {
      errors.push(`Duplicate performance target id "${target.target_id}".`);
    }
    targetIds.add(target.target_id);
    if (!target.target_id.trim()) {
      errors.push("Performance target id is required.");
    }
    if (!target.target_name.trim()) {
      errors.push(`Performance target "${target.target_id}" name is required.`);
    }
    validateGraphPositiveNumber(target.width, `${target.target_id}.width`, errors);
    validateGraphPositiveNumber(target.height, `${target.target_id}.height`, errors);
    validateGraphPositiveNumber(target.framerate, `${target.target_id}.framerate`, errors);
    validateGraphPositiveNumber(
      target.frame_budget_nanos,
      `${target.target_id}.frame_budget_nanos`,
      errors,
    );
    validateGraphPositiveNumber(
      target.render_budget_nanos,
      `${target.target_id}.render_budget_nanos`,
      errors,
    );
    if (target.framerate > 120) {
      warnings.push(
        `${target.target_name} targets ${target.framerate} fps; validate frame pacing on target hardware.`,
      );
    }
    if (target.estimated_rgba_bytes_per_frame > 33_177_600) {
      warnings.push(
        `${target.target_name} exceeds a 4K RGBA frame budget; validate GPU and encoder load.`,
      );
    }
  }

  const totalRgbaBytesPerSecond = plan.targets.reduce(
    (total, target) => total + target.estimated_rgba_bytes_per_second,
    0,
  );
  if (totalRgbaBytesPerSecond > 2_000_000_000) {
    warnings.push(
      `Estimated RGBA throughput is ${Math.floor(
        totalRgbaBytesPerSecond / 1_000_000,
      )} MB/s across enabled targets.`,
    );
  }

  return {
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

export function evaluateCompositorFrame(
  plan: CompositorRenderPlan,
  frameIndex: number,
): CompositorRenderedFrame {
  const validation = validateCompositorRenderPlan(plan);
  const framerate = plan.targets.find((target) => target.enabled)?.framerate ?? 60;
  const durationNanos = Math.floor(1_000_000_000 / Math.max(1, framerate));
  const targets = plan.targets
    .filter((target) => target.enabled)
    .map((target) => ({
      target_id: target.id,
      target_kind: target.kind,
      width: target.width,
      height: target.height,
      frame_format: target.frame_format,
      nodes: plan.graph.nodes
        .filter((node) => node.visible)
        .map((node) => evaluateNodeForTarget(node, plan.graph, target)),
    }));

  return {
    renderer: plan.renderer,
    scene_id: plan.graph.scene_id,
    scene_name: plan.graph.scene_name,
    clock: {
      frame_index: frameIndex,
      framerate,
      pts_nanos: frameIndex * durationNanos,
      duration_nanos: durationNanos,
    },
    targets,
    validation,
  };
}

function performanceTargetBudget(
  target: CompositorRenderTarget,
): PerformanceTargetBudget {
  const frameBudgetNanos = Math.floor(1_000_000_000 / Math.max(1, target.framerate));
  const pixelCount = target.width * target.height;
  const estimatedRgbaBytesPerFrame = pixelCount * 4;

  return {
    target_id: target.id,
    target_name: target.name,
    target_kind: target.kind,
    width: target.width,
    height: target.height,
    framerate: target.framerate,
    frame_budget_nanos: frameBudgetNanos,
    render_budget_nanos: Math.floor((frameBudgetNanos * 70) / 100),
    encode_budget_nanos: Math.floor((frameBudgetNanos * 20) / 100),
    max_latency_ms: Math.ceil((frameBudgetNanos * 2) / 1_000_000),
    max_dropped_frames_per_minute: Math.max(
      1,
      Math.floor((target.framerate * 60) / 200),
    ),
    pixel_count: pixelCount,
    estimated_rgba_bytes_per_frame: estimatedRgbaBytesPerFrame,
    estimated_rgba_bytes_per_second: estimatedRgbaBytesPerFrame * target.framerate,
  };
}

function validatePercent(value: number, label: string, errors: string[]) {
  if (!Number.isFinite(value) || value <= 0 || value > 100) {
    errors.push(`Performance telemetry ${label} must be 1-100.`);
  }
}

function evaluateNodeForTarget(
  node: CompositorNode,
  graph: CompositorGraph,
  target: CompositorRenderTarget,
): CompositorEvaluatedNode {
  const transform = effectiveNodeTransform(node, graph);
  const { scaleX, scaleY, offsetX, offsetY } = targetMapping(graph.output, target);
  return {
    node_id: node.id,
    source_id: node.source_id,
    name: node.name,
    role: node.role,
    status: node.status,
    rect: {
      x: offsetX + transform.position.x * scaleX,
      y: offsetY + transform.position.y * scaleY,
      width: transform.size.width * scaleX,
      height: transform.size.height * scaleY,
    },
    crop: {
      top: transform.crop.top * scaleY,
      right: transform.crop.right * scaleX,
      bottom: transform.crop.bottom * scaleY,
      left: transform.crop.left * scaleX,
    },
    rotation_degrees: transform.rotation_degrees,
    opacity: transform.opacity,
    z_index: node.z_index,
  };
}

function effectiveNodeTransform(
  node: CompositorNode,
  graph: CompositorGraph,
): CompositorTransform {
  const transform: CompositorTransform = {
    position: { ...node.transform.position },
    size: { ...node.transform.size },
    crop: { ...node.transform.crop },
    rotation_degrees: node.transform.rotation_degrees,
    opacity: node.transform.opacity,
  };
  let parentSourceId = node.parent_source_id ?? null;
  const visited = new Set<string>();

  while (parentSourceId) {
    if (visited.has(parentSourceId)) break;
    visited.add(parentSourceId);
    const parent = graph.nodes.find((candidate) => candidate.source_id === parentSourceId);
    if (!parent) break;
    transform.position.x += parent.transform.position.x;
    transform.position.y += parent.transform.position.y;
    transform.rotation_degrees += parent.transform.rotation_degrees;
    transform.opacity = Math.min(1, Math.max(0, transform.opacity * parent.transform.opacity));
    parentSourceId = parent.parent_source_id ?? null;
  }

  return transform;
}

function targetMapping(
  output: CompositorOutput,
  target: CompositorRenderTarget,
): { scaleX: number; scaleY: number; offsetX: number; offsetY: number } {
  const sourceWidth = Math.max(1, output.width);
  const sourceHeight = Math.max(1, output.height);
  const targetWidth = Math.max(1, target.width);
  const targetHeight = Math.max(1, target.height);

  if (target.scale_mode === "stretch") {
    return {
      scaleX: targetWidth / sourceWidth,
      scaleY: targetHeight / sourceHeight,
      offsetX: 0,
      offsetY: 0,
    };
  }
  if (target.scale_mode === "fill") {
    const scale = Math.max(targetWidth / sourceWidth, targetHeight / sourceHeight);
    return {
      scaleX: scale,
      scaleY: scale,
      offsetX: (targetWidth - sourceWidth * scale) / 2,
      offsetY: (targetHeight - sourceHeight * scale) / 2,
    };
  }
  if (target.scale_mode === "original_size") {
    return {
      scaleX: 1,
      scaleY: 1,
      offsetX: (targetWidth - sourceWidth) / 2,
      offsetY: (targetHeight - sourceHeight) / 2,
    };
  }

  const scale = Math.min(targetWidth / sourceWidth, targetHeight / sourceHeight);
  return {
    scaleX: scale,
    scaleY: scale,
    offsetX: (targetWidth - sourceWidth * scale) / 2,
    offsetY: (targetHeight - sourceHeight * scale) / 2,
  };
}

function compositorRenderTarget(
  id: string,
  name: string,
  kind: CompositorRenderTargetKind,
  width: number,
  height: number,
  framerate: number,
): CompositorRenderTarget {
  return {
    id,
    name,
    kind,
    width,
    height,
    framerate,
    frame_format: "bgra8",
    scale_mode: "fit",
    enabled: true,
  };
}

export function validateSceneCollection(
  collection: SceneCollection,
): SceneValidationResult {
  const issues: SceneValidationIssue[] = [];
  const sceneIds = new Set<string>();

  if (!collection.id.trim()) {
    issues.push({ path: "id", message: "Scene collection id is required." });
  }
  if (!collection.name.trim()) {
    issues.push({ path: "name", message: "Scene collection name is required." });
  }
  if (!Number.isInteger(collection.version) || collection.version < 1) {
    issues.push({ path: "version", message: "Scene collection version must be a positive integer." });
  }
  if (collection.scenes.length === 0) {
    issues.push({ path: "scenes", message: "At least one scene is required." });
  }

  collection.scenes.forEach((scene, sceneIndex) => {
    const scenePath = `scenes[${sceneIndex}]`;
    if (sceneIds.has(scene.id)) {
      issues.push({ path: `${scenePath}.id`, message: `Duplicate scene id "${scene.id}".` });
    }
    sceneIds.add(scene.id);
    if (!scene.name.trim()) {
      issues.push({ path: `${scenePath}.name`, message: "Scene name is required." });
    }
    validatePositiveNumber(scene.canvas.width, `${scenePath}.canvas.width`, issues);
    validatePositiveNumber(scene.canvas.height, `${scenePath}.canvas.height`, issues);
    validateSceneSources(scene.sources, scenePath, issues);
  });
  validateSceneTransitions(collection, issues);

  if (collection.scenes.length > 0 && !sceneIds.has(collection.active_scene_id)) {
    issues.push({
      path: "active_scene_id",
      message: "Active scene id must match a scene in the collection.",
    });
  }

  return {
    ok: issues.length === 0,
    issues,
  };
}

function validateSceneTransitions(
  collection: SceneCollection,
  issues: SceneValidationIssue[],
) {
  const transitionIds = new Set<string>();
  if (collection.transitions.length === 0) {
    issues.push({ path: "transitions", message: "At least one scene transition is required." });
  }

  collection.transitions.forEach((transition, transitionIndex) => {
    const transitionPath = `transitions[${transitionIndex}]`;
    if (transitionIds.has(transition.id)) {
      issues.push({
        path: `${transitionPath}.id`,
        message: `Duplicate transition id "${transition.id}".`,
      });
    }
    transitionIds.add(transition.id);
    if (!transition.id.trim()) {
      issues.push({ path: `${transitionPath}.id`, message: "Transition id is required." });
    }
    if (!transition.name.trim()) {
      issues.push({ path: `${transitionPath}.name`, message: "Transition name is required." });
    }
    if (!Number.isInteger(transition.duration_ms) || transition.duration_ms < 0) {
      issues.push({
        path: `${transitionPath}.duration_ms`,
        message: "Transition duration must be 0 or greater.",
      });
    } else if (transition.duration_ms > 60_000) {
      issues.push({
        path: `${transitionPath}.duration_ms`,
        message: "Transition duration must be 60 seconds or less.",
      });
    }
    if (transition.kind === "cut" && transition.duration_ms !== 0) {
      issues.push({
        path: `${transitionPath}.duration_ms`,
        message: "Cut transitions must use a zero millisecond duration.",
      });
    }
  });

  if (
    collection.transitions.length > 0 &&
    !transitionIds.has(collection.active_transition_id)
  ) {
    issues.push({
      path: "active_transition_id",
      message: "Active transition id must match a transition in the collection.",
    });
  }
}

function validateSceneSources(
  sources: SceneSource[],
  scenePath: string,
  issues: SceneValidationIssue[],
) {
  const sourceIds = new Set<string>();
  if (!sources.some((source) => source.visible)) {
    issues.push({
      path: `${scenePath}.sources`,
      message: "Scene must contain at least one visible source.",
    });
  }

  sources.forEach((source, sourceIndex) => {
    const sourcePath = `${scenePath}.sources[${sourceIndex}]`;
    if (sourceIds.has(source.id)) {
      issues.push({
        path: `${sourcePath}.id`,
        message: `Duplicate source id "${source.id}".`,
      });
    }
    sourceIds.add(source.id);
    if (!source.name.trim()) {
      issues.push({ path: `${sourcePath}.name`, message: "Source name is required." });
    }
    validateFiniteNumber(source.position.x, `${sourcePath}.position.x`, issues);
    validateFiniteNumber(source.position.y, `${sourcePath}.position.y`, issues);
    validatePositiveNumber(source.size.width, `${sourcePath}.size.width`, issues);
    validatePositiveNumber(source.size.height, `${sourcePath}.size.height`, issues);
    validateNonNegativeNumber(source.crop.top, `${sourcePath}.crop.top`, issues);
    validateNonNegativeNumber(source.crop.right, `${sourcePath}.crop.right`, issues);
    validateNonNegativeNumber(source.crop.bottom, `${sourcePath}.crop.bottom`, issues);
    validateNonNegativeNumber(source.crop.left, `${sourcePath}.crop.left`, issues);
    validateFiniteNumber(source.rotation_degrees, `${sourcePath}.rotation_degrees`, issues);
    validateFiniteNumber(source.z_index, `${sourcePath}.z_index`, issues);
    if (!Number.isFinite(source.opacity) || source.opacity < 0 || source.opacity > 1) {
      issues.push({
        path: `${sourcePath}.opacity`,
        message: "Source opacity must be between 0 and 1.",
      });
    }
    validateSceneSourceFilters(source.filters ?? [], sourcePath, issues);
  });

  validateGroupSourceChildren(sources, scenePath, sourceIds, issues);
}

function validateGroupSourceChildren(
  sources: SceneSource[],
  scenePath: string,
  sourceIds: Set<string>,
  issues: SceneValidationIssue[],
) {
  const groupIds = new Set(
    sources.filter((source) => source.kind === "group").map((source) => source.id),
  );
  const parentByChild = new Map<string, string>();
  const childrenByGroup = new Map<string, string[]>();

  sources.forEach((source, sourceIndex) => {
    if (source.kind !== "group") return;

    const childIds = new Set<string>();
    source.config.child_source_ids.forEach((rawChildId, childIndex) => {
      const childPath = `${scenePath}.sources[${sourceIndex}].config.child_source_ids[${childIndex}]`;
      const childId = rawChildId.trim();
      if (!childId) {
        issues.push({ path: childPath, message: "Group child source id is required." });
        return;
      }
      if (childIds.has(childId)) {
        issues.push({
          path: childPath,
          message: `Duplicate group child source id "${childId}".`,
        });
      }
      childIds.add(childId);
      if (childId === source.id) {
        issues.push({ path: childPath, message: "Group cannot contain itself." });
      }
      if (!sourceIds.has(childId)) {
        issues.push({
          path: childPath,
          message: `Group child source id "${childId}" does not exist.`,
        });
      }
      const existingParent = parentByChild.get(childId);
      if (existingParent) {
        issues.push({
          path: childPath,
          message: `Source "${childId}" is already grouped by "${existingParent}".`,
        });
      }
      parentByChild.set(childId, source.id);
      if (groupIds.has(childId)) {
        const nestedChildren = childrenByGroup.get(source.id) ?? [];
        nestedChildren.push(childId);
        childrenByGroup.set(source.id, nestedChildren);
      }
    });
  });

  const visited = new Set<string>();
  groupIds.forEach((groupId) => {
    const visiting = new Set<string>();
    if (groupHasCycle(groupId, childrenByGroup, visiting, visited)) {
      issues.push({
        path: `${scenePath}.sources`,
        message: `Group source "${groupId}" creates a cycle.`,
      });
    }
  });
}

function groupHasCycle(
  groupId: string,
  childrenByGroup: Map<string, string[]>,
  visiting: Set<string>,
  visited: Set<string>,
): boolean {
  if (visited.has(groupId)) return false;
  if (visiting.has(groupId)) return true;
  visiting.add(groupId);

  for (const childId of childrenByGroup.get(groupId) ?? []) {
    if (groupHasCycle(childId, childrenByGroup, visiting, visited)) {
      return true;
    }
  }

  visiting.delete(groupId);
  visited.add(groupId);
  return false;
}

function validateSceneSourceFilters(
  filters: SceneSourceFilter[],
  sourcePath: string,
  issues: SceneValidationIssue[],
) {
  const filterIds = new Set<string>();
  filters.forEach((filter, filterIndex) => {
    const filterPath = `${sourcePath}.filters[${filterIndex}]`;
    if (filterIds.has(filter.id)) {
      issues.push({
        path: `${filterPath}.id`,
        message: `Duplicate source filter id "${filter.id}".`,
      });
    }
    filterIds.add(filter.id);
    if (!filter.id.trim()) {
      issues.push({ path: `${filterPath}.id`, message: "Source filter id is required." });
    }
    if (!filter.name.trim()) {
      issues.push({ path: `${filterPath}.name`, message: "Source filter name is required." });
    }
    validateFiniteNumber(filter.order, `${filterPath}.order`, issues);
  });
}

function validateGraphFiniteNumber(value: number, path: string, errors: string[]) {
  if (!Number.isFinite(value)) {
    errors.push(`${path} must be a finite number.`);
  }
}

function validateGraphPositiveNumber(value: number, path: string, errors: string[]) {
  if (!Number.isFinite(value) || value <= 0) {
    errors.push(`${path} must be greater than 0.`);
  }
}

function validateGraphNonNegativeNumber(value: number, path: string, errors: string[]) {
  if (!Number.isFinite(value) || value < 0) {
    errors.push(`${path} must be 0 or greater.`);
  }
}

function validateNullablePositiveNumber(
  value: number | null,
  path: string,
  errors: string[],
) {
  if (value !== null && (!Number.isFinite(value) || value <= 0)) {
    errors.push(`${path} must be greater than 0.`);
  }
}

function validateFiniteNumber(
  value: number,
  path: string,
  issues: SceneValidationIssue[],
) {
  if (!Number.isFinite(value)) {
    issues.push({ path, message: "Value must be a finite number." });
  }
}

function validatePositiveNumber(
  value: number,
  path: string,
  issues: SceneValidationIssue[],
) {
  if (!Number.isFinite(value) || value <= 0) {
    issues.push({ path, message: "Value must be greater than 0." });
  }
}

function validateNonNegativeNumber(
  value: number,
  path: string,
  issues: SceneValidationIssue[],
) {
  if (!Number.isFinite(value) || value < 0) {
    issues.push({ path, message: "Value must be 0 or greater." });
  }
}

export function cloneSceneCollection(collection: SceneCollection): SceneCollection {
  return cloneJson(collection);
}
