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

export type SceneSourceBoundsMode =
  | "stretch"
  | "fit"
  | "fill"
  | "center"
  | "original_size";

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
  bounds_mode: SceneSourceBoundsMode;
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

export type SceneTransitionPreviewLayerRole = "from" | "to" | "stinger";

export interface SceneTransitionPreviewLayer {
  role: SceneTransitionPreviewLayerRole;
  scene_id: string | null;
  scene_name: string;
  label: string;
  visible: boolean;
  opacity: number;
  offset_x: number;
  offset_y: number;
}

export interface SceneTransitionPreviewFrame {
  version: number;
  transition_id: string;
  transition_kind: SceneTransitionKind;
  frame_index: number;
  elapsed_ms: number;
  linear_progress: number;
  eased_progress: number;
  width: number;
  height: number;
  checksum: string;
  layers: SceneTransitionPreviewLayer[];
  validation: SceneTransitionPreviewValidation;
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

export type CompositorScaleMode = "stretch" | "fit" | "fill" | "center" | "original_size";

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

export interface RenderTargetProfile {
  id: string;
  name: string;
  kind: CompositorRenderTargetKind;
  width: number;
  height: number;
  framerate: number;
  frame_format: CompositorFrameFormat;
  scale_mode: CompositorScaleMode;
  enabled: boolean;
  encoder_preference: EncoderPreference;
  bitrate_kbps: number | null;
}

export interface RecordingTargetContract {
  id: string;
  profile_id: string;
  profile_name: string;
  render_target_id: string;
  output_folder: string;
  filename_pattern: string;
  container: RecordingContainer;
  resolution: Resolution;
  framerate: number;
  bitrate_kbps: number;
  encoder_preference: EncoderPreference;
  output_path_preview: string;
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export interface StreamingTargetContract {
  id: string;
  destination_id: string;
  destination_name: string;
  platform: PlatformKind;
  render_target_id: string;
  ingest_url: string;
  stream_key_required: boolean;
  has_stream_key: boolean;
  bandwidth_test: boolean;
  width: number;
  height: number;
  framerate: number;
  bitrate_kbps: number;
  encoder_preference: EncoderPreference;
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export interface OutputPreflightValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export interface OutputPreflightPlan {
  version: number;
  intent: PipelineIntent;
  active_scene_id: string | null;
  active_scene_name: string | null;
  render_targets: RenderTargetProfile[];
  recording_target: RecordingTargetContract | null;
  streaming_targets: StreamingTargetContract[];
  validation: OutputPreflightValidation;
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
  status_detail: string;
  asset?: SoftwareCompositorAssetMetadata | null;
  text?: SoftwareCompositorTextMetadata | null;
  filters: SoftwareCompositorFilterMetadata[];
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

export type SoftwareCompositorAssetStatus =
  | "decoded"
  | "missing_file"
  | "unsupported_extension"
  | "decode_failed"
  | "video_placeholder"
  | "no_asset";

export interface SoftwareCompositorAssetMetadata {
  uri: string;
  status: SoftwareCompositorAssetStatus;
  status_detail: string;
  format?: string | null;
  width?: number | null;
  height?: number | null;
  checksum?: number | null;
  modified_unix_ms?: number | null;
  cache_hit: boolean;
}

export type SoftwareCompositorTextStatus =
  | "rendered"
  | "font_fallback"
  | "empty"
  | "invalid_color";

export interface SoftwareCompositorTextMetadata {
  status: SoftwareCompositorTextStatus;
  status_detail: string;
  requested_font_family: string;
  used_font_family: string;
  font_size: number;
  color: string;
  align: string;
  text_length: number;
  rendered_bounds?: CompositorRect | null;
  checksum?: number | null;
}

export type SoftwareCompositorFilterStatus =
  | "applied"
  | "skipped"
  | "deferred"
  | "error";

export interface SoftwareCompositorFilterMetadata {
  id: string;
  name: string;
  kind: SceneSourceFilterKind;
  status: SoftwareCompositorFilterStatus;
  status_detail: string;
  order: number;
  checksum?: number | null;
}

export interface SoftwareCompositorInputFrame {
  source_id: string;
  source_kind: SceneSourceKind;
  width: number;
  height: number;
  frame_format: CompositorFrameFormat;
  status: CompositorNodeStatus;
  status_detail: string;
  asset?: SoftwareCompositorAssetMetadata | null;
  text?: SoftwareCompositorTextMetadata | null;
  filters: SoftwareCompositorFilterMetadata[];
  checksum: number;
  pixels: number[];
}

export interface SoftwareCompositorFrame {
  target_id: string;
  target_kind: CompositorRenderTargetKind;
  width: number;
  height: number;
  frame_format: CompositorFrameFormat;
  bytes_per_row: number;
  checksum: number;
  pixels: number[];
}

export interface SoftwareCompositorRenderResult {
  frame: CompositorRenderedFrame;
  input_frames: SoftwareCompositorInputFrame[];
  pixel_frames: SoftwareCompositorFrame[];
}

export interface SceneRuntimeContractValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export type SceneRuntimeCommandKind =
  | "activate_scene"
  | "update_runtime_state"
  | "request_preview_frame"
  | "validate_runtime_graph"
  | "execute_transition";

export type SceneRuntimeCommandPayload =
  | SceneActivationRequest
  | SceneRuntimeStateUpdateRequest
  | PreviewFrameRequest
  | CompositorRenderRequest
  | TransitionExecutionRequest;

export interface SceneRuntimeCommand {
  version: number;
  command_id: string;
  kind: SceneRuntimeCommandKind;
  requested_at: string;
  payload: SceneRuntimeCommandPayload;
}

export type SceneRuntimeStatus =
  | "idle"
  | "activating"
  | "active"
  | "transitioning"
  | "error";

export interface SceneRuntimeSnapshot {
  version: number;
  collection_id: string;
  collection_name: string;
  active_scene_id: string;
  active_scene_name: string;
  active_transition_id: string;
  active_transition_name: string;
  status: SceneRuntimeStatus;
  preview_enabled: boolean;
  metadata: Record<string, unknown>;
  updated_at: string;
  validation: SceneRuntimeContractValidation;
}

export interface SceneRuntimeStatePatch {
  active_scene_id?: string | null;
  active_transition_id?: string | null;
  status?: SceneRuntimeStatus | null;
  preview_enabled?: boolean | null;
  metadata?: Record<string, unknown> | null;
}

export interface SceneRuntimeStateUpdateRequest {
  version: number;
  request_id: string;
  collection_id: string;
  patch: SceneRuntimeStatePatch;
  requested_at: string;
}

export interface SceneRuntimeStateUpdateResponse {
  version: number;
  request_id: string;
  collection_id: string;
  active_scene_id: string;
  active_transition_id: string;
  status: SceneRuntimeStatus;
  updated_at: string;
  validation: SceneRuntimeContractValidation;
}

export interface SceneActivationRequest {
  version: number;
  request_id: string;
  collection_id: string;
  target_scene_id: string;
  transition_id: string | null;
  requested_at: string;
  reason: string | null;
}

export type SceneActivationStatus = "accepted" | "rejected";

export interface SceneActivationResponse {
  version: number;
  request_id: string;
  collection_id: string;
  previous_scene_id: string | null;
  active_scene_id: string;
  transition_id: string | null;
  status: SceneActivationStatus;
  activated_at: string;
  runtime: SceneRuntimeSnapshot;
  validation: SceneRuntimeContractValidation;
}

export type PreviewFrameEncoding = "none" | "data_url" | "base64";

export interface PreviewFrameRequest {
  version: number;
  request_id: string;
  scene_id: string;
  width: number;
  height: number;
  framerate: number;
  frame_format: CompositorFrameFormat;
  scale_mode: CompositorScaleMode;
  encoding: PreviewFrameEncoding;
  include_debug_overlay: boolean;
  requested_at: string;
}

export interface PreviewFrameResponse {
  version: number;
  request_id: string;
  scene_id: string;
  scene_name: string;
  frame_index: number;
  width: number;
  height: number;
  frame_format: CompositorFrameFormat;
  encoding: PreviewFrameEncoding;
  image_data: string | null;
  checksum: string | null;
  render_time_ms: number;
  generated_at: string;
  rendered_frame: CompositorRenderedFrame | null;
  validation: SceneRuntimeContractValidation;
}

export interface CompositorRenderRequest {
  version: number;
  request_id: string;
  renderer: CompositorRendererKind;
  plan: CompositorRenderPlan;
  clock: CompositorFrameClock;
  requested_at: string;
}

export interface CompositorRenderTargetResult {
  target_id: string;
  target_kind: CompositorRenderTargetKind;
  width: number;
  height: number;
  frame_format: CompositorFrameFormat;
  checksum: string | null;
  byte_length: number | null;
}

export interface CompositorRenderResponse {
  version: number;
  request_id: string;
  renderer: CompositorRendererKind;
  scene_id: string;
  scene_name: string;
  frame: CompositorRenderedFrame;
  target_results: CompositorRenderTargetResult[];
  render_time_ms: number;
  rendered_at: string;
  validation: SceneRuntimeContractValidation;
}

export interface RuntimeCaptureSourceBinding {
  scene_source_id: string;
  scene_source_name: string;
  scene_source_kind: SceneSourceKind;
  capture_source_id: string | null;
  capture_kind: CaptureSourceKind;
  media_kind: CaptureFrameMediaKind;
  frame_format: CaptureFrameFormat;
  width: number | null;
  height: number | null;
  framerate: number | null;
  sample_rate: number | null;
  channels: number | null;
  required: boolean;
  status: CaptureFrameBindingStatus;
  status_detail: string;
}

export interface RuntimeCaptureSourceBindingContract {
  version: number;
  scene_id: string;
  scene_name: string;
  bindings: RuntimeCaptureSourceBinding[];
  validation: SceneRuntimeContractValidation;
}

export interface RuntimeAudioSourceBinding {
  scene_source_id: string;
  scene_source_name: string;
  capture_source_id: string | null;
  capture_kind: CaptureSourceKind;
  bus_ids: string[];
  gain_db: number;
  muted: boolean;
  monitor_enabled: boolean;
  meter_enabled: boolean;
  sync_offset_ms: number;
  status: AudioMixSourceStatus;
  status_detail: string;
}

export interface RuntimeAudioSourceBindingContract {
  version: number;
  scene_id: string;
  scene_name: string;
  sample_rate: number;
  channels: number;
  bindings: RuntimeAudioSourceBinding[];
  buses: AudioMixBus[];
  validation: SceneRuntimeContractValidation;
}

export interface SceneRuntimeBindingsSnapshot {
  version: number;
  scene_id: string;
  scene_name: string;
  capture: RuntimeCaptureSourceBindingContract;
  audio: RuntimeAudioSourceBindingContract;
  generated_at: string;
}

export interface TransitionExecutionRequest {
  version: number;
  request_id: string;
  collection_id: string;
  transition_id: string;
  from_scene_id: string;
  to_scene_id: string;
  framerate: number;
  requested_at: string;
}

export interface TransitionExecutionResponse {
  version: number;
  request_id: string;
  collection_id: string;
  transition_id: string;
  from_scene_id: string;
  to_scene_id: string;
  started_at: string;
  preview_plan: SceneTransitionPreviewPlan;
  validation: SceneRuntimeContractValidation;
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
  bounds_mode?: SceneSourceBoundsMode;
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
  filters: SceneSourceFilter[];
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

export interface AudioGraphRuntimeSource {
  scene_source_id: string;
  name: string;
  capture_source_id: string | null;
  capture_kind: CaptureSourceKind;
  gain_db: number;
  muted: boolean;
  monitor_enabled: boolean;
  meter_enabled: boolean;
  sync_offset_ms: number;
  pre_filter_level_db: number;
  pre_filter_peak_db: number;
  pre_filter_linear_level: number;
  post_filter_level_db: number;
  post_filter_peak_db: number;
  post_filter_linear_level: number;
  level_db: number;
  peak_db: number;
  linear_level: number;
  status: AudioMixSourceStatus;
  status_detail: string;
  filters: AudioFilterRuntimeMetadata[];
}

export type AudioFilterRuntimeStatus = "applied" | "skipped" | "error";

export interface AudioFilterRuntimeMetadata {
  id: string;
  name: string;
  kind: SceneSourceFilterKind;
  enabled: boolean;
  order: number;
  status: AudioFilterRuntimeStatus;
  status_detail: string;
  input_level_db: number;
  output_level_db: number;
  input_peak_db: number;
  output_peak_db: number;
  level_change_db: number;
  gain_reduction_db?: number | null;
  attenuation_db?: number | null;
  control_summary?: string | null;
}

export interface AudioGraphRuntimeBus {
  id: string;
  name: string;
  kind: AudioMixBusKind;
  gain_db: number;
  muted: boolean;
  level_db: number;
  peak_db: number;
  linear_level: number;
}

export interface AudioGraphRuntimeValidation {
  ready: boolean;
  warnings: string[];
  errors: string[];
}

export interface AudioGraphRuntimeSnapshot {
  version: number;
  scene_id: string;
  scene_name: string;
  sample_rate: number;
  channels: number;
  sources: AudioGraphRuntimeSource[];
  buses: AudioGraphRuntimeBus[];
  generated_at: string;
  validation: AudioGraphRuntimeValidation;
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
  output_preflight_plan?: OutputPreflightPlan | null;
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
    bounds_mode: defaults.bounds_mode ?? "stretch",
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

export function buildSceneTransitionPreviewFrame(
  plan: SceneTransitionPreviewPlan,
  frameIndex: number,
  width = 640,
  height = 360,
): SceneTransitionPreviewFrame {
  const safeFrameCount = Math.max(1, plan.frame_count);
  const clampedFrameIndex = Math.min(
    safeFrameCount - 1,
    Math.max(0, Math.round(frameIndex)),
  );
  const linearProgress =
    safeFrameCount <= 1 ? 1 : clampedFrameIndex / (safeFrameCount - 1);
  const easedProgress = transitionEasedProgress(
    linearProgress,
    plan.transition.easing,
  );
  const elapsedMs =
    plan.framerate <= 0
      ? 0
      : Math.min(
          plan.duration_ms,
          Math.floor((clampedFrameIndex * 1000) / plan.framerate),
        );
  const frameWidth = Math.max(1, Math.round(width));
  const frameHeight = Math.max(1, Math.round(height));
  const layers = transitionPreviewLayers(
    plan,
    linearProgress,
    easedProgress,
    elapsedMs,
    frameWidth,
    frameHeight,
  );
  const validation = validateSceneTransitionPreviewPlan(plan);
  const checksum = transitionPreviewChecksum({
    transition_id: plan.transition.id,
    transition_kind: plan.transition.kind,
    frame_index: clampedFrameIndex,
    elapsed_ms: elapsedMs,
    linear_progress: linearProgress,
    eased_progress: easedProgress,
    width: frameWidth,
    height: frameHeight,
    layers,
  });

  return {
    version: 1,
    transition_id: plan.transition.id,
    transition_kind: plan.transition.kind,
    frame_index: clampedFrameIndex,
    elapsed_ms: elapsedMs,
    linear_progress: linearProgress,
    eased_progress: easedProgress,
    width: frameWidth,
    height: frameHeight,
    checksum,
    layers,
    validation,
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

function transitionPreviewLayers(
  plan: SceneTransitionPreviewPlan,
  linearProgress: number,
  easedProgress: number,
  elapsedMs: number,
  width: number,
  height: number,
): SceneTransitionPreviewLayer[] {
  const baseFrom = transitionPreviewLayer("from", plan.from_scene_id, plan.from_scene_name);
  const baseTo = transitionPreviewLayer("to", plan.to_scene_id, plan.to_scene_name);

  switch (plan.transition.kind) {
    case "cut":
      return [
        { ...baseFrom, visible: false, opacity: 0 },
        { ...baseTo, visible: true, opacity: 1 },
      ];
    case "fade":
      return [
        { ...baseFrom, visible: true, opacity: roundPreviewNumber(1 - easedProgress) },
        { ...baseTo, visible: true, opacity: roundPreviewNumber(easedProgress) },
      ];
    case "swipe": {
      const direction = String(plan.transition.config.direction ?? "left");
      const offset = swipePreviewOffsets(direction, easedProgress, width, height);
      return [
        {
          ...baseFrom,
          visible: true,
          offset_x: offset.fromX,
          offset_y: offset.fromY,
        },
        {
          ...baseTo,
          visible: true,
          offset_x: offset.toX,
          offset_y: offset.toY,
        },
      ];
    }
    case "stinger": {
      const triggerMs = Number(
        plan.transition.config.trigger_time_ms ?? Math.floor(plan.duration_ms / 2),
      );
      const triggered = elapsedMs >= Math.max(0, triggerMs);
      const assetUri = String(plan.transition.config.asset_uri ?? "").trim();
      return [
        { ...baseFrom, visible: !triggered, opacity: triggered ? 0 : 1 },
        { ...baseTo, visible: triggered, opacity: triggered ? 1 : 0 },
        {
          role: "stinger",
          scene_id: null,
          scene_name: "",
          label: assetUri
            ? `Stinger placeholder: ${basenameFromUri(assetUri)}`
            : "Stinger placeholder: no asset selected",
          visible: true,
          opacity: 1,
          offset_x: 0,
          offset_y: 0,
        },
      ];
    }
  }
}

function transitionPreviewLayer(
  role: "from" | "to",
  sceneId: string,
  sceneName: string,
): SceneTransitionPreviewLayer {
  return {
    role,
    scene_id: sceneId,
    scene_name: sceneName,
    label: sceneName || sceneId || role,
    visible: true,
    opacity: 1,
    offset_x: 0,
    offset_y: 0,
  };
}

function swipePreviewOffsets(
  direction: string,
  progress: number,
  width: number,
  height: number,
): { fromX: number; fromY: number; toX: number; toY: number } {
  const eased = Math.min(1, Math.max(0, progress));
  switch (direction) {
    case "right":
      return {
        fromX: roundPreviewNumber(width * eased),
        fromY: 0,
        toX: roundPreviewNumber(-width + width * eased),
        toY: 0,
      };
    case "up":
      return {
        fromX: 0,
        fromY: roundPreviewNumber(-height * eased),
        toX: 0,
        toY: roundPreviewNumber(height - height * eased),
      };
    case "down":
      return {
        fromX: 0,
        fromY: roundPreviewNumber(height * eased),
        toX: 0,
        toY: roundPreviewNumber(-height + height * eased),
      };
    case "left":
    default:
      return {
        fromX: roundPreviewNumber(-width * eased),
        fromY: 0,
        toX: roundPreviewNumber(width - width * eased),
        toY: 0,
      };
  }
}

function transitionPreviewChecksum(value: unknown): string {
  const serialized = JSON.stringify(value);
  let hash = 2166136261;
  for (let index = 0; index < serialized.length; index += 1) {
    hash ^= serialized.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

function basenameFromUri(value: string): string {
  const normalized = value.replaceAll("\\", "/");
  return normalized.split("/").filter(Boolean).at(-1) ?? normalized;
}

function roundPreviewNumber(value: number): number {
  return Math.round(value * 1000) / 1000;
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

export function buildAudioGraphRuntimeSnapshot(
  scene: Scene,
  frameIndex = 0,
  generatedAt = new Date().toISOString(),
): AudioGraphRuntimeSnapshot {
  const plan = buildAudioMixerPlan(scene);
  const sources = plan.sources.map((source) =>
    audioGraphRuntimeSource(source, frameIndex),
  );
  const buses = plan.buses.map((bus) => audioGraphRuntimeBus(bus, sources));
  const snapshot: AudioGraphRuntimeSnapshot = {
    version: 1,
    scene_id: plan.scene_id,
    scene_name: plan.scene_name,
    sample_rate: plan.sample_rate,
    channels: plan.channels,
    sources,
    buses,
    generated_at: generatedAt,
    validation: {
      ready: true,
      warnings: [...plan.validation.warnings],
      errors: [...plan.validation.errors],
    },
  };
  snapshot.validation = validateAudioGraphRuntimeSnapshot(snapshot);
  return snapshot;
}

export function validateAudioGraphRuntimeSnapshot(
  snapshot: AudioGraphRuntimeSnapshot,
): AudioGraphRuntimeValidation {
  const warnings = [...snapshot.validation.warnings];
  const errors = [...snapshot.validation.errors];
  const sourceIds = new Set<string>();

  if (!Number.isInteger(snapshot.version) || snapshot.version < 1) {
    errors.push("Audio graph runtime version must be a positive integer.");
  }
  if (!snapshot.scene_id.trim()) {
    errors.push("Audio graph runtime scene id is required.");
  }
  if (!snapshot.scene_name.trim()) {
    errors.push("Audio graph runtime scene name is required.");
  }
  validateNullablePositiveNumber(snapshot.sample_rate, "runtime.audio.sample_rate", errors);
  validateNullablePositiveNumber(snapshot.channels, "runtime.audio.channels", errors);
  if (snapshot.sources.length === 0) {
    warnings.push("Audio graph runtime has no audio meter sources.");
  }

  snapshot.sources.forEach((source) => {
    if (sourceIds.has(source.scene_source_id)) {
      errors.push(`Duplicate audio graph source "${source.scene_source_id}".`);
    }
    sourceIds.add(source.scene_source_id);
    if (!source.scene_source_id.trim()) {
      errors.push("Audio graph source id is required.");
    }
    if (!source.name.trim()) {
      errors.push(`Audio graph source "${source.scene_source_id}" name is required.`);
    }
    validateAudioLevel(source.level_db, source.name, errors);
    validateAudioLevel(source.peak_db, source.name, errors);
    validateAudioLevel(source.pre_filter_level_db, source.name, errors);
    validateAudioLevel(source.pre_filter_peak_db, source.name, errors);
    validateAudioLevel(source.post_filter_level_db, source.name, errors);
    validateAudioLevel(source.post_filter_peak_db, source.name, errors);
    validateLinearAudioLevel(source.linear_level, source.name, "linear level", errors);
    validateLinearAudioLevel(
      source.pre_filter_linear_level,
      source.name,
      "pre-filter linear level",
      errors,
    );
    validateLinearAudioLevel(
      source.post_filter_linear_level,
      source.name,
      "post-filter linear level",
      errors,
    );
    source.filters.forEach((filter) => {
      validateAudioLevel(filter.input_level_db, filter.name, errors);
      validateAudioLevel(filter.output_level_db, filter.name, errors);
      validateAudioLevel(filter.input_peak_db, filter.name, errors);
      validateAudioLevel(filter.output_peak_db, filter.name, errors);
      if (!Number.isFinite(filter.level_change_db)) {
        errors.push(`${filter.name} level change must be finite.`);
      }
      if (
        filter.gain_reduction_db !== undefined &&
        filter.gain_reduction_db !== null &&
        (!Number.isFinite(filter.gain_reduction_db) || filter.gain_reduction_db < 0)
      ) {
        errors.push(`${filter.name} gain reduction must be zero or greater.`);
      }
      if (
        filter.attenuation_db !== undefined &&
        filter.attenuation_db !== null &&
        (!Number.isFinite(filter.attenuation_db) || filter.attenuation_db < 0)
      ) {
        errors.push(`${filter.name} attenuation must be zero or greater.`);
      }
    });
  });

  snapshot.buses.forEach((bus) => {
    validateAudioLevel(bus.level_db, bus.name, errors);
    validateAudioLevel(bus.peak_db, bus.name, errors);
    validateLinearAudioLevel(bus.linear_level, bus.name, "linear bus level", errors);
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
    filters: cloneJson(source.filters ?? []),
  };
}

function audioGraphRuntimeSource(
  source: AudioMixSource,
  frameIndex: number,
): AudioGraphRuntimeSource {
  const simulated = simulatedAudioLevel(source, frameIndex);
  const filtered = applyAudioFilters(source, simulated.levelDb, simulated.peakDb);
  const postFilterLinearLevel = dbToLinear(filtered.levelDb);
  return {
    scene_source_id: source.scene_source_id,
    name: source.name,
    capture_source_id: source.capture_source_id,
    capture_kind: source.capture_kind,
    gain_db: source.gain_db,
    muted: source.muted,
    monitor_enabled: source.monitor_enabled,
    meter_enabled: source.meter_enabled,
    sync_offset_ms: source.sync_offset_ms,
    pre_filter_level_db: simulated.levelDb,
    pre_filter_peak_db: simulated.peakDb,
    pre_filter_linear_level: simulated.linearLevel,
    post_filter_level_db: filtered.levelDb,
    post_filter_peak_db: filtered.peakDb,
    post_filter_linear_level: postFilterLinearLevel,
    level_db: filtered.levelDb,
    peak_db: filtered.peakDb,
    linear_level: postFilterLinearLevel,
    status: source.status,
    status_detail: source.status_detail,
    filters: filtered.filters,
  };
}

function audioGraphRuntimeBus(
  bus: AudioMixBus,
  sources: AudioGraphRuntimeSource[],
): AudioGraphRuntimeBus {
  const linearLevel =
    bus.muted || sources.length === 0
      ? 0
      : Math.min(1, Math.max(...sources.map((source) => source.linear_level)));
  const levelDb = clampAudioLevel(linearToDb(linearLevel) + bus.gain_db);
  const peakDb = clampAudioLevel(levelDb + 4.5);
  return {
    id: bus.id,
    name: bus.name,
    kind: bus.kind,
    gain_db: bus.gain_db,
    muted: bus.muted,
    level_db: levelDb,
    peak_db: peakDb,
    linear_level: linearLevel,
  };
}

function applyAudioFilters(
  source: AudioMixSource,
  levelDb: number,
  peakDb: number,
): {
  levelDb: number;
  peakDb: number;
  filters: AudioFilterRuntimeMetadata[];
} {
  let currentLevelDb = levelDb;
  let currentPeakDb = peakDb;
  const filters: AudioFilterRuntimeMetadata[] = [];

  sortedSceneSourceFilters(source.filters).forEach((filter) => {
    const inputLevelDb = currentLevelDb;
    const inputPeakDb = currentPeakDb;
    if (!filter.enabled) {
      filters.push(
        audioFilterMetadata(
          filter,
          "skipped",
          "Filter is disabled.",
          inputLevelDb,
          inputPeakDb,
          inputLevelDb,
          inputPeakDb,
        ),
      );
      return;
    }

    const result = applyAudioFilter(inputLevelDb, inputPeakDb, filter);
    if (result.status === "applied") {
      currentLevelDb = result.levelDb;
      currentPeakDb = result.peakDb;
      filters.push(
        audioFilterMetadata(
          filter,
          "applied",
          result.detail,
          inputLevelDb,
          inputPeakDb,
          currentLevelDb,
          currentPeakDb,
          {
            gainReductionDb: result.gainReductionDb,
            attenuationDb: result.attenuationDb,
            controlSummary: result.controlSummary,
          },
        ),
      );
      return;
    }

    filters.push(
      audioFilterMetadata(
        filter,
        result.status,
        result.detail,
        inputLevelDb,
        inputPeakDb,
        inputLevelDb,
        inputPeakDb,
      ),
    );
  });

  return { levelDb: currentLevelDb, peakDb: currentPeakDb, filters };
}

function sortedSceneSourceFilters(filters: SceneSourceFilter[]): SceneSourceFilter[] {
  return [...filters].sort(
    (left, right) => left.order - right.order || left.id.localeCompare(right.id),
  );
}

function applyAudioFilter(
  levelDb: number,
  peakDb: number,
  filter: SceneSourceFilter,
):
  | {
      status: "applied";
      levelDb: number;
      peakDb: number;
      detail: string;
      gainReductionDb?: number;
      attenuationDb?: number;
      controlSummary?: string;
    }
  | { status: "skipped" | "error"; detail: string } {
  switch (filter.kind) {
    case "audio_gain":
      return applyAudioGainFilter(levelDb, peakDb, filter);
    case "noise_gate":
      return applyNoiseGateFilter(levelDb, peakDb, filter);
    case "compressor":
      return applyCompressorFilter(levelDb, peakDb, filter);
    default:
      return {
        status: "skipped",
        detail: "Non-audio filter is not applied in the audio graph runtime.",
      };
  }
}

function applyAudioGainFilter(
  levelDb: number,
  peakDb: number,
  filter: SceneSourceFilter,
) {
  const gainDb = audioFilterNumber(filter, "gain_db", -60, 24);
  if (typeof gainDb === "string") return { status: "error" as const, detail: gainDb };
  const outputLevelDb = clampAudioLevel(levelDb + gainDb);
  return {
    status: "applied" as const,
    levelDb: outputLevelDb,
    peakDb: clampAudioLevel(peakDb + gainDb),
    detail: `Applied ${gainDb.toFixed(1)} dB audio gain.`,
    attenuationDb: gainDb < 0 ? Math.max(0, levelDb - outputLevelDb) : undefined,
    controlSummary: `${formatSignedDb(gainDb)} dB`,
  };
}

function applyNoiseGateFilter(
  levelDb: number,
  peakDb: number,
  filter: SceneSourceFilter,
) {
  const closeThresholdDb = audioFilterNumber(filter, "close_threshold_db", -100, 0);
  if (typeof closeThresholdDb === "string") {
    return { status: "error" as const, detail: closeThresholdDb };
  }
  const openThresholdDb = audioFilterNumber(filter, "open_threshold_db", -100, 0);
  if (typeof openThresholdDb === "string") {
    return { status: "error" as const, detail: openThresholdDb };
  }
  if (closeThresholdDb >= openThresholdDb) {
    return {
      status: "error" as const,
      detail: "Noise gate open threshold must be greater than close threshold.",
    };
  }
  const attackMs = audioFilterNumber(filter, "attack_ms", 0, 5_000);
  if (typeof attackMs === "string") return { status: "error" as const, detail: attackMs };
  const releaseMs = audioFilterNumber(filter, "release_ms", 0, 5_000);
  if (typeof releaseMs === "string") return { status: "error" as const, detail: releaseMs };

  let outputLevelDb = levelDb;
  let outputPeakDb = peakDb;
  let detail = `Gate open above ${openThresholdDb.toFixed(1)} dB.`;
  if (levelDb <= closeThresholdDb) {
    outputLevelDb = -90;
    outputPeakDb = -90;
    detail = `Gate closed below ${closeThresholdDb.toFixed(1)} dB.`;
  } else if (levelDb < openThresholdDb) {
    const openness = Math.min(
      1,
      Math.max(0, (levelDb - closeThresholdDb) / (openThresholdDb - closeThresholdDb)),
    );
    outputLevelDb = linearToDb(dbToLinear(levelDb) * openness);
    outputPeakDb = linearToDb(dbToLinear(peakDb) * openness);
    detail = "Gate applied deterministic threshold-band attenuation.";
  }

  return {
    status: "applied" as const,
    levelDb: outputLevelDb,
    peakDb: outputPeakDb,
    detail,
    attenuationDb: Math.max(0, levelDb - outputLevelDb),
    controlSummary: `close ${closeThresholdDb.toFixed(1)} dB / open ${openThresholdDb.toFixed(1)} dB / attack ${attackMs.toFixed(0)} ms / release ${releaseMs.toFixed(0)} ms`,
  };
}

function applyCompressorFilter(
  levelDb: number,
  peakDb: number,
  filter: SceneSourceFilter,
) {
  const thresholdDb = audioFilterNumber(filter, "threshold_db", -100, 0);
  if (typeof thresholdDb === "string") return { status: "error" as const, detail: thresholdDb };
  const ratio = audioFilterNumber(filter, "ratio", 1, 20);
  if (typeof ratio === "string") return { status: "error" as const, detail: ratio };
  const attackMs = audioFilterNumber(filter, "attack_ms", 0, 5_000);
  if (typeof attackMs === "string") return { status: "error" as const, detail: attackMs };
  const releaseMs = audioFilterNumber(filter, "release_ms", 0, 5_000);
  if (typeof releaseMs === "string") return { status: "error" as const, detail: releaseMs };
  const makeupGainDb = audioFilterNumber(filter, "makeup_gain_db", -24, 24);
  if (typeof makeupGainDb === "string") {
    return { status: "error" as const, detail: makeupGainDb };
  }
  const compressedLevelDb = compressAudioLevel(levelDb, thresholdDb, ratio);
  const compressedPeakDb = compressAudioLevel(peakDb, thresholdDb, ratio);
  const outputLevelDb = clampAudioLevel(compressedLevelDb + makeupGainDb);
  return {
    status: "applied" as const,
    levelDb: outputLevelDb,
    peakDb: clampAudioLevel(compressedPeakDb + makeupGainDb),
    detail: `Compressed above ${thresholdDb.toFixed(1)} dB at ${ratio.toFixed(1)}:1 with ${formatSignedDb(makeupGainDb)} dB makeup.`,
    gainReductionDb: Math.max(0, levelDb - compressedLevelDb),
    attenuationDb: outputLevelDb < levelDb ? levelDb - outputLevelDb : undefined,
    controlSummary: `threshold ${thresholdDb.toFixed(1)} dB / ratio ${ratio.toFixed(1)}:1 / attack ${attackMs.toFixed(0)} ms / release ${releaseMs.toFixed(0)} ms / makeup ${formatSignedDb(makeupGainDb)} dB`,
  };
}

function audioFilterMetadata(
  filter: SceneSourceFilter,
  status: AudioFilterRuntimeStatus,
  statusDetail: string,
  inputLevelDb: number,
  inputPeakDb: number,
  outputLevelDb: number,
  outputPeakDb: number,
  options: {
    gainReductionDb?: number;
    attenuationDb?: number;
    controlSummary?: string;
  } = {},
): AudioFilterRuntimeMetadata {
  return {
    id: filter.id,
    name: filter.name,
    kind: filter.kind,
    enabled: filter.enabled,
    order: filter.order,
    status,
    status_detail: statusDetail,
    input_level_db: inputLevelDb,
    output_level_db: outputLevelDb,
    input_peak_db: inputPeakDb,
    output_peak_db: outputPeakDb,
    level_change_db: outputLevelDb - inputLevelDb,
    gain_reduction_db: options.gainReductionDb ?? null,
    attenuation_db: options.attenuationDb ?? null,
    control_summary: options.controlSummary ?? null,
  };
}

function audioFilterNumber(
  filter: SceneSourceFilter,
  key: string,
  min: number,
  max: number,
): number | string {
  const value = filter.config?.[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return `Filter config ${key} must be a number.`;
  }
  if (value < min || value > max) {
    return `Filter config ${key} must be between ${min} and ${max}.`;
  }
  return value;
}

function simulatedAudioLevel(
  source: AudioMixSource,
  frameIndex: number,
): { levelDb: number; peakDb: number; linearLevel: number } {
  if (source.muted || !source.meter_enabled) {
    return { levelDb: -90, peakDb: -90, linearLevel: 0 };
  }
  const seed = stableAudioSeed(source.scene_source_id);
  const phase = (((Math.trunc(frameIndex) * 17 + seed) % 100) / 100);
  const wave = Math.sin(phase * Math.PI * 2) * 0.5 + 0.5;
  const statusOffset =
    source.status === "ready"
      ? 0
      : source.status === "placeholder"
        ? -8
        : source.status === "permission_required"
          ? -12
          : -18;
  const levelDb = clampAudioLevel(-48 + wave * 32 + source.gain_db + statusOffset);
  const peakDb = clampAudioLevel(levelDb + 5 + (seed % 7) * 0.35);
  return { levelDb, peakDb, linearLevel: dbToLinear(levelDb) };
}

function compressAudioLevel(levelDb: number, thresholdDb: number, ratio: number): number {
  return levelDb <= thresholdDb ? levelDb : thresholdDb + (levelDb - thresholdDb) / ratio;
}

function stableAudioSeed(value: string): number {
  return [...value].reduce((hash, char) => hash * 37 + char.charCodeAt(0), 17) >>> 0;
}

function dbToLinear(value: number): number {
  return value <= -90 ? 0 : Math.min(1, Math.max(0, 10 ** (value / 20)));
}

function linearToDb(value: number): number {
  return value <= 0 ? -90 : clampAudioLevel(20 * Math.log10(value));
}

function clampAudioLevel(value: number): number {
  return Math.min(6, Math.max(-90, value));
}

function formatSignedDb(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(1)}`;
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

function validateAudioLevel(value: number, label: string, errors: string[]) {
  if (!Number.isFinite(value) || value < -90 || value > 6) {
    errors.push(`${label} level must be between -90 dB and 6 dB.`);
  }
}

function validateLinearAudioLevel(
  value: number,
  label: string,
  field: string,
  errors: string[],
) {
  if (!Number.isFinite(value) || value < 0 || value > 1) {
    errors.push(`${label} ${field} must be between zero and one.`);
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
        scale_mode: compositorScaleMode(source.bounds_mode),
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

function compositorScaleMode(boundsMode: SceneSourceBoundsMode | undefined): CompositorScaleMode {
  switch (boundsMode) {
    case "stretch":
      return "stretch";
    case "fit":
      return "fit";
    case "fill":
      return "fill";
    case "center":
      return "center";
    case "original_size":
      return "original_size";
    default:
      return "stretch";
  }
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

export function buildOutputPreflightPlan(
  intent: PipelineIntent,
  activeScene: Scene | null | undefined,
  renderPlan: CompositorRenderPlan | null | undefined,
  recordingProfile: MediaProfile | null | undefined,
  streamDestinations: StreamDestination[] = [],
): OutputPreflightPlan {
  const renderTargets =
    renderPlan?.targets.map((target) =>
      renderTargetProfile(target, recordingProfile),
    ) ?? fallbackRenderTargetProfiles(intent, recordingProfile, streamDestinations);
  const recordingTarget =
    intent === "recording" || intent === "recording_and_stream"
      ? recordingProfile
        ? recordingTargetContract(
            recordingProfile,
            preferredRenderTargetId(renderTargets, "recording"),
          )
        : null
      : null;
  const streamingTargets =
    intent === "stream" || intent === "recording_and_stream"
      ? streamDestinations
          .filter((destination) => destination.enabled)
          .map((destination) =>
            streamingTargetContract(
              destination,
              recordingProfile,
              streamTargetProfile(renderTargets, destination),
            ),
          )
      : [];
  const plan: OutputPreflightPlan = {
    version: 1,
    intent,
    active_scene_id: activeScene?.id ?? null,
    active_scene_name: activeScene?.name ?? null,
    render_targets: renderTargets,
    recording_target: recordingTarget,
    streaming_targets: streamingTargets,
    validation: {
      ready: true,
      warnings: [],
      errors: [],
    },
  };
  plan.validation = validateOutputPreflightPlan(plan);
  return plan;
}

export function validateOutputPreflightPlan(
  plan: OutputPreflightPlan,
): OutputPreflightValidation {
  const warnings: string[] = [];
  const errors: string[] = [];
  const targetIds = new Set<string>();
  const needsRecording =
    plan.intent === "recording" || plan.intent === "recording_and_stream";
  const needsStream = plan.intent === "stream" || plan.intent === "recording_and_stream";

  if (!Number.isInteger(plan.version) || plan.version < 1) {
    errors.push("Output preflight plan version must be a positive integer.");
  }
  if (!plan.active_scene_id) {
    warnings.push("Output preflight has no active scene.");
  }
  if (plan.render_targets.length === 0) {
    errors.push("Output preflight requires at least one render target.");
  }

  for (const target of plan.render_targets) {
    if (targetIds.has(target.id)) {
      errors.push(`Duplicate render target profile "${target.id}".`);
    }
    targetIds.add(target.id);
    if (!target.id.trim()) errors.push("Render target profile id is required.");
    if (!target.name.trim()) {
      errors.push(`Render target profile "${target.id}" name is required.`);
    }
    validateGraphPositiveNumber(target.width, `${target.id}.width`, errors);
    validateGraphPositiveNumber(target.height, `${target.id}.height`, errors);
    validateGraphPositiveNumber(target.framerate, `${target.id}.framerate`, errors);
    if (!target.enabled) {
      warnings.push(`Render target profile "${target.id}" is disabled.`);
    }
    validateEncoderPreference(
      target.encoder_preference,
      `Render target profile "${target.id}"`,
      warnings,
      errors,
    );
  }

  if (needsRecording) {
    if (!plan.recording_target) {
      errors.push("Recording output preflight requires a recording target.");
    } else {
      warnings.push(...plan.recording_target.warnings);
      errors.push(...plan.recording_target.errors);
      if (!targetIds.has(plan.recording_target.render_target_id)) {
        errors.push(
          `Recording target references unknown render target "${plan.recording_target.render_target_id}".`,
        );
      }
    }
  }

  if (needsStream) {
    if (plan.streaming_targets.length === 0) {
      errors.push("Stream output preflight requires at least one streaming target.");
    }
    for (const target of plan.streaming_targets) {
      warnings.push(...target.warnings);
      errors.push(...target.errors);
      if (!targetIds.has(target.render_target_id)) {
        errors.push(
          `Streaming target "${target.destination_name}" references unknown render target "${target.render_target_id}".`,
        );
      }
    }
  }

  return {
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

function renderTargetProfile(
  target: CompositorRenderTarget,
  recordingProfile: MediaProfile | null | undefined,
): RenderTargetProfile {
  return {
    id: target.id,
    name: target.name,
    kind: target.kind,
    width: target.width,
    height: target.height,
    framerate: target.framerate,
    frame_format: target.frame_format,
    scale_mode: target.scale_mode,
    enabled: target.enabled,
    encoder_preference: recordingProfile?.encoder_preference ?? "auto",
    bitrate_kbps:
      target.kind === "recording" || target.kind === "stream"
        ? (recordingProfile?.bitrate_kbps ?? 6000)
        : null,
  };
}

function fallbackRenderTargetProfiles(
  intent: PipelineIntent,
  recordingProfile: MediaProfile | null | undefined,
  streamDestinations: StreamDestination[],
): RenderTargetProfile[] {
  const width = recordingProfile?.resolution.width ?? 1920;
  const height = recordingProfile?.resolution.height ?? 1080;
  const framerate = recordingProfile?.framerate ?? 60;
  const targets = [
    fallbackRenderTargetProfile(
      "target-preview",
      "Preview",
      "preview",
      width,
      height,
      framerate,
      recordingProfile,
    ),
    fallbackRenderTargetProfile(
      "target-program",
      "Program",
      "program",
      width,
      height,
      framerate,
      recordingProfile,
    ),
  ];

  if (intent === "recording" || intent === "recording_and_stream") {
    targets.push(
      fallbackRenderTargetProfile(
        "target-recording",
        "Recording Output",
        "recording",
        width,
        height,
        framerate,
        recordingProfile,
      ),
    );
  }

  if (intent === "stream" || intent === "recording_and_stream") {
    if (streamDestinations.length === 0) {
      targets.push(
        fallbackRenderTargetProfile(
          "target-stream",
          "Stream Output",
          "stream",
          width,
          height,
          framerate,
          recordingProfile,
        ),
      );
    } else {
      targets.push(
        ...streamDestinations.map((destination) =>
          fallbackRenderTargetProfile(
            `target-stream-${destination.id}`,
            `Stream Output: ${destination.name}`,
            "stream",
            width,
            height,
            framerate,
            recordingProfile,
          ),
        ),
      );
    }
  }

  return targets;
}

function fallbackRenderTargetProfile(
  id: string,
  name: string,
  kind: CompositorRenderTargetKind,
  width: number,
  height: number,
  framerate: number,
  recordingProfile: MediaProfile | null | undefined,
): RenderTargetProfile {
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
    encoder_preference: recordingProfile?.encoder_preference ?? "auto",
    bitrate_kbps:
      kind === "recording" || kind === "stream"
        ? (recordingProfile?.bitrate_kbps ?? 6000)
        : null,
  };
}

function recordingTargetContract(
  profile: MediaProfile,
  renderTargetId: string,
): RecordingTargetContract {
  const warnings: string[] = [];
  const errors: string[] = [];
  if (!profile.output_folder.trim()) errors.push("Recording output folder is required.");
  if (!profile.filename_pattern.trim()) {
    errors.push("Recording filename pattern is required.");
  }
  if (profile.filename_pattern.includes("/") || profile.filename_pattern.includes("\\")) {
    warnings.push("Recording filename pattern includes path separators.");
  }
  validateGraphPositiveNumber(profile.resolution.width, "recording.width", errors);
  validateGraphPositiveNumber(profile.resolution.height, "recording.height", errors);
  validateGraphPositiveNumber(profile.framerate, "recording.framerate", errors);
  validateGraphPositiveNumber(profile.bitrate_kbps, "recording.bitrate_kbps", errors);
  validateEncoderPreference(
    profile.encoder_preference,
    "Recording target",
    warnings,
    errors,
  );

  return {
    id: `recording-target-${profile.id}`,
    profile_id: profile.id,
    profile_name: profile.name,
    render_target_id: renderTargetId,
    output_folder: profile.output_folder,
    filename_pattern: profile.filename_pattern,
    container: profile.container,
    resolution: profile.resolution,
    framerate: profile.framerate,
    bitrate_kbps: profile.bitrate_kbps,
    encoder_preference: profile.encoder_preference,
    output_path_preview: recordingOutputPathPreview(profile),
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

function streamingTargetContract(
  destination: StreamDestination,
  recordingProfile: MediaProfile | null | undefined,
  renderTarget: RenderTargetProfile | null,
): StreamingTargetContract {
  const warnings: string[] = [];
  const errors: string[] = [];
  const encoderPreference = recordingProfile?.encoder_preference ?? "auto";
  if (!destination.ingest_url.trim()) {
    errors.push(`Stream destination "${destination.name}" requires an ingest URL.`);
  }
  if (!destination.stream_key_ref) {
    warnings.push(`Stream destination "${destination.name}" has no stored stream key.`);
  }
  validateEncoderPreference(
    encoderPreference,
    `Streaming target "${destination.name}"`,
    warnings,
    errors,
  );

  return {
    id: `streaming-target-${destination.id}`,
    destination_id: destination.id,
    destination_name: destination.name,
    platform: destination.platform,
    render_target_id: renderTarget?.id ?? "target-stream",
    ingest_url: destination.ingest_url,
    stream_key_required: true,
    has_stream_key: Boolean(destination.stream_key_ref),
    bandwidth_test: false,
    width: renderTarget?.width ?? 1920,
    height: renderTarget?.height ?? 1080,
    framerate: renderTarget?.framerate ?? 60,
    bitrate_kbps: recordingProfile?.bitrate_kbps ?? 6000,
    encoder_preference: encoderPreference,
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

function streamTargetProfile(
  targets: RenderTargetProfile[],
  destination: StreamDestination,
) {
  return (
    targets.find(
      (target) =>
        target.kind === "stream" && target.id === `target-stream-${destination.id}`,
    ) ??
    targets.find((target) => target.kind === "stream") ??
    targets.find((target) => target.kind === "program") ??
    null
  );
}

function preferredRenderTargetId(
  targets: RenderTargetProfile[],
  kind: "recording" | "stream" | "program",
) {
  return (
    targets.find((target) => target.kind === kind)?.id ??
    targets.find((target) => target.kind === "program")?.id ??
    targets[0]?.id ??
    `target-${kind}`
  );
}

function validateEncoderPreference(
  encoder: EncoderPreference,
  label: string,
  warnings: string[],
  errors: string[],
) {
  if (typeof encoder === "object" && !encoder.named.trim()) {
    errors.push(`${label} named encoder cannot be empty.`);
  } else if (encoder === "hardware") {
    warnings.push(`${label} requests hardware encoding; validate availability on target machine.`);
  }
}

function recordingOutputPathPreview(profile: MediaProfile) {
  const filename = profile.filename_pattern
    .replace("{date}", "2026-05-09")
    .replace("{time}", "12-00-00")
    .replace("{profile}", slugForPath(profile.name));
  return `${profile.output_folder}/${filename}.${profile.container}`;
}

function slugForPath(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function createSceneRuntimeSnapshot(
  collection: SceneCollection,
  options: {
    status?: SceneRuntimeStatus;
    previewEnabled?: boolean;
    metadata?: Record<string, unknown>;
    updatedAt?: string;
  } = {},
): SceneRuntimeSnapshot {
  const activeScene =
    collection.scenes.find((scene) => scene.id === collection.active_scene_id) ??
    collection.scenes[0];
  const activeTransition =
    collection.transitions.find(
      (transition) => transition.id === collection.active_transition_id,
    ) ??
    collection.transitions[0] ??
    defaultSceneTransitions[0];

  return {
    version: 1,
    collection_id: collection.id,
    collection_name: collection.name,
    active_scene_id: activeScene?.id ?? "",
    active_scene_name: activeScene?.name ?? "",
    active_transition_id: activeTransition?.id ?? "",
    active_transition_name: activeTransition?.name ?? "",
    status: options.status ?? "active",
    preview_enabled: options.previewEnabled ?? true,
    metadata: options.metadata ?? { source: "scene_collection" },
    updated_at: options.updatedAt ?? new Date().toISOString(),
    validation: runtimeValidation(
      [],
      validateSceneCollection(collection).ok
        ? []
        : ["Scene collection has validation issues."],
    ),
  };
}

export function validateSceneRuntimeSnapshot(
  snapshot: SceneRuntimeSnapshot,
): SceneRuntimeContractValidation {
  const errors: string[] = [];

  if (!Number.isInteger(snapshot.version) || snapshot.version < 1) {
    errors.push("Scene runtime snapshot version must be a positive integer.");
  }
  if (!snapshot.collection_id.trim()) {
    errors.push("Scene runtime snapshot collection id is required.");
  }
  if (!snapshot.active_scene_id.trim()) {
    errors.push("Scene runtime snapshot active scene id is required.");
  }
  if (!snapshot.active_transition_id.trim()) {
    errors.push("Scene runtime snapshot active transition id is required.");
  }
  if (!["idle", "activating", "active", "transitioning", "error"].includes(snapshot.status)) {
    errors.push("Scene runtime snapshot status is invalid.");
  }
  if (Number.isNaN(Date.parse(snapshot.updated_at))) {
    errors.push("Scene runtime snapshot updated timestamp must be valid.");
  }

  return runtimeValidation(errors, snapshot.validation.warnings);
}

export function createSceneRuntimeCommand(
  kind: SceneRuntimeCommandKind,
  payload: SceneRuntimeCommandPayload,
  options: { commandId?: string; requestedAt?: string } = {},
): SceneRuntimeCommand {
  return {
    version: 1,
    command_id: options.commandId ?? runtimeId("runtime-command"),
    kind,
    requested_at: options.requestedAt ?? new Date().toISOString(),
    payload: cloneJson(payload),
  };
}

export function validateSceneRuntimeCommand(
  command: SceneRuntimeCommand,
): SceneRuntimeContractValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  validateRuntimeEnvelope(
    command.version,
    command.command_id,
    command.requested_at,
    "Scene runtime command",
    errors,
  );
  if (!command.kind.trim()) {
    errors.push("Scene runtime command kind is required.");
  }
  if (!command.payload || typeof command.payload !== "object") {
    errors.push("Scene runtime command payload is required.");
  }

  if (command.payload && typeof command.payload === "object") {
    const validation = validateSceneRuntimeCommandPayload(command);
    warnings.push(...validation.warnings);
    errors.push(...validation.errors);
  }

  return runtimeValidation(errors, warnings);
}

export function createSceneActivationRequest(
  collection: SceneCollection,
  targetSceneId: string,
  options: {
    requestId?: string;
    requestedAt?: string;
    transitionId?: string | null;
    reason?: string | null;
  } = {},
): SceneActivationRequest {
  return {
    version: 1,
    request_id: options.requestId ?? runtimeId("scene-activation"),
    collection_id: collection.id,
    target_scene_id: targetSceneId,
    transition_id: options.transitionId ?? collection.active_transition_id ?? null,
    requested_at: options.requestedAt ?? new Date().toISOString(),
    reason: options.reason ?? null,
  };
}

export function validateSceneActivationRequest(
  request: SceneActivationRequest,
  collection?: SceneCollection,
): SceneRuntimeContractValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  validateRuntimeEnvelope(
    request.version,
    request.request_id,
    request.requested_at,
    "Scene activation request",
    errors,
  );
  if (!request.collection_id.trim()) {
    errors.push("Scene activation collection id is required.");
  }
  if (!request.target_scene_id.trim()) {
    errors.push("Scene activation target scene id is required.");
  }
  if (request.transition_id !== null && !request.transition_id.trim()) {
    errors.push("Scene activation transition id cannot be blank.");
  }

  if (collection) {
    if (collection.id !== request.collection_id) {
      errors.push("Scene activation request collection id does not match collection.");
    }
    if (!collection.scenes.some((scene) => scene.id === request.target_scene_id)) {
      errors.push(
        `Scene activation target scene "${request.target_scene_id}" does not exist.`,
      );
    }
    if (
      request.transition_id &&
      !collection.transitions.some(
        (transition) => transition.id === request.transition_id,
      )
    ) {
      errors.push(
        `Scene activation transition "${request.transition_id}" does not exist.`,
      );
    }
    const collectionValidation = validateSceneCollection(collection);
    if (!collectionValidation.ok) {
      warnings.push(
        `Scene collection has ${collectionValidation.issues.length} validation issue(s).`,
      );
    }
  }

  return runtimeValidation(errors, warnings);
}

export function createSceneActivationResponse(
  request: SceneActivationRequest,
  collection: SceneCollection,
  options: {
    previousSceneId?: string | null;
    status?: SceneActivationStatus;
    activatedAt?: string;
  } = {},
): SceneActivationResponse {
  const validation = validateSceneActivationRequest(request, collection);
  return {
    version: 1,
    request_id: request.request_id,
    collection_id: request.collection_id,
    previous_scene_id: options.previousSceneId ?? collection.active_scene_id ?? null,
    active_scene_id: request.target_scene_id,
    transition_id: request.transition_id,
    status: options.status ?? (validation.ready ? "accepted" : "rejected"),
    activated_at: options.activatedAt ?? new Date().toISOString(),
    runtime: createSceneRuntimeSnapshot(collection, {
      status: validation.ready ? "active" : "error",
      metadata: { request_id: request.request_id },
    }),
    validation,
  };
}

export function validateSceneActivationResponse(
  response: SceneActivationResponse,
  collection?: SceneCollection,
): SceneRuntimeContractValidation {
  const warnings = [...response.validation.warnings];
  const errors = [...response.validation.errors];

  validateRuntimeEnvelope(
    response.version,
    response.request_id,
    response.activated_at,
    "Scene activation response",
    errors,
  );
  if (!response.collection_id.trim()) {
    errors.push("Scene activation response collection id is required.");
  }
  if (!response.active_scene_id.trim()) {
    errors.push("Scene activation response active scene id is required.");
  }
  if (
    response.previous_scene_id !== null &&
    typeof response.previous_scene_id === "string" &&
    !response.previous_scene_id.trim()
  ) {
    errors.push("Scene activation response previous scene id cannot be blank.");
  }
  if (response.transition_id !== null && !response.transition_id.trim()) {
    errors.push("Scene activation response transition id cannot be blank.");
  }
  if (!["accepted", "rejected"].includes(response.status)) {
    errors.push("Scene activation response status is invalid.");
  }
  if (response.status === "accepted" && response.validation.errors.length > 0) {
    errors.push("Accepted scene activation responses cannot contain validation errors.");
  }
  const snapshotValidation = validateSceneRuntimeSnapshot(response.runtime);
  warnings.push(...snapshotValidation.warnings);
  errors.push(...snapshotValidation.errors);

  if (collection) {
    if (collection.id !== response.collection_id) {
      errors.push("Scene activation response collection id does not match collection.");
    }
    if (!collection.scenes.some((scene) => scene.id === response.active_scene_id)) {
      errors.push(
        `Scene activation response active scene "${response.active_scene_id}" does not exist.`,
      );
    }
    if (
      response.previous_scene_id &&
      !collection.scenes.some((scene) => scene.id === response.previous_scene_id)
    ) {
      errors.push(
        `Scene activation response previous scene "${response.previous_scene_id}" does not exist.`,
      );
    }
    if (
      response.transition_id &&
      !collection.transitions.some((transition) => transition.id === response.transition_id)
    ) {
      errors.push(
        `Scene activation response transition "${response.transition_id}" does not exist.`,
      );
    }
  }

  return runtimeValidation(errors, warnings);
}

export function createSceneRuntimeStateUpdateRequest(
  collection: SceneCollection,
  patch: SceneRuntimeStatePatch,
  options: { requestId?: string; requestedAt?: string } = {},
): SceneRuntimeStateUpdateRequest {
  return {
    version: 1,
    request_id: options.requestId ?? runtimeId("scene-state"),
    collection_id: collection.id,
    patch: cloneJson(patch),
    requested_at: options.requestedAt ?? new Date().toISOString(),
  };
}

export function validateSceneRuntimeStateUpdateRequest(
  request: SceneRuntimeStateUpdateRequest,
  collection?: SceneCollection,
): SceneRuntimeContractValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  validateRuntimeEnvelope(
    request.version,
    request.request_id,
    request.requested_at,
    "Scene runtime state update request",
    errors,
  );
  if (!request.collection_id.trim()) {
    errors.push("Scene runtime state update collection id is required.");
  }
  if (!request.patch || typeof request.patch !== "object") {
    errors.push("Scene runtime state patch is required.");
  }
  if (request.patch.active_scene_id === "") {
    errors.push("Scene runtime state active scene id cannot be blank.");
  }
  if (request.patch.active_transition_id === "") {
    errors.push("Scene runtime state active transition id cannot be blank.");
  }

  if (collection) {
    if (collection.id !== request.collection_id) {
      errors.push("Scene runtime state update collection id does not match collection.");
    }
    if (
      request.patch.active_scene_id &&
      !collection.scenes.some((scene) => scene.id === request.patch.active_scene_id)
    ) {
      errors.push(
        `Scene runtime state active scene "${request.patch.active_scene_id}" does not exist.`,
      );
    }
    if (
      request.patch.active_transition_id &&
      !collection.transitions.some(
        (transition) => transition.id === request.patch.active_transition_id,
      )
    ) {
      errors.push(
        `Scene runtime state active transition "${request.patch.active_transition_id}" does not exist.`,
      );
    }
  }

  return runtimeValidation(errors, warnings);
}

export function createSceneRuntimeStateUpdateResponse(
  request: SceneRuntimeStateUpdateRequest,
  collection: SceneCollection,
  options: { updatedAt?: string } = {},
): SceneRuntimeStateUpdateResponse {
  const validation = validateSceneRuntimeStateUpdateRequest(request, collection);
  return {
    version: 1,
    request_id: request.request_id,
    collection_id: request.collection_id,
    active_scene_id: request.patch.active_scene_id ?? collection.active_scene_id,
    active_transition_id:
      request.patch.active_transition_id ?? collection.active_transition_id,
    status: request.patch.status ?? "active",
    updated_at: options.updatedAt ?? new Date().toISOString(),
    validation,
  };
}

export function validateSceneRuntimeStateUpdateResponse(
  response: SceneRuntimeStateUpdateResponse,
  collection?: SceneCollection,
): SceneRuntimeContractValidation {
  const warnings = [...response.validation.warnings];
  const errors = [...response.validation.errors];

  validateRuntimeEnvelope(
    response.version,
    response.request_id,
    response.updated_at,
    "Scene runtime state update response",
    errors,
  );
  if (!response.collection_id.trim()) {
    errors.push("Scene runtime state update response collection id is required.");
  }
  if (!response.active_scene_id.trim()) {
    errors.push("Scene runtime state update response active scene id is required.");
  }
  if (!response.active_transition_id.trim()) {
    errors.push("Scene runtime state update response active transition id is required.");
  }
  if (!["idle", "activating", "active", "transitioning", "error"].includes(response.status)) {
    errors.push("Scene runtime state update response status is invalid.");
  }

  if (collection) {
    if (collection.id !== response.collection_id) {
      errors.push(
        "Scene runtime state update response collection id does not match collection.",
      );
    }
    if (!collection.scenes.some((scene) => scene.id === response.active_scene_id)) {
      errors.push(
        `Scene runtime state update response active scene "${response.active_scene_id}" does not exist.`,
      );
    }
    if (
      !collection.transitions.some(
        (transition) => transition.id === response.active_transition_id,
      )
    ) {
      errors.push(
        `Scene runtime state update response active transition "${response.active_transition_id}" does not exist.`,
      );
    }
  }

  return runtimeValidation(errors, warnings);
}

export function createPreviewFrameRequest(
  scene: Scene,
  options: Partial<
    Pick<
      PreviewFrameRequest,
      | "request_id"
      | "width"
      | "height"
      | "framerate"
      | "frame_format"
      | "scale_mode"
      | "encoding"
      | "include_debug_overlay"
      | "requested_at"
    >
  > = {},
): PreviewFrameRequest {
  return {
    version: 1,
    request_id: options.request_id ?? runtimeId("preview-frame"),
    scene_id: scene.id,
    width: options.width ?? scene.canvas.width,
    height: options.height ?? scene.canvas.height,
    framerate: options.framerate ?? 30,
    frame_format: options.frame_format ?? "rgba8",
    scale_mode: options.scale_mode ?? "fit",
    encoding: options.encoding ?? "data_url",
    include_debug_overlay: options.include_debug_overlay ?? false,
    requested_at: options.requested_at ?? new Date().toISOString(),
  };
}

export function validatePreviewFrameRequest(
  request: PreviewFrameRequest,
): SceneRuntimeContractValidation {
  const errors: string[] = [];

  validateRuntimeEnvelope(
    request.version,
    request.request_id,
    request.requested_at,
    "Preview frame request",
    errors,
  );
  if (!request.scene_id.trim()) {
    errors.push("Preview frame scene id is required.");
  }
  validateGraphPositiveNumber(request.width, "preview.width", errors);
  validateGraphPositiveNumber(request.height, "preview.height", errors);
  validateGraphPositiveNumber(request.framerate, "preview.framerate", errors);
  if (request.width > 7680 || request.height > 4320) {
    errors.push("Preview frame dimensions must be 8K or smaller.");
  }

  return runtimeValidation(errors, []);
}

export function createCompositorRenderRequest(
  plan: CompositorRenderPlan,
  options: {
    requestId?: string;
    requestedAt?: string;
    frameIndex?: number;
    framerate?: number;
    renderer?: CompositorRendererKind;
  } = {},
): CompositorRenderRequest {
  const framerate =
    options.framerate ??
    plan.targets.find((target) => target.enabled)?.framerate ??
    60;
  const frameIndex = options.frameIndex ?? 0;
  const durationNanos = Math.floor(1_000_000_000 / Math.max(1, framerate));

  return {
    version: 1,
    request_id: options.requestId ?? runtimeId("compositor-render"),
    renderer: options.renderer ?? plan.renderer,
    plan: cloneJson(plan),
    clock: {
      frame_index: frameIndex,
      framerate,
      pts_nanos: frameIndex * durationNanos,
      duration_nanos: durationNanos,
    },
    requested_at: options.requestedAt ?? new Date().toISOString(),
  };
}

export function validateCompositorRenderRequest(
  request: CompositorRenderRequest,
): SceneRuntimeContractValidation {
  const renderValidation = validateCompositorRenderPlan(request.plan);
  const warnings = [...renderValidation.warnings];
  const errors = [...renderValidation.errors];

  validateRuntimeEnvelope(
    request.version,
    request.request_id,
    request.requested_at,
    "Compositor render request",
    errors,
  );
  if (!Number.isInteger(request.clock.frame_index) || request.clock.frame_index < 0) {
    errors.push("Compositor render clock frame index must be zero or greater.");
  }
  validateGraphPositiveNumber(request.clock.framerate, "render.clock.framerate", errors);
  validateGraphNonNegativeNumber(request.clock.pts_nanos, "render.clock.pts_nanos", errors);
  validateGraphPositiveNumber(
    request.clock.duration_nanos,
    "render.clock.duration_nanos",
    errors,
  );

  return runtimeValidation(errors, warnings);
}

export function createCompositorRenderResponse(
  request: CompositorRenderRequest,
  frame: CompositorRenderedFrame,
  options: {
    renderedAt?: string;
    renderTimeMs?: number;
    targetResults?: CompositorRenderTargetResult[];
  } = {},
): CompositorRenderResponse {
  return {
    version: 1,
    request_id: request.request_id,
    renderer: frame.renderer,
    scene_id: frame.scene_id,
    scene_name: frame.scene_name,
    frame: cloneJson(frame),
    target_results:
      options.targetResults ??
      frame.targets.map((target) => ({
        target_id: target.target_id,
        target_kind: target.target_kind,
        width: target.width,
        height: target.height,
        frame_format: target.frame_format,
        checksum: null,
        byte_length: null,
      })),
    render_time_ms: options.renderTimeMs ?? 0,
    rendered_at: options.renderedAt ?? new Date().toISOString(),
    validation: runtimeValidation([], []),
  };
}

export function validateCompositorRenderResponse(
  response: CompositorRenderResponse,
): SceneRuntimeContractValidation {
  const warnings = [...response.frame.validation.warnings];
  const errors = [...response.frame.validation.errors];
  const targetIds = new Set(response.frame.targets.map((target) => target.target_id));

  validateRuntimeEnvelope(
    response.version,
    response.request_id,
    response.rendered_at,
    "Compositor render response",
    errors,
  );
  if (response.scene_id !== response.frame.scene_id) {
    errors.push("Compositor render response scene id must match rendered frame.");
  }
  validateGraphNonNegativeNumber(
    response.render_time_ms,
    "render.response.render_time_ms",
    errors,
  );
  response.target_results.forEach((target) => {
    if (!targetIds.has(target.target_id)) {
      errors.push(`Render response target "${target.target_id}" is not in the frame.`);
    }
    validateGraphPositiveNumber(target.width, `${target.target_id}.width`, errors);
    validateGraphPositiveNumber(target.height, `${target.target_id}.height`, errors);
    if (target.byte_length !== null) {
      validateGraphNonNegativeNumber(
        target.byte_length,
        `${target.target_id}.byte_length`,
        errors,
      );
    }
  });

  return runtimeValidation(errors, warnings);
}

export function createPreviewFrameResponse(
  request: PreviewFrameRequest,
  frame: CompositorRenderedFrame | null,
  options: {
    sceneName?: string;
    imageData?: string | null;
    checksum?: string | null;
    renderTimeMs?: number;
    generatedAt?: string;
  } = {},
): PreviewFrameResponse {
  const validation = frame
    ? runtimeValidation(frame.validation.errors, frame.validation.warnings)
    : runtimeValidation([], ["Preview frame response has no rendered frame payload."]);

  return {
    version: 1,
    request_id: request.request_id,
    scene_id: request.scene_id,
    scene_name: options.sceneName ?? frame?.scene_name ?? "",
    frame_index: frame?.clock.frame_index ?? 0,
    width: request.width,
    height: request.height,
    frame_format: request.frame_format,
    encoding: request.encoding,
    image_data: options.imageData ?? null,
    checksum: options.checksum ?? null,
    render_time_ms: options.renderTimeMs ?? 0,
    generated_at: options.generatedAt ?? new Date().toISOString(),
    rendered_frame: frame ? cloneJson(frame) : null,
    validation,
  };
}

export function validatePreviewFrameResponse(
  response: PreviewFrameResponse,
): SceneRuntimeContractValidation {
  const warnings = [...response.validation.warnings];
  const errors = [...response.validation.errors];

  validateRuntimeEnvelope(
    response.version,
    response.request_id,
    response.generated_at,
    "Preview frame response",
    errors,
  );
  if (!response.scene_id.trim()) {
    errors.push("Preview frame response scene id is required.");
  }
  if (!response.scene_name.trim()) {
    errors.push("Preview frame response scene name is required.");
  }
  validateGraphPositiveNumber(response.width, "preview.response.width", errors);
  validateGraphPositiveNumber(response.height, "preview.response.height", errors);
  validateGraphNonNegativeNumber(
    response.render_time_ms,
    "preview.response.render_time_ms",
    errors,
  );
  if (response.encoding !== "none" && !response.image_data && !response.checksum) {
    warnings.push("Preview frame response has no image data or checksum.");
  }
  if (response.rendered_frame && response.rendered_frame.scene_id !== response.scene_id) {
    errors.push("Preview frame response scene id must match rendered frame.");
  }

  return runtimeValidation(errors, warnings);
}

export function buildRuntimeCaptureSourceBindingContract(
  scene: Scene,
): RuntimeCaptureSourceBindingContract {
  const framePlan = buildCaptureFramePlan(scene);
  const contract: RuntimeCaptureSourceBindingContract = {
    version: 1,
    scene_id: scene.id,
    scene_name: scene.name,
    bindings: framePlan.bindings.map((binding) => {
      const source = scene.sources.find(
        (candidate) => candidate.id === binding.scene_source_id,
      );
      return {
        scene_source_id: binding.scene_source_id,
        scene_source_name: binding.scene_source_name,
        scene_source_kind: source?.kind ?? "display",
        capture_source_id: binding.capture_source_id,
        capture_kind: binding.capture_kind,
        media_kind: binding.media_kind,
        frame_format: binding.format,
        width: binding.width,
        height: binding.height,
        framerate: binding.framerate,
        sample_rate: binding.sample_rate,
        channels: binding.channels,
        required: source?.visible ?? true,
        status: binding.status,
        status_detail: binding.status_detail,
      };
    }),
    validation: runtimeValidation([], []),
  };
  contract.validation = validateRuntimeCaptureSourceBindingContract(contract);
  return contract;
}

export function validateRuntimeCaptureSourceBindingContract(
  contract: RuntimeCaptureSourceBindingContract,
): SceneRuntimeContractValidation {
  const warnings: string[] = [];
  const errors: string[] = [];
  const sourceIds = new Set<string>();

  if (!Number.isInteger(contract.version) || contract.version < 1) {
    errors.push("Runtime capture binding contract version must be positive.");
  }
  if (!contract.scene_id.trim()) {
    errors.push("Runtime capture binding scene id is required.");
  }
  if (!contract.scene_name.trim()) {
    errors.push("Runtime capture binding scene name is required.");
  }
  if (contract.bindings.length === 0) {
    warnings.push("Runtime capture binding contract has no bindings.");
  }

  contract.bindings.forEach((binding) => {
    if (sourceIds.has(binding.scene_source_id)) {
      errors.push(`Duplicate runtime capture binding "${binding.scene_source_id}".`);
    }
    sourceIds.add(binding.scene_source_id);
    if (!binding.scene_source_id.trim()) {
      errors.push("Runtime capture binding source id is required.");
    }
    if (!binding.scene_source_name.trim()) {
      errors.push(`Runtime capture binding "${binding.scene_source_id}" name is required.`);
    }
    if (binding.required && binding.status !== "ready") {
      warnings.push(
        `${binding.scene_source_name} capture is ${binding.status}: ${binding.status_detail}`,
      );
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
  });

  return runtimeValidation(errors, warnings);
}

export function buildRuntimeAudioSourceBindingContract(
  scene: Scene,
): RuntimeAudioSourceBindingContract {
  const mixerPlan = buildAudioMixerPlan(scene);
  const busIds = mixerPlan.buses.map((bus) => bus.id);
  const contract: RuntimeAudioSourceBindingContract = {
    version: 1,
    scene_id: scene.id,
    scene_name: scene.name,
    sample_rate: mixerPlan.sample_rate,
    channels: mixerPlan.channels,
    bindings: mixerPlan.sources.map((source) => ({
      scene_source_id: source.scene_source_id,
      scene_source_name: source.name,
      capture_source_id: source.capture_source_id,
      capture_kind: source.capture_kind,
      bus_ids: busIds,
      gain_db: source.gain_db,
      muted: source.muted,
      monitor_enabled: source.monitor_enabled,
      meter_enabled: source.meter_enabled,
      sync_offset_ms: source.sync_offset_ms,
      status: source.status,
      status_detail: source.status_detail,
    })),
    buses: cloneJson(mixerPlan.buses),
    validation: runtimeValidation([], []),
  };
  contract.validation = validateRuntimeAudioSourceBindingContract(contract);
  return contract;
}

export function validateRuntimeAudioSourceBindingContract(
  contract: RuntimeAudioSourceBindingContract,
): SceneRuntimeContractValidation {
  const warnings: string[] = [];
  const errors: string[] = [];
  const sourceIds = new Set<string>();
  const busIds = new Set(contract.buses.map((bus) => bus.id));

  if (!Number.isInteger(contract.version) || contract.version < 1) {
    errors.push("Runtime audio binding contract version must be positive.");
  }
  if (!contract.scene_id.trim()) {
    errors.push("Runtime audio binding scene id is required.");
  }
  validateNullablePositiveNumber(contract.sample_rate, "runtime.audio.sample_rate", errors);
  validateNullablePositiveNumber(contract.channels, "runtime.audio.channels", errors);
  if (!contract.buses.some((bus) => bus.kind === "master")) {
    errors.push("Runtime audio binding contract requires a master bus.");
  }
  if (contract.bindings.length === 0) {
    warnings.push("Runtime audio binding contract has no audio bindings.");
  }

  contract.bindings.forEach((binding) => {
    if (sourceIds.has(binding.scene_source_id)) {
      errors.push(`Duplicate runtime audio binding "${binding.scene_source_id}".`);
    }
    sourceIds.add(binding.scene_source_id);
    if (!binding.scene_source_id.trim()) {
      errors.push("Runtime audio binding source id is required.");
    }
    validateGain(binding.gain_db, binding.scene_source_name, errors);
    binding.bus_ids.forEach((busId) => {
      if (!busIds.has(busId)) {
        errors.push(`${binding.scene_source_name} references missing bus "${busId}".`);
      }
    });
    if (binding.status !== "ready") {
      warnings.push(
        `${binding.scene_source_name} audio is ${binding.status}: ${binding.status_detail}`,
      );
    }
  });

  return runtimeValidation(errors, warnings);
}

export function createTransitionExecutionRequest(
  collection: SceneCollection,
  fromSceneId: string,
  toSceneId: string,
  options: {
    requestId?: string;
    requestedAt?: string;
    transitionId?: string;
    framerate?: number;
  } = {},
): TransitionExecutionRequest {
  return {
    version: 1,
    request_id: options.requestId ?? runtimeId("transition"),
    collection_id: collection.id,
    transition_id: options.transitionId ?? collection.active_transition_id,
    from_scene_id: fromSceneId,
    to_scene_id: toSceneId,
    framerate: options.framerate ?? 60,
    requested_at: options.requestedAt ?? new Date().toISOString(),
  };
}

export function validateTransitionExecutionRequest(
  request: TransitionExecutionRequest,
  collection?: SceneCollection,
): SceneRuntimeContractValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  validateRuntimeEnvelope(
    request.version,
    request.request_id,
    request.requested_at,
    "Transition execution request",
    errors,
  );
  if (!request.collection_id.trim()) {
    errors.push("Transition execution collection id is required.");
  }
  if (!request.transition_id.trim()) {
    errors.push("Transition execution transition id is required.");
  }
  if (!request.from_scene_id.trim()) {
    errors.push("Transition execution from scene id is required.");
  }
  if (!request.to_scene_id.trim()) {
    errors.push("Transition execution to scene id is required.");
  }
  validateGraphPositiveNumber(request.framerate, "transition.framerate", errors);
  if (request.from_scene_id === request.to_scene_id) {
    warnings.push("Transition execution uses the same from and to scene.");
  }

  if (collection) {
    if (collection.id !== request.collection_id) {
      errors.push("Transition execution collection id does not match collection.");
    }
    if (!collection.scenes.some((scene) => scene.id === request.from_scene_id)) {
      errors.push(`Transition from scene "${request.from_scene_id}" does not exist.`);
    }
    if (!collection.scenes.some((scene) => scene.id === request.to_scene_id)) {
      errors.push(`Transition to scene "${request.to_scene_id}" does not exist.`);
    }
    if (!collection.transitions.some((transition) => transition.id === request.transition_id)) {
      errors.push(`Transition "${request.transition_id}" does not exist.`);
    }
  }

  return runtimeValidation(errors, warnings);
}

export function createTransitionExecutionResponse(
  request: TransitionExecutionRequest,
  collection: SceneCollection,
  options: { startedAt?: string } = {},
): TransitionExecutionResponse {
  const previewPlan = buildSceneTransitionPreviewPlan(
    {
      ...collection,
      active_transition_id: request.transition_id,
    },
    request.from_scene_id,
    request.to_scene_id,
    request.framerate,
  );
  return {
    version: 1,
    request_id: request.request_id,
    collection_id: request.collection_id,
    transition_id: request.transition_id,
    from_scene_id: request.from_scene_id,
    to_scene_id: request.to_scene_id,
    started_at: options.startedAt ?? new Date().toISOString(),
    preview_plan: previewPlan,
    validation: runtimeValidation(
      previewPlan.validation.errors,
      previewPlan.validation.warnings,
    ),
  };
}

function runtimeId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`;
}

function runtimeValidation(
  errors: string[],
  warnings: string[],
): SceneRuntimeContractValidation {
  return {
    ready: errors.length === 0,
    warnings,
    errors,
  };
}

function validateRuntimeEnvelope(
  version: number,
  id: string,
  timestamp: string,
  label: string,
  errors: string[],
) {
  if (!Number.isInteger(version) || version < 1) {
    errors.push(`${label} version must be a positive integer.`);
  }
  if (typeof id !== "string" || !id.trim()) {
    errors.push(`${label} id is required.`);
  }
  if (typeof timestamp !== "string" || !timestamp.trim()) {
    errors.push(`${label} timestamp is required.`);
    return;
  }
  if (Number.isNaN(Date.parse(timestamp))) {
    errors.push(`${label} timestamp must be a valid ISO-8601 timestamp.`);
  }
}

function validateSceneRuntimeCommandPayload(
  command: SceneRuntimeCommand,
): SceneRuntimeContractValidation {
  switch (command.kind) {
    case "activate_scene":
      return validateSceneActivationRequest(command.payload as SceneActivationRequest);
    case "update_runtime_state":
      return validateSceneRuntimeStateUpdateRequest(
        command.payload as SceneRuntimeStateUpdateRequest,
      );
    case "request_preview_frame":
      return validatePreviewFrameRequest(command.payload as PreviewFrameRequest);
    case "validate_runtime_graph":
      return validateCompositorRenderRequest(command.payload as CompositorRenderRequest);
    case "execute_transition":
      return validateTransitionExecutionRequest(command.payload as TransitionExecutionRequest);
    default:
      return runtimeValidation([`Unsupported scene runtime command kind "${command.kind}".`], []);
  }
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
  const sourceRect = nodeBoundsRect(transform, node);
  const { scaleX, scaleY, offsetX, offsetY } = targetMapping(graph.output, target);
  return {
    node_id: node.id,
    source_id: node.source_id,
    name: node.name,
    role: node.role,
    status: node.status,
    status_detail: node.status_detail,
    asset: null,
    text: null,
    filters: [],
    rect: {
      x: offsetX + sourceRect.x * scaleX,
      y: offsetY + sourceRect.y * scaleY,
      width: sourceRect.width * scaleX,
      height: sourceRect.height * scaleY,
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

function nodeBoundsRect(
  transform: CompositorTransform,
  node: CompositorNode,
): CompositorRect {
  const bounds: CompositorRect = {
    x: transform.position.x,
    y: transform.position.y,
    width: transform.size.width,
    height: transform.size.height,
  };
  const nativeSize = nodeNativeSize(node, transform);

  if (node.scale_mode === "fit") {
    const scale = Math.min(bounds.width / nativeSize.width, bounds.height / nativeSize.height);
    return centeredRect(bounds, nativeSize.width * scale, nativeSize.height * scale);
  }
  if (node.scale_mode === "fill") {
    const scale = Math.max(bounds.width / nativeSize.width, bounds.height / nativeSize.height);
    return centeredRect(bounds, nativeSize.width * scale, nativeSize.height * scale);
  }
  if (node.scale_mode === "center") {
    return centeredRect(bounds, nativeSize.width, nativeSize.height);
  }
  if (node.scale_mode === "original_size") {
    return {
      x: bounds.x,
      y: bounds.y,
      width: nativeSize.width,
      height: nativeSize.height,
    };
  }

  return bounds;
}

function centeredRect(bounds: CompositorRect, width: number, height: number): CompositorRect {
  return {
    x: bounds.x + (bounds.width - width) / 2,
    y: bounds.y + (bounds.height - height) / 2,
    width,
    height,
  };
}

function nodeNativeSize(node: CompositorNode, transform: CompositorTransform): SceneSize {
  const size =
    node.source_kind === "browser_overlay"
      ? configSize(node.config, "viewport")
      : node.source_kind === "display" ||
          node.source_kind === "window" ||
          node.source_kind === "camera"
        ? configSize(node.config, "resolution")
        : null;

  return {
    width: Math.max(1, size?.width ?? transform.size.width),
    height: Math.max(1, size?.height ?? transform.size.height),
  };
}

function configSize(config: SceneSourceConfig, key: "resolution" | "viewport"): SceneSize | null {
  const value =
    key === "resolution" && "resolution" in config
      ? config.resolution
      : key === "viewport" && "viewport" in config
        ? config.viewport
        : null;

  if (
    value &&
    Number.isFinite(value.width) &&
    Number.isFinite(value.height) &&
    value.width > 0 &&
    value.height > 0
  ) {
    return { width: value.width, height: value.height };
  }

  return null;
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
  if (target.scale_mode === "center" || target.scale_mode === "original_size") {
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
    validateSceneSourceFilterConfig(filter, filterPath, issues);
  });
}

function validateSceneSourceFilterConfig(
  filter: SceneSourceFilter,
  filterPath: string,
  issues: SceneValidationIssue[],
) {
  switch (filter.kind) {
    case "color_correction":
      validateFilterNumberRange(filter, filterPath, "brightness", -1, 1, issues);
      validateFilterNumberRange(filter, filterPath, "contrast", 0, 4, issues);
      validateFilterNumberRange(filter, filterPath, "saturation", 0, 4, issues);
      validateFilterNumberRange(filter, filterPath, "gamma", 0.01, 4, issues);
      break;
    case "chroma_key":
      validateFilterRequiredString(filter, filterPath, "key_color", issues);
      validateFilterNumberRange(filter, filterPath, "similarity", 0, 1, issues);
      validateFilterNumberRange(filter, filterPath, "smoothness", 0, 1, issues);
      break;
    case "crop_pad":
      ["top", "right", "bottom", "left"].forEach((key) =>
        validateFilterNumberRange(filter, filterPath, key, 0, 100_000, issues),
      );
      break;
    case "mask_blend":
      validateFilterOptionalUri(filter, filterPath, "mask_uri", issues);
      validateFilterStringEnum(
        filter,
        filterPath,
        "blend_mode",
        ["normal", "multiply", "screen", "overlay", "alpha"],
        issues,
      );
      break;
    case "blur":
      validateFilterNumberRange(filter, filterPath, "radius", 0, 100, issues);
      break;
    case "sharpen":
      validateFilterNumberRange(filter, filterPath, "amount", 0, 5, issues);
      break;
    case "lut":
      validateFilterOptionalUri(filter, filterPath, "lut_uri", issues);
      validateFilterNumberRange(filter, filterPath, "strength", 0, 1, issues);
      break;
    case "audio_gain":
      validateFilterNumberRange(filter, filterPath, "gain_db", -60, 24, issues);
      break;
    case "noise_gate": {
      const close = validateFilterNumberRange(
        filter,
        filterPath,
        "close_threshold_db",
        -100,
        0,
        issues,
      );
      const open = validateFilterNumberRange(
        filter,
        filterPath,
        "open_threshold_db",
        -100,
        0,
        issues,
      );
      if (close !== null && open !== null && close >= open) {
        issues.push({
          path: `${filterPath}.config.open_threshold_db`,
          message: "Noise gate open threshold must be greater than close threshold.",
        });
      }
      validateFilterNumberRange(filter, filterPath, "attack_ms", 0, 5_000, issues);
      validateFilterNumberRange(filter, filterPath, "release_ms", 0, 5_000, issues);
      break;
    }
    case "compressor":
      validateFilterNumberRange(filter, filterPath, "threshold_db", -100, 0, issues);
      validateFilterNumberRange(filter, filterPath, "ratio", 1, 20, issues);
      validateFilterNumberRange(filter, filterPath, "attack_ms", 0, 5_000, issues);
      validateFilterNumberRange(filter, filterPath, "release_ms", 0, 5_000, issues);
      validateFilterNumberRange(filter, filterPath, "makeup_gain_db", -24, 24, issues);
      break;
  }
}

function validateFilterNumberRange(
  filter: SceneSourceFilter,
  filterPath: string,
  key: string,
  min: number,
  max: number,
  issues: SceneValidationIssue[],
): number | null {
  const path = `${filterPath}.config.${key}`;
  const value = filter.config?.[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    issues.push({ path, message: `Filter config ${key} must be a number.` });
    return null;
  }
  if (value < min || value > max) {
    issues.push({
      path,
      message: `Filter config ${key} must be between ${min} and ${max}.`,
    });
    return null;
  }
  return value;
}

function validateFilterRequiredString(
  filter: SceneSourceFilter,
  filterPath: string,
  key: string,
  issues: SceneValidationIssue[],
) {
  const value = filter.config?.[key];
  if (typeof value !== "string" || !value.trim()) {
    issues.push({
      path: `${filterPath}.config.${key}`,
      message: `Filter config ${key} is required.`,
    });
  }
}

function validateFilterOptionalUri(
  filter: SceneSourceFilter,
  filterPath: string,
  key: string,
  issues: SceneValidationIssue[],
) {
  const value = filter.config?.[key];
  if (value === undefined || value === null) return;
  if (typeof value !== "string" || !value.trim()) {
    issues.push({
      path: `${filterPath}.config.${key}`,
      message: `Filter config ${key} must be null or a non-empty string.`,
    });
  }
}

function validateFilterStringEnum(
  filter: SceneSourceFilter,
  filterPath: string,
  key: string,
  allowed: string[],
  issues: SceneValidationIssue[],
) {
  const value = filter.config?.[key];
  if (typeof value !== "string" || !value.trim()) {
    issues.push({
      path: `${filterPath}.config.${key}`,
      message: `Filter config ${key} is required.`,
    });
    return;
  }
  if (!allowed.includes(value)) {
    issues.push({
      path: `${filterPath}.config.${key}`,
      message: `Filter config ${key} must be one of: ${allowed.join(", ")}.`,
    });
  }
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
