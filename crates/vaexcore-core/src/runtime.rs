use std::collections::HashSet;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::{
    build_audio_mixer_plan, build_capture_frame_plan, build_compositor_graph,
    build_compositor_render_plan, build_scene_transition_preview_plan, checksum_software_pixels,
    evaluate_compositor_frame, render_software_compositor_frame, stinger_video_input_frame,
    validate_compositor_render_plan, validate_scene_collection, AudioMixBus, AudioMixBusKind,
    AudioMixSourceStatus, CaptureFrameBindingStatus, CaptureFrameFormat, CaptureFrameMediaKind,
    CaptureSourceKind, CompositorFrameClock, CompositorFrameFormat, CompositorNodeStatus,
    CompositorRenderPlan, CompositorRenderTarget, CompositorRenderTargetKind,
    CompositorRenderedFrame, CompositorRendererKind, CompositorScaleMode, Scene, SceneCollection,
    SceneSourceKind, SceneTransition, SceneTransitionEasing, SceneTransitionKind,
    SceneTransitionPreviewPlan, SoftwareCompositorAssetMetadata, SoftwareCompositorAssetStatus,
    SoftwareCompositorBrowserStatus, SoftwareCompositorCaptureStatus,
    SoftwareCompositorFilterStatus, SoftwareCompositorFrame, SoftwareCompositorInputFrame,
    SoftwareCompositorMediaPlaybackState, SoftwareCompositorRenderResult,
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SceneRuntimeContractValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneRuntimeCommandKind {
    ActivateScene,
    UpdateRuntimeState,
    RequestPreviewFrame,
    RequestProgramPreviewFrame,
    ValidateRuntimeGraph,
    ExecuteTransition,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneRuntimeCommand {
    pub version: u32,
    pub command_id: String,
    pub kind: SceneRuntimeCommandKind,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneRuntimeStatus {
    Idle,
    Activating,
    Active,
    Transitioning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DesignerRuntimeSessionState {
    Idle,
    Running,
    Paused,
    Degraded,
    Blocked,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DesignerRuntimeReadinessState {
    Ready,
    Degraded,
    Blocked,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DesignerRuntimeSourceSession {
    pub source_id: String,
    pub source_name: String,
    pub source_kind: SceneSourceKind,
    pub runtime_session_id: String,
    pub session_state: DesignerRuntimeSessionState,
    pub last_frame_at: chrono::DateTime<chrono::Utc>,
    pub stale_frame_ms: u64,
    pub restart_count: u64,
    pub dropped_frames: u64,
    pub provider_status: String,
    pub readiness_state: DesignerRuntimeReadinessState,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DesignerRuntimeSessionSnapshot {
    pub version: u32,
    pub runtime_session_id: String,
    pub target: String,
    pub scene_id: String,
    pub scene_name: String,
    pub frame_index: u64,
    pub target_framerate: u32,
    pub session_state: DesignerRuntimeSessionState,
    pub readiness_state: DesignerRuntimeReadinessState,
    pub provider_status: String,
    pub last_frame_at: chrono::DateTime<chrono::Utc>,
    pub stale_frame_ms: u64,
    pub restart_count: u64,
    pub dropped_frames: u64,
    pub sources: Vec<DesignerRuntimeSourceSession>,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DesignerRuntimeSessionControlRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DesignerRuntimeSessionControlResponse {
    pub changed: bool,
    pub action: String,
    pub detail: String,
    pub snapshot: DesignerRuntimeSessionSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DesignerReadinessReportItem {
    pub id: String,
    pub label: String,
    pub state: DesignerRuntimeReadinessState,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneOutputReadyDiagnostic {
    pub version: u32,
    pub ready: bool,
    pub state: DesignerRuntimeReadinessState,
    pub active_scene_id: String,
    pub active_scene_name: String,
    pub program_preview_frame_ready: bool,
    pub compositor_render_plan_ready: bool,
    pub output_preflight_ready: bool,
    pub media_pipeline_ready: bool,
    pub detail: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DesignerReadinessReport {
    pub version: u32,
    pub collection_id: String,
    pub active_scene_id: String,
    pub active_scene_name: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub overall: DesignerRuntimeReadinessState,
    pub items: Vec<DesignerReadinessReportItem>,
    pub output_ready: SceneOutputReadyDiagnostic,
    pub windows_handoff: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneRuntimeSnapshot {
    pub version: u32,
    pub collection_id: String,
    pub collection_name: String,
    pub active_scene_id: String,
    pub active_scene_name: String,
    pub active_transition_id: String,
    pub active_transition_name: String,
    pub status: SceneRuntimeStatus,
    pub preview_enabled: bool,
    pub metadata: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SceneRuntimeStatePatch {
    pub active_scene_id: Option<String>,
    pub active_transition_id: Option<String>,
    pub status: Option<SceneRuntimeStatus>,
    pub preview_enabled: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneRuntimeStateUpdateRequest {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub patch: SceneRuntimeStatePatch,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneRuntimeStateUpdateResponse {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub active_scene_id: String,
    pub active_transition_id: String,
    pub status: SceneRuntimeStatus,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneActivationRequest {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub target_scene_id: String,
    pub transition_id: Option<String>,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneActivationStatus {
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneActivationResponse {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub previous_scene_id: Option<String>,
    pub active_scene_id: String,
    pub transition_id: Option<String>,
    pub status: SceneActivationStatus,
    pub activated_at: chrono::DateTime<chrono::Utc>,
    pub runtime: SceneRuntimeSnapshot,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewFrameEncoding {
    None,
    DataUrl,
    Base64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PreviewFrameRequest {
    pub version: u32,
    pub request_id: String,
    pub scene_id: String,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub frame_format: CompositorFrameFormat,
    pub scale_mode: CompositorScaleMode,
    pub encoding: PreviewFrameEncoding,
    pub include_debug_overlay: bool,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PreviewFrameResponse {
    pub version: u32,
    pub request_id: String,
    pub scene_id: String,
    pub scene_name: String,
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub frame_format: CompositorFrameFormat,
    pub encoding: PreviewFrameEncoding,
    pub image_data: Option<String>,
    pub checksum: Option<String>,
    pub render_time_ms: f64,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub runtime_session_id: String,
    pub session_state: DesignerRuntimeSessionState,
    pub last_frame_at: chrono::DateTime<chrono::Utc>,
    pub stale_frame_ms: u64,
    pub restart_count: u64,
    pub dropped_frames: u64,
    pub provider_status: String,
    pub readiness_state: DesignerRuntimeReadinessState,
    pub runtime_session: DesignerRuntimeSessionSnapshot,
    pub rendered_frame: Option<CompositorRenderedFrame>,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProgramPreviewFrameRequest {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub frame_format: CompositorFrameFormat,
    pub scale_mode: CompositorScaleMode,
    pub encoding: PreviewFrameEncoding,
    pub include_debug_overlay: bool,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProgramPreviewFrameResponse {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub scene_id: String,
    pub scene_name: String,
    pub active_transition_id: String,
    pub active_transition_name: String,
    pub program_target_id: String,
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub frame_format: CompositorFrameFormat,
    pub encoding: PreviewFrameEncoding,
    pub image_data: Option<String>,
    pub checksum: Option<String>,
    pub render_time_ms: f64,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub runtime_session_id: String,
    pub session_state: DesignerRuntimeSessionState,
    pub last_frame_at: chrono::DateTime<chrono::Utc>,
    pub stale_frame_ms: u64,
    pub restart_count: u64,
    pub dropped_frames: u64,
    pub provider_status: String,
    pub readiness_state: DesignerRuntimeReadinessState,
    pub runtime_session: DesignerRuntimeSessionSnapshot,
    pub rendered_frame: Option<CompositorRenderedFrame>,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorRenderRequest {
    pub version: u32,
    pub request_id: String,
    pub renderer: CompositorRendererKind,
    pub plan: CompositorRenderPlan,
    pub clock: CompositorFrameClock,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorRenderTargetResult {
    pub target_id: String,
    pub target_kind: crate::CompositorRenderTargetKind,
    pub width: u32,
    pub height: u32,
    pub frame_format: CompositorFrameFormat,
    pub checksum: Option<String>,
    pub byte_length: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorRenderResponse {
    pub version: u32,
    pub request_id: String,
    pub renderer: CompositorRendererKind,
    pub scene_id: String,
    pub scene_name: String,
    pub frame: CompositorRenderedFrame,
    pub target_results: Vec<CompositorRenderTargetResult>,
    pub render_time_ms: f64,
    pub rendered_at: chrono::DateTime<chrono::Utc>,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RuntimeCaptureSourceBinding {
    pub scene_source_id: String,
    pub scene_source_name: String,
    pub scene_source_kind: SceneSourceKind,
    pub capture_source_id: Option<String>,
    pub capture_kind: CaptureSourceKind,
    pub media_kind: CaptureFrameMediaKind,
    pub frame_format: CaptureFrameFormat,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub framerate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub required: bool,
    pub status: CaptureFrameBindingStatus,
    pub status_detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RuntimeCaptureSourceBindingContract {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub bindings: Vec<RuntimeCaptureSourceBinding>,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RuntimeAudioSourceBinding {
    pub scene_source_id: String,
    pub scene_source_name: String,
    pub capture_source_id: Option<String>,
    pub capture_kind: CaptureSourceKind,
    pub bus_ids: Vec<String>,
    pub gain_db: f64,
    pub muted: bool,
    pub monitor_enabled: bool,
    pub meter_enabled: bool,
    pub sync_offset_ms: i32,
    pub status: AudioMixSourceStatus,
    pub status_detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RuntimeAudioSourceBindingContract {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bindings: Vec<RuntimeAudioSourceBinding>,
    pub buses: Vec<AudioMixBus>,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneRuntimeBindingsSnapshot {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub capture: RuntimeCaptureSourceBindingContract,
    pub audio: RuntimeAudioSourceBindingContract,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransitionExecutionRequest {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub transition_id: String,
    pub from_scene_id: String,
    pub to_scene_id: String,
    pub framerate: u32,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransitionExecutionResponse {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub transition_id: String,
    pub from_scene_id: String,
    pub to_scene_id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub preview_plan: SceneTransitionPreviewPlan,
    pub validation: SceneRuntimeContractValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransitionPreviewFrameRequest {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub transition_id: String,
    pub from_scene_id: String,
    pub to_scene_id: String,
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub frame_format: CompositorFrameFormat,
    pub scale_mode: CompositorScaleMode,
    pub encoding: PreviewFrameEncoding,
    pub include_debug_overlay: bool,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StingerTransitionRuntimeStatus {
    Rendered,
    NoAsset,
    MissingFile,
    UnsupportedExtension,
    FfmpegUnavailable,
    DecodeFailed,
    NotStinger,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StingerTransitionRuntimeMetadata {
    pub uri: String,
    pub status: StingerTransitionRuntimeStatus,
    pub status_detail: String,
    pub trigger_time_ms: u32,
    pub triggered: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_unix_ms: Option<u64>,
    #[serde(default)]
    pub cache_hit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampled_frame_time_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoder_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_timeline_state: Option<SoftwareCompositorMediaPlaybackState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_position_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_base_position_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_on_scene_activate: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransitionPreviewFrameResponse {
    pub version: u32,
    pub request_id: String,
    pub collection_id: String,
    pub transition_id: String,
    pub transition_kind: SceneTransitionKind,
    pub from_scene_id: String,
    pub from_scene_name: String,
    pub to_scene_id: String,
    pub to_scene_name: String,
    pub frame_index: u64,
    pub elapsed_ms: u32,
    pub linear_progress: f64,
    pub eased_progress: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_time_ms: Option<u32>,
    pub triggered: bool,
    pub width: u32,
    pub height: u32,
    pub frame_format: CompositorFrameFormat,
    pub encoding: PreviewFrameEncoding,
    pub image_data: Option<String>,
    pub checksum: Option<String>,
    pub render_time_ms: f64,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub runtime_session_id: String,
    pub session_state: DesignerRuntimeSessionState,
    pub last_frame_at: chrono::DateTime<chrono::Utc>,
    pub stale_frame_ms: u64,
    pub restart_count: u64,
    pub dropped_frames: u64,
    pub provider_status: String,
    pub readiness_state: DesignerRuntimeReadinessState,
    pub runtime_session: DesignerRuntimeSessionSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stinger: Option<StingerTransitionRuntimeMetadata>,
    pub validation: SceneRuntimeContractValidation,
}

pub fn scene_runtime_snapshot(collection: &SceneCollection) -> SceneRuntimeSnapshot {
    scene_runtime_snapshot_with_options(
        collection,
        SceneRuntimeStatus::Active,
        true,
        serde_json::json!({ "source": "saved_scene_collection" }),
    )
}

pub fn scene_runtime_snapshot_with_options(
    collection: &SceneCollection,
    status: SceneRuntimeStatus,
    preview_enabled: bool,
    metadata: serde_json::Value,
) -> SceneRuntimeSnapshot {
    let active_scene = collection
        .active_scene()
        .or_else(|| collection.scenes.first());
    let active_transition = collection
        .transitions
        .iter()
        .find(|transition| transition.id == collection.active_transition_id)
        .or_else(|| collection.transitions.first());
    let validation = validate_scene_runtime_collection(collection);

    SceneRuntimeSnapshot {
        version: 1,
        collection_id: collection.id.clone(),
        collection_name: collection.name.clone(),
        active_scene_id: active_scene
            .map(|scene| scene.id.clone())
            .unwrap_or_default(),
        active_scene_name: active_scene
            .map(|scene| scene.name.clone())
            .unwrap_or_default(),
        active_transition_id: active_transition
            .map(|transition| transition.id.clone())
            .unwrap_or_default(),
        active_transition_name: active_transition
            .map(|transition| transition.name.clone())
            .unwrap_or_default(),
        status,
        preview_enabled,
        metadata,
        updated_at: crate::now_utc(),
        validation,
    }
}

pub fn validate_scene_activation_request(
    request: &SceneActivationRequest,
    collection: &SceneCollection,
) -> SceneRuntimeContractValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    validate_runtime_envelope(
        request.version,
        &request.request_id,
        "scene activation request",
        &mut errors,
    );
    if request.collection_id.trim().is_empty() {
        errors.push("scene activation collection id is required".to_string());
    }
    if request.collection_id != collection.id {
        errors.push("scene activation collection id does not match saved collection".to_string());
    }
    if request.target_scene_id.trim().is_empty() {
        errors.push("scene activation target scene id is required".to_string());
    }
    if !collection
        .scenes
        .iter()
        .any(|scene| scene.id == request.target_scene_id)
    {
        errors.push(format!(
            "scene activation target scene \"{}\" does not exist",
            request.target_scene_id
        ));
    }
    if let Some(transition_id) = &request.transition_id {
        if transition_id.trim().is_empty() {
            errors.push("scene activation transition id cannot be blank".to_string());
        } else if !collection
            .transitions
            .iter()
            .any(|transition| transition.id == *transition_id)
        {
            errors.push(format!(
                "scene activation transition \"{}\" does not exist",
                transition_id
            ));
        }
    }

    let collection_validation = validate_scene_collection(collection);
    if !collection_validation.ok {
        warnings.push(format!(
            "scene collection has {} validation issue(s)",
            collection_validation.issues.len()
        ));
    }

    runtime_validation(errors, warnings)
}

pub fn create_scene_activation_response(
    request: &SceneActivationRequest,
    collection: &SceneCollection,
    previous_scene_id: Option<String>,
) -> SceneActivationResponse {
    let validation = validate_scene_activation_request(request, collection);
    let status = if validation.ready {
        SceneActivationStatus::Accepted
    } else {
        SceneActivationStatus::Rejected
    };
    let runtime = scene_runtime_snapshot_with_options(
        collection,
        if validation.ready {
            SceneRuntimeStatus::Active
        } else {
            SceneRuntimeStatus::Error
        },
        true,
        serde_json::json!({ "request_id": request.request_id }),
    );

    SceneActivationResponse {
        version: 1,
        request_id: request.request_id.clone(),
        collection_id: request.collection_id.clone(),
        previous_scene_id,
        active_scene_id: request.target_scene_id.clone(),
        transition_id: request.transition_id.clone(),
        status,
        activated_at: crate::now_utc(),
        runtime,
        validation,
    }
}

pub fn validate_scene_runtime_state_update_request(
    request: &SceneRuntimeStateUpdateRequest,
    collection: &SceneCollection,
) -> SceneRuntimeContractValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    validate_runtime_envelope(
        request.version,
        &request.request_id,
        "scene runtime state update request",
        &mut errors,
    );
    if request.collection_id.trim().is_empty() {
        errors.push("scene runtime state update collection id is required".to_string());
    }
    if request.collection_id != collection.id {
        errors.push(
            "scene runtime state update collection id does not match saved collection".to_string(),
        );
    }
    if let Some(scene_id) = &request.patch.active_scene_id {
        if scene_id.trim().is_empty() {
            errors.push("scene runtime state active scene id cannot be blank".to_string());
        } else if !collection.scenes.iter().any(|scene| scene.id == *scene_id) {
            errors.push(format!(
                "scene runtime state active scene \"{}\" does not exist",
                scene_id
            ));
        }
    }
    if let Some(transition_id) = &request.patch.active_transition_id {
        if transition_id.trim().is_empty() {
            errors.push("scene runtime state active transition id cannot be blank".to_string());
        } else if !collection
            .transitions
            .iter()
            .any(|transition| transition.id == *transition_id)
        {
            errors.push(format!(
                "scene runtime state active transition \"{}\" does not exist",
                transition_id
            ));
        }
    }
    if matches!(request.patch.status, Some(SceneRuntimeStatus::Error)) {
        warnings.push("scene runtime state patch explicitly marks runtime as error".to_string());
    }

    runtime_validation(errors, warnings)
}

pub fn create_scene_runtime_state_update_response(
    request: &SceneRuntimeStateUpdateRequest,
    collection: &SceneCollection,
) -> SceneRuntimeStateUpdateResponse {
    let validation = validate_scene_runtime_state_update_request(request, collection);
    SceneRuntimeStateUpdateResponse {
        version: 1,
        request_id: request.request_id.clone(),
        collection_id: request.collection_id.clone(),
        active_scene_id: request
            .patch
            .active_scene_id
            .clone()
            .unwrap_or_else(|| collection.active_scene_id.clone()),
        active_transition_id: request
            .patch
            .active_transition_id
            .clone()
            .unwrap_or_else(|| collection.active_transition_id.clone()),
        status: request
            .patch
            .status
            .clone()
            .unwrap_or(SceneRuntimeStatus::Active),
        updated_at: crate::now_utc(),
        validation,
    }
}

pub fn validate_preview_frame_request(
    request: &PreviewFrameRequest,
    collection: &SceneCollection,
) -> SceneRuntimeContractValidation {
    let mut errors = Vec::new();

    validate_runtime_envelope(
        request.version,
        &request.request_id,
        "preview frame request",
        &mut errors,
    );
    if request.scene_id.trim().is_empty() {
        errors.push("preview frame scene id is required".to_string());
    }
    if !collection
        .scenes
        .iter()
        .any(|scene| scene.id == request.scene_id)
    {
        errors.push(format!(
            "preview frame scene \"{}\" does not exist",
            request.scene_id
        ));
    }
    if request.width == 0 || request.height == 0 {
        errors.push("preview frame dimensions must be greater than zero".to_string());
    }
    if request.width > 7680 || request.height > 4320 {
        errors.push("preview frame dimensions must be 8K or smaller".to_string());
    }
    if request.framerate == 0 {
        errors.push("preview frame framerate must be greater than zero".to_string());
    }

    runtime_validation(errors, Vec::new())
}

pub fn validate_program_preview_frame_request(
    request: &ProgramPreviewFrameRequest,
    collection: &SceneCollection,
) -> SceneRuntimeContractValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    validate_runtime_envelope(
        request.version,
        &request.request_id,
        "program preview frame request",
        &mut errors,
    );
    if request.collection_id.trim().is_empty() {
        errors.push("program preview frame collection id is required".to_string());
    }
    if request.collection_id != collection.id {
        errors.push(
            "program preview frame collection id does not match saved collection".to_string(),
        );
    }
    if collection.active_scene().is_none() {
        errors.push("program preview frame requires an active scene".to_string());
    }
    if request.width == 0 || request.height == 0 {
        errors.push("program preview frame dimensions must be greater than zero".to_string());
    }
    if request.width > 7680 || request.height > 4320 {
        errors.push("program preview frame dimensions must be 8K or smaller".to_string());
    }
    if request.framerate == 0 {
        errors.push("program preview frame framerate must be greater than zero".to_string());
    }

    let collection_validation = validate_scene_collection(collection);
    if !collection_validation.ok {
        warnings.push(format!(
            "scene collection has {} validation issue(s)",
            collection_validation.issues.len()
        ));
    }

    runtime_validation(errors, warnings)
}

pub fn create_preview_frame_response(
    request: &PreviewFrameRequest,
    collection: &SceneCollection,
    frame_index: u64,
) -> PreviewFrameResponse {
    let started_at = Instant::now();
    let mut validation = validate_preview_frame_request(request, collection);
    let scene = collection
        .scenes
        .iter()
        .find(|scene| scene.id == request.scene_id)
        .or_else(|| collection.active_scene())
        .or_else(|| collection.scenes.first());
    let render_result = scene.map(|scene| preview_render_result(scene, request, frame_index));
    let rendered_frame = render_result.as_ref().map(|result| result.frame.clone());

    if let Some(frame) = &rendered_frame {
        validation
            .warnings
            .extend(frame.validation.warnings.clone());
        validation.errors.extend(frame.validation.errors.clone());
        validation.ready = validation.errors.is_empty();
    }

    let pixel_frame = render_result
        .as_ref()
        .and_then(|result| result.pixel_frames.first());
    let checksum = pixel_frame
        .map(|frame| format!("software:{:016x}", frame.checksum))
        .or_else(|| {
            rendered_frame
                .as_ref()
                .map(|frame| preview_frame_checksum(frame, request))
        });
    let image_data = pixel_frame.and_then(|frame| preview_image_data(frame, &request.encoding));
    let generated_at = crate::now_utc();
    let runtime_session = designer_runtime_session_snapshot(
        "runtime_preview",
        (
            request.scene_id.as_str(),
            scene.map(|scene| scene.name.as_str()).unwrap_or_default(),
        ),
        frame_index,
        request.framerate,
        generated_at,
        render_result.as_ref(),
        &validation,
    );

    PreviewFrameResponse {
        version: 1,
        request_id: request.request_id.clone(),
        scene_id: request.scene_id.clone(),
        scene_name: scene.map(|scene| scene.name.clone()).unwrap_or_default(),
        frame_index,
        width: request.width,
        height: request.height,
        frame_format: request.frame_format.clone(),
        encoding: request.encoding.clone(),
        image_data,
        checksum,
        render_time_ms: started_at.elapsed().as_secs_f64() * 1000.0,
        generated_at,
        runtime_session_id: runtime_session.runtime_session_id.clone(),
        session_state: runtime_session.session_state.clone(),
        last_frame_at: runtime_session.last_frame_at,
        stale_frame_ms: runtime_session.stale_frame_ms,
        restart_count: runtime_session.restart_count,
        dropped_frames: runtime_session.dropped_frames,
        provider_status: runtime_session.provider_status.clone(),
        readiness_state: runtime_session.readiness_state.clone(),
        runtime_session,
        rendered_frame,
        validation,
    }
}

pub fn create_program_preview_frame_response(
    request: &ProgramPreviewFrameRequest,
    collection: &SceneCollection,
    frame_index: u64,
) -> ProgramPreviewFrameResponse {
    let started_at = Instant::now();
    let mut validation = validate_program_preview_frame_request(request, collection);
    let scene = collection
        .active_scene()
        .or_else(|| collection.scenes.first());
    let transition = collection
        .transitions
        .iter()
        .find(|transition| transition.id == collection.active_transition_id)
        .or_else(|| collection.transitions.first());
    let render_result = scene.map(|scene| {
        scene_render_result(
            scene,
            SceneRenderTargetOptions {
                target_id: "target-program-preview",
                target_name: "Program Preview",
                target_kind: CompositorRenderTargetKind::Program,
                width: request.width,
                height: request.height,
                framerate: request.framerate,
                frame_format: request.frame_format.clone(),
                scale_mode: request.scale_mode.clone(),
            },
            frame_index,
        )
    });
    let rendered_frame = render_result.as_ref().map(|result| result.frame.clone());

    if let Some(frame) = &rendered_frame {
        validation
            .warnings
            .extend(frame.validation.warnings.clone());
        validation.errors.extend(frame.validation.errors.clone());
        validation.ready = validation.errors.is_empty();
    }

    let pixel_frame = render_result
        .as_ref()
        .and_then(|result| result.pixel_frames.first());
    let checksum = pixel_frame
        .map(|frame| format!("software-program:{:016x}", frame.checksum))
        .or_else(|| {
            rendered_frame
                .as_ref()
                .map(|frame| rendered_frame_checksum(frame, request.width, request.height))
        });
    let image_data = pixel_frame.and_then(|frame| preview_image_data(frame, &request.encoding));
    let generated_at = crate::now_utc();
    let runtime_session = designer_runtime_session_snapshot(
        "program_preview",
        (
            scene.map(|scene| scene.id.as_str()).unwrap_or_default(),
            scene.map(|scene| scene.name.as_str()).unwrap_or_default(),
        ),
        frame_index,
        request.framerate,
        generated_at,
        render_result.as_ref(),
        &validation,
    );

    ProgramPreviewFrameResponse {
        version: 1,
        request_id: request.request_id.clone(),
        collection_id: request.collection_id.clone(),
        scene_id: scene.map(|scene| scene.id.clone()).unwrap_or_default(),
        scene_name: scene.map(|scene| scene.name.clone()).unwrap_or_default(),
        active_transition_id: transition
            .map(|transition| transition.id.clone())
            .unwrap_or_default(),
        active_transition_name: transition
            .map(|transition| transition.name.clone())
            .unwrap_or_default(),
        program_target_id: "target-program-preview".to_string(),
        frame_index,
        width: request.width,
        height: request.height,
        framerate: request.framerate,
        frame_format: request.frame_format.clone(),
        encoding: request.encoding.clone(),
        image_data,
        checksum,
        render_time_ms: started_at.elapsed().as_secs_f64() * 1000.0,
        generated_at,
        runtime_session_id: runtime_session.runtime_session_id.clone(),
        session_state: runtime_session.session_state.clone(),
        last_frame_at: runtime_session.last_frame_at,
        stale_frame_ms: runtime_session.stale_frame_ms,
        restart_count: runtime_session.restart_count,
        dropped_frames: runtime_session.dropped_frames,
        provider_status: runtime_session.provider_status.clone(),
        readiness_state: runtime_session.readiness_state.clone(),
        runtime_session,
        rendered_frame,
        validation,
    }
}

pub fn validate_compositor_render_request(
    request: &CompositorRenderRequest,
) -> SceneRuntimeContractValidation {
    let mut validation =
        compositor_validation_to_runtime(validate_compositor_render_plan(&request.plan));
    validate_runtime_envelope(
        request.version,
        &request.request_id,
        "compositor render request",
        &mut validation.errors,
    );
    if request.clock.framerate == 0 {
        validation
            .errors
            .push("compositor render clock framerate must be greater than zero".to_string());
    }
    if request.clock.duration_nanos == 0 {
        validation
            .errors
            .push("compositor render clock duration nanos must be greater than zero".to_string());
    }
    validation.ready = validation.errors.is_empty();
    validation
}

pub fn create_compositor_render_response(
    request: &CompositorRenderRequest,
) -> CompositorRenderResponse {
    let mut frame = evaluate_compositor_frame(&request.plan, request.clock.frame_index);
    frame.renderer = request.renderer.clone();
    let validation = validate_compositor_render_request(request);
    let target_results = frame
        .targets
        .iter()
        .map(|target| CompositorRenderTargetResult {
            target_id: target.target_id.clone(),
            target_kind: target.target_kind.clone(),
            width: target.width,
            height: target.height,
            frame_format: target.frame_format.clone(),
            checksum: Some(format!(
                "contract:{}:{}:{}",
                target.target_id,
                frame.clock.frame_index,
                target.nodes.len()
            )),
            byte_length: None,
        })
        .collect();

    CompositorRenderResponse {
        version: 1,
        request_id: request.request_id.clone(),
        renderer: frame.renderer.clone(),
        scene_id: frame.scene_id.clone(),
        scene_name: frame.scene_name.clone(),
        frame,
        target_results,
        render_time_ms: 0.0,
        rendered_at: crate::now_utc(),
        validation,
    }
}

pub fn build_runtime_capture_source_binding_contract(
    scene: &Scene,
) -> RuntimeCaptureSourceBindingContract {
    let plan = build_capture_frame_plan(scene);
    let mut contract = RuntimeCaptureSourceBindingContract {
        version: 1,
        scene_id: scene.id.clone(),
        scene_name: scene.name.clone(),
        bindings: plan
            .bindings
            .iter()
            .map(|binding| {
                let source = scene
                    .sources
                    .iter()
                    .find(|source| source.id == binding.scene_source_id);
                RuntimeCaptureSourceBinding {
                    scene_source_id: binding.scene_source_id.clone(),
                    scene_source_name: binding.scene_source_name.clone(),
                    scene_source_kind: source
                        .map(|source| source.kind.clone())
                        .unwrap_or(SceneSourceKind::Display),
                    capture_source_id: binding.capture_source_id.clone(),
                    capture_kind: binding.capture_kind.clone(),
                    media_kind: binding.media_kind.clone(),
                    frame_format: binding.format.clone(),
                    width: binding.width,
                    height: binding.height,
                    framerate: binding.framerate,
                    sample_rate: binding.sample_rate,
                    channels: binding.channels,
                    required: source.map(|source| source.visible).unwrap_or(true),
                    status: binding.status.clone(),
                    status_detail: binding.status_detail.clone(),
                }
            })
            .collect(),
        validation: runtime_validation(Vec::new(), Vec::new()),
    };
    contract.validation = validate_runtime_capture_source_binding_contract(&contract);
    contract
}

pub fn validate_runtime_capture_source_binding_contract(
    contract: &RuntimeCaptureSourceBindingContract,
) -> SceneRuntimeContractValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut source_ids = HashSet::new();

    if contract.version == 0 {
        errors
            .push("runtime capture binding contract version must be greater than zero".to_string());
    }
    if contract.scene_id.trim().is_empty() {
        errors.push("runtime capture binding scene id is required".to_string());
    }
    if contract.scene_name.trim().is_empty() {
        errors.push("runtime capture binding scene name is required".to_string());
    }
    if contract.bindings.is_empty() {
        warnings.push("runtime capture binding contract has no bindings".to_string());
    }

    for binding in &contract.bindings {
        if !source_ids.insert(binding.scene_source_id.as_str()) {
            errors.push(format!(
                "duplicate runtime capture binding \"{}\"",
                binding.scene_source_id
            ));
        }
        if binding.scene_source_id.trim().is_empty() {
            errors.push("runtime capture binding source id is required".to_string());
        }
        if binding.scene_source_name.trim().is_empty() {
            errors.push(format!(
                "runtime capture binding \"{}\" name is required",
                binding.scene_source_id
            ));
        }
        if binding.required && binding.status != CaptureFrameBindingStatus::Ready {
            warnings.push(format!(
                "{} capture is {:?}: {}",
                binding.scene_source_name, binding.status, binding.status_detail
            ));
        }
        match binding.media_kind {
            CaptureFrameMediaKind::Video => {
                validate_optional_positive(binding.width, &binding.scene_source_id, &mut errors);
                validate_optional_positive(binding.height, &binding.scene_source_id, &mut errors);
                validate_optional_positive(
                    binding.framerate,
                    &binding.scene_source_id,
                    &mut errors,
                );
            }
            CaptureFrameMediaKind::Audio => {
                validate_optional_positive(
                    binding.sample_rate,
                    &binding.scene_source_id,
                    &mut errors,
                );
                if matches!(binding.channels, Some(0)) {
                    errors.push(format!(
                        "{} channels must be greater than zero",
                        binding.scene_source_id
                    ));
                }
            }
        }
    }

    runtime_validation(errors, warnings)
}

pub fn build_runtime_audio_source_binding_contract(
    scene: &Scene,
) -> RuntimeAudioSourceBindingContract {
    let plan = build_audio_mixer_plan(scene);
    let bus_ids = plan
        .buses
        .iter()
        .map(|bus| bus.id.clone())
        .collect::<Vec<_>>();
    let mut contract = RuntimeAudioSourceBindingContract {
        version: 1,
        scene_id: scene.id.clone(),
        scene_name: scene.name.clone(),
        sample_rate: plan.sample_rate,
        channels: plan.channels,
        bindings: plan
            .sources
            .iter()
            .map(|source| RuntimeAudioSourceBinding {
                scene_source_id: source.scene_source_id.clone(),
                scene_source_name: source.name.clone(),
                capture_source_id: source.capture_source_id.clone(),
                capture_kind: source.capture_kind.clone(),
                bus_ids: bus_ids.clone(),
                gain_db: source.gain_db,
                muted: source.muted,
                monitor_enabled: source.monitor_enabled,
                meter_enabled: source.meter_enabled,
                sync_offset_ms: source.sync_offset_ms,
                status: source.status.clone(),
                status_detail: source.status_detail.clone(),
            })
            .collect(),
        buses: plan.buses,
        validation: runtime_validation(Vec::new(), Vec::new()),
    };
    contract.validation = validate_runtime_audio_source_binding_contract(&contract);
    contract
}

pub fn validate_runtime_audio_source_binding_contract(
    contract: &RuntimeAudioSourceBindingContract,
) -> SceneRuntimeContractValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut source_ids = HashSet::new();
    let bus_ids = contract
        .buses
        .iter()
        .map(|bus| bus.id.as_str())
        .collect::<HashSet<_>>();

    if contract.version == 0 {
        errors.push("runtime audio binding contract version must be greater than zero".to_string());
    }
    if contract.scene_id.trim().is_empty() {
        errors.push("runtime audio binding scene id is required".to_string());
    }
    if contract.sample_rate == 0 {
        errors.push("runtime audio sample rate must be greater than zero".to_string());
    }
    if contract.channels == 0 {
        errors.push("runtime audio channels must be greater than zero".to_string());
    }
    if !contract
        .buses
        .iter()
        .any(|bus| bus.kind == AudioMixBusKind::Master)
    {
        errors.push("runtime audio binding contract requires a master bus".to_string());
    }
    if contract.bindings.is_empty() {
        warnings.push("runtime audio binding contract has no audio bindings".to_string());
    }

    for binding in &contract.bindings {
        if !source_ids.insert(binding.scene_source_id.as_str()) {
            errors.push(format!(
                "duplicate runtime audio binding \"{}\"",
                binding.scene_source_id
            ));
        }
        if binding.scene_source_id.trim().is_empty() {
            errors.push("runtime audio binding source id is required".to_string());
        }
        for bus_id in &binding.bus_ids {
            if !bus_ids.contains(bus_id.as_str()) {
                errors.push(format!(
                    "{} references missing bus \"{}\"",
                    binding.scene_source_name, bus_id
                ));
            }
        }
        if binding.status != AudioMixSourceStatus::Ready {
            warnings.push(format!(
                "{} audio is {:?}: {}",
                binding.scene_source_name, binding.status, binding.status_detail
            ));
        }
    }

    runtime_validation(errors, warnings)
}

pub fn build_scene_runtime_bindings_snapshot(scene: &Scene) -> SceneRuntimeBindingsSnapshot {
    SceneRuntimeBindingsSnapshot {
        version: 1,
        scene_id: scene.id.clone(),
        scene_name: scene.name.clone(),
        capture: build_runtime_capture_source_binding_contract(scene),
        audio: build_runtime_audio_source_binding_contract(scene),
        generated_at: crate::now_utc(),
    }
}

pub fn validate_transition_execution_request(
    request: &TransitionExecutionRequest,
    collection: &SceneCollection,
) -> SceneRuntimeContractValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    validate_runtime_envelope(
        request.version,
        &request.request_id,
        "transition execution request",
        &mut errors,
    );
    if request.collection_id != collection.id {
        errors
            .push("transition execution collection id does not match saved collection".to_string());
    }
    if !collection
        .transitions
        .iter()
        .any(|transition| transition.id == request.transition_id)
    {
        errors.push(format!(
            "transition \"{}\" does not exist",
            request.transition_id
        ));
    }
    if !collection
        .scenes
        .iter()
        .any(|scene| scene.id == request.from_scene_id)
    {
        errors.push(format!(
            "transition from scene \"{}\" does not exist",
            request.from_scene_id
        ));
    }
    if !collection
        .scenes
        .iter()
        .any(|scene| scene.id == request.to_scene_id)
    {
        errors.push(format!(
            "transition to scene \"{}\" does not exist",
            request.to_scene_id
        ));
    }
    if request.framerate == 0 {
        errors.push("transition execution framerate must be greater than zero".to_string());
    }
    if request.from_scene_id == request.to_scene_id {
        warnings.push("transition execution uses the same from and to scene".to_string());
    }

    runtime_validation(errors, warnings)
}

pub fn create_transition_execution_response(
    request: &TransitionExecutionRequest,
    collection: &SceneCollection,
) -> TransitionExecutionResponse {
    let mut preview_collection = collection.clone();
    preview_collection.active_transition_id = request.transition_id.clone();
    let preview_plan = build_scene_transition_preview_plan(
        &preview_collection,
        Some(request.from_scene_id.as_str()),
        Some(request.to_scene_id.as_str()),
        request.framerate,
    );
    let mut validation = validate_transition_execution_request(request, collection);
    validation
        .warnings
        .extend(preview_plan.validation.warnings.clone());
    validation
        .errors
        .extend(preview_plan.validation.errors.clone());
    validation.ready = validation.errors.is_empty();

    TransitionExecutionResponse {
        version: 1,
        request_id: request.request_id.clone(),
        collection_id: request.collection_id.clone(),
        transition_id: request.transition_id.clone(),
        from_scene_id: request.from_scene_id.clone(),
        to_scene_id: request.to_scene_id.clone(),
        started_at: crate::now_utc(),
        preview_plan,
        validation,
    }
}

pub fn validate_transition_preview_frame_request(
    request: &TransitionPreviewFrameRequest,
    collection: &SceneCollection,
) -> SceneRuntimeContractValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    validate_runtime_envelope(
        request.version,
        &request.request_id,
        "transition preview frame request",
        &mut errors,
    );
    if request.collection_id != collection.id {
        errors.push(
            "transition preview frame collection id does not match saved collection".to_string(),
        );
    }
    let transition = collection
        .transitions
        .iter()
        .find(|transition| transition.id == request.transition_id);
    if transition.is_none() {
        errors.push(format!(
            "transition preview frame transition \"{}\" does not exist",
            request.transition_id
        ));
    }
    if !collection
        .scenes
        .iter()
        .any(|scene| scene.id == request.from_scene_id)
    {
        errors.push(format!(
            "transition preview frame from scene \"{}\" does not exist",
            request.from_scene_id
        ));
    }
    if !collection
        .scenes
        .iter()
        .any(|scene| scene.id == request.to_scene_id)
    {
        errors.push(format!(
            "transition preview frame to scene \"{}\" does not exist",
            request.to_scene_id
        ));
    }
    if request.width == 0 || request.height == 0 {
        errors.push("transition preview frame dimensions must be greater than zero".to_string());
    }
    if request.width > 7680 || request.height > 4320 {
        errors.push("transition preview frame dimensions must be 8K or smaller".to_string());
    }
    if request.framerate == 0 {
        errors.push("transition preview frame framerate must be greater than zero".to_string());
    }
    if request.from_scene_id == request.to_scene_id {
        warnings.push("transition preview frame uses the same from and to scene".to_string());
    }

    runtime_validation(errors, warnings)
}

pub fn create_transition_preview_frame_response(
    request: &TransitionPreviewFrameRequest,
    collection: &SceneCollection,
) -> TransitionPreviewFrameResponse {
    let started_at = Instant::now();
    let mut validation = validate_transition_preview_frame_request(request, collection);
    let transition = collection
        .transitions
        .iter()
        .find(|transition| transition.id == request.transition_id);
    let from_scene = collection
        .scenes
        .iter()
        .find(|scene| scene.id == request.from_scene_id);
    let to_scene = collection
        .scenes
        .iter()
        .find(|scene| scene.id == request.to_scene_id);
    let transition_kind = transition
        .map(|transition| transition.kind.clone())
        .unwrap_or(SceneTransitionKind::Stinger);
    let frame_count = transition
        .map(|transition| transition_preview_frame_count(transition.duration_ms, request.framerate))
        .unwrap_or(1);
    let frame_index = request.frame_index.min(frame_count.saturating_sub(1));
    let elapsed_ms = transition
        .map(|transition| {
            transition_preview_elapsed_ms(transition.duration_ms, frame_index, request.framerate)
        })
        .unwrap_or(0);
    let linear_progress = transition
        .map(|transition| {
            transition_preview_linear_progress(transition.duration_ms, frame_index, frame_count)
        })
        .unwrap_or(0.0);
    let eased_progress = transition
        .map(|transition| transition_eased_progress(linear_progress, &transition.easing))
        .unwrap_or(linear_progress);
    let trigger_time_ms = transition.and_then(|transition| {
        (transition.kind == SceneTransitionKind::Stinger)
            .then(|| stinger_trigger_time_ms(transition))
    });
    let triggered =
        if transition.is_some_and(|transition| transition.kind == SceneTransitionKind::Stinger) {
            trigger_time_ms.is_some_and(|trigger| elapsed_ms >= trigger)
        } else {
            eased_progress >= 1.0
                || transition.is_some_and(|transition| transition.kind == SceneTransitionKind::Cut)
        };

    let mut rendered_pixel_frame = None;
    let mut stinger_metadata = None;

    if validation.errors.is_empty() {
        if let (Some(transition), Some(from_scene), Some(to_scene)) =
            (transition, from_scene, to_scene)
        {
            let rendered = render_transition_preview_frame(TransitionRenderContext {
                request,
                transition,
                from_scene,
                to_scene,
                frame_index,
                elapsed_ms,
                trigger_time_ms: trigger_time_ms.unwrap_or(0),
                triggered,
                eased_progress,
            });
            validation.warnings.extend(rendered.validation.warnings);
            validation.errors.extend(rendered.validation.errors);
            validation.ready = validation.errors.is_empty();
            stinger_metadata = rendered.stinger;
            rendered_pixel_frame = Some(rendered.frame);
        }
    }

    let image_data = rendered_pixel_frame
        .as_ref()
        .and_then(|frame| preview_image_data(frame, &request.encoding));
    let checksum = rendered_pixel_frame
        .as_ref()
        .map(|frame| format!("software-transition:{:016x}", frame.checksum));
    let generated_at = crate::now_utc();
    let transition_source_session = transition_runtime_source_session(
        transition,
        frame_index,
        request.framerate,
        generated_at,
        &validation,
        stinger_metadata.as_ref(),
    );
    let runtime_session = designer_transition_runtime_session_snapshot(
        request,
        transition_kind.clone(),
        from_scene
            .map(|scene| scene.name.as_str())
            .unwrap_or_default(),
        generated_at,
        transition_source_session,
        &validation,
    );

    TransitionPreviewFrameResponse {
        version: 1,
        request_id: request.request_id.clone(),
        collection_id: request.collection_id.clone(),
        transition_id: request.transition_id.clone(),
        transition_kind,
        from_scene_id: request.from_scene_id.clone(),
        from_scene_name: from_scene
            .map(|scene| scene.name.clone())
            .unwrap_or_default(),
        to_scene_id: request.to_scene_id.clone(),
        to_scene_name: to_scene.map(|scene| scene.name.clone()).unwrap_or_default(),
        frame_index,
        elapsed_ms,
        linear_progress,
        eased_progress,
        trigger_time_ms,
        triggered,
        width: request.width,
        height: request.height,
        frame_format: request.frame_format.clone(),
        encoding: request.encoding.clone(),
        image_data,
        checksum,
        render_time_ms: started_at.elapsed().as_secs_f64() * 1000.0,
        generated_at,
        runtime_session_id: runtime_session.runtime_session_id.clone(),
        session_state: runtime_session.session_state.clone(),
        last_frame_at: runtime_session.last_frame_at,
        stale_frame_ms: runtime_session.stale_frame_ms,
        restart_count: runtime_session.restart_count,
        dropped_frames: runtime_session.dropped_frames,
        provider_status: runtime_session.provider_status.clone(),
        readiness_state: runtime_session.readiness_state.clone(),
        runtime_session,
        stinger: stinger_metadata,
        validation,
    }
}

fn designer_runtime_session_snapshot(
    target: &str,
    scene: (&str, &str),
    frame_index: u64,
    framerate: u32,
    generated_at: chrono::DateTime<chrono::Utc>,
    render_result: Option<&SoftwareCompositorRenderResult>,
    validation: &SceneRuntimeContractValidation,
) -> DesignerRuntimeSessionSnapshot {
    let sources = render_result
        .map(|result| {
            result
                .input_frames
                .iter()
                .map(|input| {
                    let source_name = result
                        .frame
                        .targets
                        .iter()
                        .flat_map(|target| target.nodes.iter())
                        .find(|node| node.source_id == input.source_id)
                        .map(|node| node.name.as_str())
                        .unwrap_or(input.source_id.as_str());
                    designer_runtime_source_session(
                        target,
                        source_name,
                        input,
                        framerate,
                        frame_index,
                        generated_at,
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let dropped_frames = sources
        .iter()
        .map(|source| source.dropped_frames)
        .sum::<u64>();
    let readiness_state = aggregate_runtime_readiness(
        sources.iter().map(|source| &source.readiness_state),
        validation,
    );
    let session_state = session_state_from_readiness(&readiness_state, validation);
    let provider_status = provider_summary(&sources, validation);
    let stale_frame_ms = sources
        .iter()
        .map(|source| source.stale_frame_ms)
        .max()
        .unwrap_or_else(|| frame_interval_ms(framerate));

    DesignerRuntimeSessionSnapshot {
        version: 1,
        runtime_session_id: format!("designer:{target}:{}", scene.0),
        target: target.to_string(),
        scene_id: scene.0.to_string(),
        scene_name: scene.1.to_string(),
        frame_index,
        target_framerate: framerate.max(1),
        session_state,
        readiness_state,
        provider_status,
        last_frame_at: generated_at,
        stale_frame_ms,
        restart_count: 0,
        dropped_frames,
        sources,
        validation: validation.clone(),
    }
}

fn designer_runtime_source_session(
    target: &str,
    source_name: &str,
    input: &SoftwareCompositorInputFrame,
    framerate: u32,
    _frame_index: u64,
    generated_at: chrono::DateTime<chrono::Utc>,
) -> DesignerRuntimeSourceSession {
    let readiness_state = input_readiness_state(input);
    let session_state = match readiness_state {
        DesignerRuntimeReadinessState::Ready => DesignerRuntimeSessionState::Running,
        DesignerRuntimeReadinessState::Degraded => DesignerRuntimeSessionState::Degraded,
        DesignerRuntimeReadinessState::Blocked => DesignerRuntimeSessionState::Blocked,
        DesignerRuntimeReadinessState::NotApplicable => DesignerRuntimeSessionState::Idle,
    };
    let dropped_frames = input
        .capture
        .as_ref()
        .map(|capture| capture.dropped_frames)
        .unwrap_or(0);
    let stale_frame_ms = if readiness_state == DesignerRuntimeReadinessState::Ready {
        0
    } else {
        frame_interval_ms(framerate).saturating_mul(2)
    };

    DesignerRuntimeSourceSession {
        source_id: input.source_id.clone(),
        source_name: source_name.to_string(),
        source_kind: input.source_kind.clone(),
        runtime_session_id: format!(
            "designer:{target}:{:?}:{}",
            input.source_kind, input.source_id
        ),
        session_state,
        last_frame_at: generated_at,
        stale_frame_ms,
        restart_count: 0,
        dropped_frames,
        provider_status: input_provider_status(input),
        readiness_state,
        detail: input_runtime_detail(input),
    }
}

fn transition_runtime_source_session(
    transition: Option<&SceneTransition>,
    frame_index: u64,
    framerate: u32,
    generated_at: chrono::DateTime<chrono::Utc>,
    validation: &SceneRuntimeContractValidation,
    stinger: Option<&StingerTransitionRuntimeMetadata>,
) -> DesignerRuntimeSourceSession {
    let transition_id = transition
        .map(|transition| transition.id.as_str())
        .unwrap_or("missing-transition");
    let transition_name = transition
        .map(|transition| transition.name.as_str())
        .unwrap_or("Transition");
    let readiness_state = if !validation.errors.is_empty() {
        DesignerRuntimeReadinessState::Blocked
    } else if let Some(stinger) = stinger {
        match stinger.status {
            StingerTransitionRuntimeStatus::Rendered => DesignerRuntimeReadinessState::Ready,
            StingerTransitionRuntimeStatus::NoAsset => DesignerRuntimeReadinessState::NotApplicable,
            StingerTransitionRuntimeStatus::MissingFile
            | StingerTransitionRuntimeStatus::UnsupportedExtension => {
                DesignerRuntimeReadinessState::Blocked
            }
            StingerTransitionRuntimeStatus::FfmpegUnavailable
            | StingerTransitionRuntimeStatus::DecodeFailed
            | StingerTransitionRuntimeStatus::NotStinger => DesignerRuntimeReadinessState::Degraded,
        }
    } else if validation.warnings.is_empty() {
        DesignerRuntimeReadinessState::Ready
    } else {
        DesignerRuntimeReadinessState::Degraded
    };
    let session_state = session_state_from_readiness(&readiness_state, validation);
    let stale_frame_ms = if readiness_state == DesignerRuntimeReadinessState::Ready {
        0
    } else {
        frame_interval_ms(framerate).saturating_mul(2)
    };

    DesignerRuntimeSourceSession {
        source_id: transition_id.to_string(),
        source_name: transition_name.to_string(),
        source_kind: SceneSourceKind::ImageMedia,
        runtime_session_id: format!("designer:transition:{transition_id}"),
        session_state,
        last_frame_at: generated_at,
        stale_frame_ms,
        restart_count: 0,
        dropped_frames: 0,
        provider_status: stinger
            .and_then(|stinger| stinger.decoder_name.clone())
            .unwrap_or_else(|| "software-transition".to_string()),
        readiness_state,
        detail: stinger
            .map(|stinger| stinger.status_detail.clone())
            .unwrap_or_else(|| format!("Transition frame {frame_index} rendered in software.")),
    }
}

fn designer_transition_runtime_session_snapshot(
    request: &TransitionPreviewFrameRequest,
    transition_kind: SceneTransitionKind,
    scene_name: &str,
    generated_at: chrono::DateTime<chrono::Utc>,
    source: DesignerRuntimeSourceSession,
    validation: &SceneRuntimeContractValidation,
) -> DesignerRuntimeSessionSnapshot {
    let sources = vec![source];
    let readiness_state = aggregate_runtime_readiness(
        sources.iter().map(|source| &source.readiness_state),
        validation,
    );
    let session_state = session_state_from_readiness(&readiness_state, validation);
    let provider_status = provider_summary(&sources, validation);
    let dropped_frames = sources
        .iter()
        .map(|source| source.dropped_frames)
        .sum::<u64>();
    let stale_frame_ms = sources
        .iter()
        .map(|source| source.stale_frame_ms)
        .max()
        .unwrap_or_else(|| frame_interval_ms(request.framerate));

    DesignerRuntimeSessionSnapshot {
        version: 1,
        runtime_session_id: format!(
            "designer:transition_preview:{:?}:{}",
            transition_kind, request.transition_id
        ),
        target: "transition_preview".to_string(),
        scene_id: request.from_scene_id.clone(),
        scene_name: scene_name.to_string(),
        frame_index: request.frame_index,
        target_framerate: request.framerate.max(1),
        session_state,
        readiness_state,
        provider_status,
        last_frame_at: generated_at,
        stale_frame_ms,
        restart_count: 0,
        dropped_frames,
        sources,
        validation: validation.clone(),
    }
}

fn input_readiness_state(input: &SoftwareCompositorInputFrame) -> DesignerRuntimeReadinessState {
    let mut state = match input.status {
        CompositorNodeStatus::Ready => DesignerRuntimeReadinessState::Ready,
        CompositorNodeStatus::Hidden => DesignerRuntimeReadinessState::NotApplicable,
        CompositorNodeStatus::Placeholder => DesignerRuntimeReadinessState::Degraded,
        CompositorNodeStatus::PermissionRequired | CompositorNodeStatus::Unavailable => {
            DesignerRuntimeReadinessState::Blocked
        }
    };

    if let Some(asset) = &input.asset {
        state = merge_runtime_readiness(
            state,
            match asset.status {
                SoftwareCompositorAssetStatus::Decoded => DesignerRuntimeReadinessState::Ready,
                SoftwareCompositorAssetStatus::NoAsset => {
                    DesignerRuntimeReadinessState::NotApplicable
                }
                SoftwareCompositorAssetStatus::VideoPlaceholder => {
                    DesignerRuntimeReadinessState::Degraded
                }
                SoftwareCompositorAssetStatus::MissingFile
                | SoftwareCompositorAssetStatus::UnsupportedExtension
                | SoftwareCompositorAssetStatus::DecodeFailed => {
                    DesignerRuntimeReadinessState::Blocked
                }
            },
        );
    }

    if let Some(browser) = &input.browser {
        state = merge_runtime_readiness(
            state,
            match browser.status {
                SoftwareCompositorBrowserStatus::Rendered => DesignerRuntimeReadinessState::Ready,
                SoftwareCompositorBrowserStatus::NoUrl => {
                    DesignerRuntimeReadinessState::NotApplicable
                }
                SoftwareCompositorBrowserStatus::UnsupportedUrl => {
                    DesignerRuntimeReadinessState::Blocked
                }
                SoftwareCompositorBrowserStatus::BrowserUnavailable
                | SoftwareCompositorBrowserStatus::NavigationFailed
                | SoftwareCompositorBrowserStatus::CaptureFailed => {
                    DesignerRuntimeReadinessState::Degraded
                }
            },
        );
    }

    if let Some(capture) = &input.capture {
        state = merge_runtime_readiness(
            state,
            match capture.status {
                SoftwareCompositorCaptureStatus::Rendered => DesignerRuntimeReadinessState::Ready,
                SoftwareCompositorCaptureStatus::NoSource
                | SoftwareCompositorCaptureStatus::PermissionRequired
                | SoftwareCompositorCaptureStatus::UnsupportedSource => {
                    DesignerRuntimeReadinessState::Blocked
                }
                SoftwareCompositorCaptureStatus::DecoderUnavailable
                | SoftwareCompositorCaptureStatus::UnsupportedPlatform
                | SoftwareCompositorCaptureStatus::CaptureFailed => {
                    DesignerRuntimeReadinessState::Degraded
                }
            },
        );
    }

    if input
        .filters
        .iter()
        .any(|filter| filter.status == SoftwareCompositorFilterStatus::Error)
    {
        state = merge_runtime_readiness(state, DesignerRuntimeReadinessState::Degraded);
    }

    state
}

fn merge_runtime_readiness(
    current: DesignerRuntimeReadinessState,
    next: DesignerRuntimeReadinessState,
) -> DesignerRuntimeReadinessState {
    if runtime_readiness_rank(&next) > runtime_readiness_rank(&current) {
        next
    } else {
        current
    }
}

fn aggregate_runtime_readiness<'a>(
    states: impl Iterator<Item = &'a DesignerRuntimeReadinessState>,
    validation: &SceneRuntimeContractValidation,
) -> DesignerRuntimeReadinessState {
    if !validation.errors.is_empty() {
        return DesignerRuntimeReadinessState::Blocked;
    }
    let mut state = if validation.warnings.is_empty() {
        DesignerRuntimeReadinessState::Ready
    } else {
        DesignerRuntimeReadinessState::Degraded
    };
    let mut saw_source = false;
    for source_state in states {
        saw_source = true;
        state = merge_runtime_readiness(state, source_state.clone());
    }
    if saw_source {
        state
    } else if validation.errors.is_empty() {
        DesignerRuntimeReadinessState::NotApplicable
    } else {
        DesignerRuntimeReadinessState::Blocked
    }
}

fn runtime_readiness_rank(state: &DesignerRuntimeReadinessState) -> u8 {
    match state {
        DesignerRuntimeReadinessState::Ready => 0,
        DesignerRuntimeReadinessState::NotApplicable => 1,
        DesignerRuntimeReadinessState::Degraded => 2,
        DesignerRuntimeReadinessState::Blocked => 3,
    }
}

fn session_state_from_readiness(
    readiness: &DesignerRuntimeReadinessState,
    validation: &SceneRuntimeContractValidation,
) -> DesignerRuntimeSessionState {
    if !validation.errors.is_empty() {
        return DesignerRuntimeSessionState::Blocked;
    }
    match readiness {
        DesignerRuntimeReadinessState::Ready => DesignerRuntimeSessionState::Running,
        DesignerRuntimeReadinessState::Degraded => DesignerRuntimeSessionState::Degraded,
        DesignerRuntimeReadinessState::Blocked => DesignerRuntimeSessionState::Blocked,
        DesignerRuntimeReadinessState::NotApplicable => DesignerRuntimeSessionState::Idle,
    }
}

fn input_provider_status(input: &SoftwareCompositorInputFrame) -> String {
    if let Some(capture) = &input.capture {
        return format!("{}:{:?}", capture.provider_name, capture.status);
    }
    if let Some(browser) = &input.browser {
        return browser
            .browser_name
            .clone()
            .unwrap_or_else(|| format!("browser:{:?}", browser.status));
    }
    if let Some(asset) = &input.asset {
        return asset
            .decoder_name
            .clone()
            .or_else(|| asset.format.clone())
            .unwrap_or_else(|| format!("asset:{:?}", asset.status));
    }
    if let Some(text) = &input.text {
        return format!("text:{}", text.used_font_family);
    }
    "software-placeholder".to_string()
}

fn input_runtime_detail(input: &SoftwareCompositorInputFrame) -> String {
    if let Some(capture) = &input.capture {
        return capture.status_detail.clone();
    }
    if let Some(browser) = &input.browser {
        return browser.status_detail.clone();
    }
    if let Some(asset) = &input.asset {
        return asset.status_detail.clone();
    }
    if let Some(text) = &input.text {
        return text.status_detail.clone();
    }
    input.status_detail.clone()
}

fn provider_summary(
    sources: &[DesignerRuntimeSourceSession],
    validation: &SceneRuntimeContractValidation,
) -> String {
    if !validation.errors.is_empty() {
        return format!("blocked:{} error(s)", validation.errors.len());
    }
    if sources.is_empty() {
        return "no runtime sources".to_string();
    }
    let ready = sources
        .iter()
        .filter(|source| source.readiness_state == DesignerRuntimeReadinessState::Ready)
        .count();
    let degraded = sources
        .iter()
        .filter(|source| source.readiness_state == DesignerRuntimeReadinessState::Degraded)
        .count();
    let blocked = sources
        .iter()
        .filter(|source| source.readiness_state == DesignerRuntimeReadinessState::Blocked)
        .count();
    format!(
        "{ready} ready / {degraded} degraded / {blocked} blocked / {} total",
        sources.len()
    )
}

fn frame_interval_ms(framerate: u32) -> u64 {
    1_000_u64.div_ceil(u64::from(framerate.max(1)))
}

struct RenderedTransitionFrame {
    frame: SoftwareCompositorFrame,
    stinger: Option<StingerTransitionRuntimeMetadata>,
    validation: SceneRuntimeContractValidation,
}

struct TransitionRenderContext<'a> {
    request: &'a TransitionPreviewFrameRequest,
    transition: &'a SceneTransition,
    from_scene: &'a Scene,
    to_scene: &'a Scene,
    frame_index: u64,
    elapsed_ms: u32,
    trigger_time_ms: u32,
    triggered: bool,
    eased_progress: f64,
}

fn render_transition_preview_frame(
    context: TransitionRenderContext<'_>,
) -> RenderedTransitionFrame {
    match context.transition.kind {
        SceneTransitionKind::Stinger => {
            let rendered =
                render_stinger_transition_preview_frame(StingerTransitionRenderContext {
                    request: context.request,
                    transition: context.transition,
                    from_scene: context.from_scene,
                    to_scene: context.to_scene,
                    frame_index: context.frame_index,
                    elapsed_ms: context.elapsed_ms,
                    trigger_time_ms: context.trigger_time_ms,
                    triggered: context.triggered,
                });
            RenderedTransitionFrame {
                frame: rendered.frame,
                stinger: Some(rendered.stinger),
                validation: rendered.validation,
            }
        }
        SceneTransitionKind::Cut | SceneTransitionKind::Fade | SceneTransitionKind::Swipe => {
            render_pixel_transition_preview_frame(context)
        }
    }
}

fn render_pixel_transition_preview_frame(
    context: TransitionRenderContext<'_>,
) -> RenderedTransitionFrame {
    let mut validation = runtime_validation(Vec::new(), Vec::new());
    let from_frame =
        render_transition_scene_frame(context.request, context.from_scene, context.frame_index);
    let to_frame =
        render_transition_scene_frame(context.request, context.to_scene, context.frame_index);

    validation
        .warnings
        .extend(from_frame.validation.warnings.clone());
    validation
        .errors
        .extend(from_frame.validation.errors.clone());
    validation
        .warnings
        .extend(to_frame.validation.warnings.clone());
    validation.errors.extend(to_frame.validation.errors.clone());
    validation.ready = validation.errors.is_empty();

    let mut frame = match context.transition.kind {
        SceneTransitionKind::Cut => to_frame.frame,
        SceneTransitionKind::Fade => {
            fade_transition_frame(&from_frame.frame, &to_frame.frame, context.eased_progress)
        }
        SceneTransitionKind::Swipe => swipe_transition_frame(
            &from_frame.frame,
            &to_frame.frame,
            context.transition,
            context.eased_progress,
        ),
        SceneTransitionKind::Stinger => unreachable!("stingers are rendered in the stinger path"),
    };
    frame.target_id = "target-transition-preview".to_string();
    frame.target_kind = CompositorRenderTargetKind::Preview;
    frame.checksum = checksum_software_pixels(&frame.pixels);

    RenderedTransitionFrame {
        frame,
        stinger: None,
        validation,
    }
}

struct RenderedSceneTransitionFrame {
    frame: SoftwareCompositorFrame,
    validation: SceneRuntimeContractValidation,
}

fn render_transition_scene_frame(
    request: &TransitionPreviewFrameRequest,
    scene: &Scene,
    frame_index: u64,
) -> RenderedSceneTransitionFrame {
    let result = scene_render_result(
        scene,
        SceneRenderTargetOptions {
            target_id: "target-transition-preview",
            target_name: "Transition Preview",
            target_kind: CompositorRenderTargetKind::Preview,
            width: request.width,
            height: request.height,
            framerate: request.framerate,
            frame_format: request.frame_format.clone(),
            scale_mode: request.scale_mode.clone(),
        },
        frame_index,
    );
    let frame = result
        .pixel_frames
        .first()
        .cloned()
        .unwrap_or_else(|| blank_software_preview_frame(request));
    RenderedSceneTransitionFrame {
        frame,
        validation: runtime_validation(
            result.frame.validation.errors.clone(),
            result.frame.validation.warnings.clone(),
        ),
    }
}

fn fade_transition_frame(
    from_frame: &SoftwareCompositorFrame,
    to_frame: &SoftwareCompositorFrame,
    progress: f64,
) -> SoftwareCompositorFrame {
    let progress = progress.clamp(0.0, 1.0);
    let inverse = 1.0 - progress;
    let mut frame = from_frame.clone();
    for (output, (from, to)) in frame
        .pixels
        .iter_mut()
        .zip(from_frame.pixels.iter().zip(to_frame.pixels.iter()))
    {
        *output = ((*from as f64 * inverse) + (*to as f64 * progress)).round() as u8;
    }
    frame
}

fn swipe_transition_frame(
    from_frame: &SoftwareCompositorFrame,
    to_frame: &SoftwareCompositorFrame,
    transition: &SceneTransition,
    progress: f64,
) -> SoftwareCompositorFrame {
    let width = from_frame.width.max(1);
    let height = from_frame.height.max(1);
    let progress = progress.clamp(0.0, 1.0);
    let direction = transition
        .config
        .get("direction")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("left");
    let distance_x = (f64::from(width) * progress).round() as i32;
    let distance_y = (f64::from(height) * progress).round() as i32;
    let width_i32 = width as i32;
    let height_i32 = height as i32;
    let (from_dx, from_dy, to_dx, to_dy) = match direction {
        "right" => (distance_x, 0, -width_i32 + distance_x, 0),
        "up" => (0, -distance_y, 0, height_i32 - distance_y),
        "down" => (0, distance_y, 0, -height_i32 + distance_y),
        _ => (-distance_x, 0, width_i32 - distance_x, 0),
    };

    let mut pixels = vec![0; width as usize * height as usize * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[7, 9, 17, 255]);
    }
    draw_frame_with_offset(&mut pixels, width, height, from_frame, from_dx, from_dy);
    draw_frame_with_offset(&mut pixels, width, height, to_frame, to_dx, to_dy);

    SoftwareCompositorFrame {
        target_id: "target-transition-preview".to_string(),
        target_kind: CompositorRenderTargetKind::Preview,
        width,
        height,
        frame_format: from_frame.frame_format.clone(),
        bytes_per_row: width as usize * 4,
        checksum: checksum_software_pixels(&pixels),
        pixels,
    }
}

fn draw_frame_with_offset(
    destination: &mut [u8],
    destination_width: u32,
    destination_height: u32,
    source: &SoftwareCompositorFrame,
    offset_x: i32,
    offset_y: i32,
) {
    let destination_width = destination_width.max(1) as i32;
    let destination_height = destination_height.max(1) as i32;
    let source_width = source.width.max(1) as i32;
    let source_height = source.height.max(1) as i32;

    for source_y in 0..source_height {
        let destination_y = source_y + offset_y;
        if destination_y < 0 || destination_y >= destination_height {
            continue;
        }
        for source_x in 0..source_width {
            let destination_x = source_x + offset_x;
            if destination_x < 0 || destination_x >= destination_width {
                continue;
            }
            let source_offset = ((source_y * source_width + source_x) * 4) as usize;
            let destination_offset =
                ((destination_y * destination_width + destination_x) * 4) as usize;
            blend_rgba_pixel(
                &mut destination[destination_offset..destination_offset + 4],
                [
                    source.pixels[source_offset],
                    source.pixels[source_offset + 1],
                    source.pixels[source_offset + 2],
                    source.pixels[source_offset + 3],
                ],
            );
        }
    }
}

struct RenderedStingerTransitionFrame {
    frame: SoftwareCompositorFrame,
    stinger: StingerTransitionRuntimeMetadata,
    validation: SceneRuntimeContractValidation,
}

struct StingerTransitionRenderContext<'a> {
    request: &'a TransitionPreviewFrameRequest,
    transition: &'a SceneTransition,
    from_scene: &'a Scene,
    to_scene: &'a Scene,
    frame_index: u64,
    elapsed_ms: u32,
    trigger_time_ms: u32,
    triggered: bool,
}

fn render_stinger_transition_preview_frame(
    context: StingerTransitionRenderContext<'_>,
) -> RenderedStingerTransitionFrame {
    let StingerTransitionRenderContext {
        request,
        transition,
        from_scene,
        to_scene,
        frame_index,
        elapsed_ms,
        trigger_time_ms,
        triggered,
    } = context;
    let mut validation = runtime_validation(Vec::new(), Vec::new());
    let base_scene = if triggered { to_scene } else { from_scene };
    let scene_request = PreviewFrameRequest {
        version: 1,
        request_id: format!("{}-scene", request.request_id),
        scene_id: base_scene.id.clone(),
        width: request.width,
        height: request.height,
        framerate: request.framerate,
        frame_format: request.frame_format.clone(),
        scale_mode: request.scale_mode.clone(),
        encoding: PreviewFrameEncoding::None,
        include_debug_overlay: request.include_debug_overlay,
        requested_at: request.requested_at,
    };
    let base_render = preview_render_result(base_scene, &scene_request, 0);
    validation
        .warnings
        .extend(base_render.frame.validation.warnings.clone());
    validation
        .errors
        .extend(base_render.frame.validation.errors.clone());
    validation.ready = validation.errors.is_empty();

    let mut frame = base_render
        .pixel_frames
        .first()
        .cloned()
        .unwrap_or_else(|| blank_software_preview_frame(request));
    frame.target_id = "target-transition-preview".to_string();
    frame.target_kind = CompositorRenderTargetKind::Preview;

    let asset_uri = transition
        .config
        .get("asset_uri")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    let clock = transition_frame_clock(request.framerate, frame_index, elapsed_ms);
    let stinger_input =
        stinger_video_input_frame(&asset_uri, &clock, request.width, request.height);
    let stinger = stinger_metadata_from_input(
        asset_uri,
        trigger_time_ms,
        triggered,
        stinger_input.asset.as_ref(),
    );

    if stinger.status == StingerTransitionRuntimeStatus::Rendered {
        composite_scaled_rgba(&mut frame.pixels, frame.width, frame.height, &stinger_input);
    } else {
        paint_stinger_placeholder(&mut frame.pixels, frame.width, frame.height, triggered);
        validation.warnings.push(format!(
            "stinger transition preview is using a placeholder: {}",
            stinger.status_detail
        ));
        validation.ready = validation.errors.is_empty();
    }
    frame.checksum = checksum_software_pixels(&frame.pixels);

    RenderedStingerTransitionFrame {
        frame,
        stinger,
        validation,
    }
}

fn preview_render_result(
    scene: &Scene,
    request: &PreviewFrameRequest,
    frame_index: u64,
) -> SoftwareCompositorRenderResult {
    scene_render_result(
        scene,
        SceneRenderTargetOptions {
            target_id: "target-runtime-preview",
            target_name: "Runtime Preview",
            target_kind: CompositorRenderTargetKind::Preview,
            width: request.width,
            height: request.height,
            framerate: request.framerate,
            frame_format: request.frame_format.clone(),
            scale_mode: request.scale_mode.clone(),
        },
        frame_index,
    )
}

struct SceneRenderTargetOptions {
    target_id: &'static str,
    target_name: &'static str,
    target_kind: CompositorRenderTargetKind,
    width: u32,
    height: u32,
    framerate: u32,
    frame_format: CompositorFrameFormat,
    scale_mode: CompositorScaleMode,
}

fn scene_render_result(
    scene: &Scene,
    target_options: SceneRenderTargetOptions,
    frame_index: u64,
) -> SoftwareCompositorRenderResult {
    let graph = build_compositor_graph(scene);
    let target = CompositorRenderTarget {
        id: target_options.target_id.to_string(),
        name: target_options.target_name.to_string(),
        kind: target_options.target_kind,
        width: target_options.width,
        height: target_options.height,
        framerate: target_options.framerate,
        frame_format: target_options.frame_format,
        scale_mode: target_options.scale_mode,
        enabled: true,
    };
    let plan = build_compositor_render_plan(&graph, vec![target]);
    render_software_compositor_frame(&plan, frame_index)
}

fn transition_preview_frame_count(duration_ms: u32, framerate: u32) -> u64 {
    if duration_ms == 0 || framerate == 0 {
        return 1;
    }
    ((u64::from(duration_ms) * u64::from(framerate)).div_ceil(1_000)).max(1)
}

fn transition_preview_elapsed_ms(duration_ms: u32, frame_index: u64, framerate: u32) -> u32 {
    if framerate == 0 {
        return 0;
    }
    let elapsed = (frame_index.saturating_mul(1_000)) / u64::from(framerate);
    elapsed.min(u64::from(duration_ms)) as u32
}

fn transition_preview_linear_progress(duration_ms: u32, frame_index: u64, frame_count: u64) -> f64 {
    if duration_ms == 0 || frame_count <= 1 {
        return 1.0;
    }
    (frame_index as f64 / (frame_count - 1) as f64).clamp(0.0, 1.0)
}

fn transition_eased_progress(progress: f64, easing: &SceneTransitionEasing) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    match easing {
        SceneTransitionEasing::Linear => progress,
        SceneTransitionEasing::EaseIn => progress * progress,
        SceneTransitionEasing::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
        SceneTransitionEasing::EaseInOut => {
            if progress < 0.5 {
                2.0 * progress * progress
            } else {
                1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0
            }
        }
    }
}

fn stinger_trigger_time_ms(transition: &SceneTransition) -> u32 {
    let fallback = transition.duration_ms / 2;
    let value = transition
        .config
        .get("trigger_time_ms")
        .and_then(|value| {
            value.as_u64().or_else(|| {
                value
                    .as_f64()
                    .filter(|number| number.is_finite() && *number >= 0.0)
                    .map(|number| number.round() as u64)
            })
        })
        .unwrap_or(u64::from(fallback));
    value.min(u64::from(transition.duration_ms)) as u32
}

fn transition_frame_clock(
    framerate: u32,
    frame_index: u64,
    elapsed_ms: u32,
) -> CompositorFrameClock {
    let framerate = framerate.max(1);
    CompositorFrameClock {
        frame_index,
        framerate,
        pts_nanos: u64::from(elapsed_ms) * 1_000_000,
        duration_nanos: 1_000_000_000_u64 / u64::from(framerate),
    }
}

fn stinger_metadata_from_input(
    asset_uri: String,
    trigger_time_ms: u32,
    triggered: bool,
    asset: Option<&SoftwareCompositorAssetMetadata>,
) -> StingerTransitionRuntimeMetadata {
    let Some(asset) = asset else {
        return StingerTransitionRuntimeMetadata {
            uri: asset_uri,
            status: StingerTransitionRuntimeStatus::NoAsset,
            status_detail: "No local stinger asset has been selected.".to_string(),
            trigger_time_ms,
            triggered,
            format: None,
            width: None,
            height: None,
            checksum: None,
            modified_unix_ms: None,
            cache_hit: false,
            sampled_frame_time_ms: None,
            sample_index: None,
            decoder_name: None,
            media_timeline_state: None,
            timeline_position_ms: None,
            timeline_base_position_ms: None,
            playback_rate: None,
            loop_enabled: None,
            restart_on_scene_activate: None,
            fallback_reason: Some("No local stinger asset has been selected.".to_string()),
        };
    };
    let status = match asset.status {
        SoftwareCompositorAssetStatus::Decoded => StingerTransitionRuntimeStatus::Rendered,
        SoftwareCompositorAssetStatus::MissingFile => StingerTransitionRuntimeStatus::MissingFile,
        SoftwareCompositorAssetStatus::UnsupportedExtension => {
            StingerTransitionRuntimeStatus::UnsupportedExtension
        }
        SoftwareCompositorAssetStatus::DecodeFailed => StingerTransitionRuntimeStatus::DecodeFailed,
        SoftwareCompositorAssetStatus::VideoPlaceholder => {
            StingerTransitionRuntimeStatus::FfmpegUnavailable
        }
        SoftwareCompositorAssetStatus::NoAsset => StingerTransitionRuntimeStatus::NoAsset,
    };
    let fallback_reason = if status == StingerTransitionRuntimeStatus::Rendered {
        None
    } else {
        Some(asset.status_detail.clone())
    };

    StingerTransitionRuntimeMetadata {
        uri: asset.uri.clone(),
        status,
        status_detail: asset.status_detail.clone(),
        trigger_time_ms,
        triggered,
        format: asset.format.clone(),
        width: asset.width,
        height: asset.height,
        checksum: asset.checksum,
        modified_unix_ms: asset.modified_unix_ms,
        cache_hit: asset.cache_hit,
        sampled_frame_time_ms: asset.sampled_frame_time_ms,
        sample_index: asset.sample_index,
        decoder_name: asset.decoder_name.clone(),
        media_timeline_state: asset.media_timeline_state.clone(),
        timeline_position_ms: asset.timeline_position_ms,
        timeline_base_position_ms: asset.timeline_base_position_ms,
        playback_rate: asset.playback_rate,
        loop_enabled: asset.loop_enabled,
        restart_on_scene_activate: asset.restart_on_scene_activate,
        fallback_reason,
    }
}

fn blank_software_preview_frame(
    request: &TransitionPreviewFrameRequest,
) -> SoftwareCompositorFrame {
    let width = request.width.max(1);
    let height = request.height.max(1);
    let mut pixels = vec![0; width as usize * height as usize * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[16, 20, 29, 255]);
    }
    SoftwareCompositorFrame {
        target_id: "target-transition-preview".to_string(),
        target_kind: CompositorRenderTargetKind::Preview,
        width,
        height,
        frame_format: CompositorFrameFormat::Rgba8,
        bytes_per_row: width as usize * 4,
        checksum: checksum_software_pixels(&pixels),
        pixels,
    }
}

fn composite_scaled_rgba(
    destination: &mut [u8],
    destination_width: u32,
    destination_height: u32,
    source: &SoftwareCompositorInputFrame,
) {
    let destination_width = destination_width.max(1) as usize;
    let destination_height = destination_height.max(1) as usize;
    let source_width = source.width.max(1) as usize;
    let source_height = source.height.max(1) as usize;

    for y in 0..destination_height {
        let source_y = ((y * source_height) / destination_height).min(source_height - 1);
        for x in 0..destination_width {
            let source_x = ((x * source_width) / destination_width).min(source_width - 1);
            let source_offset = (source_y * source_width + source_x) * 4;
            let destination_offset = (y * destination_width + x) * 4;
            blend_rgba_pixel(
                &mut destination[destination_offset..destination_offset + 4],
                [
                    source.pixels[source_offset],
                    source.pixels[source_offset + 1],
                    source.pixels[source_offset + 2],
                    source.pixels[source_offset + 3],
                ],
            );
        }
    }
}

fn paint_stinger_placeholder(destination: &mut [u8], width: u32, height: u32, triggered: bool) {
    let width = width.max(1) as usize;
    let height = height.max(1) as usize;
    let color = if triggered {
        [64, 205, 150, 150]
    } else {
        [242, 151, 76, 150]
    };

    for y in 0..height {
        for x in 0..width {
            let border = x < 8 || y < 8 || x + 8 >= width || y + 8 >= height;
            let diagonal = ((x + y) / 18) % 6 == 0;
            let center_band = y > height / 2 - height / 16 && y < height / 2 + height / 16;
            if border || diagonal || center_band {
                let offset = (y * width + x) * 4;
                blend_rgba_pixel(&mut destination[offset..offset + 4], color);
            }
        }
    }
}

fn blend_rgba_pixel(pixel: &mut [u8], color: [u8; 4]) {
    let alpha = u16::from(color[3]);
    if alpha == 0 {
        return;
    }
    let inverse_alpha = 255 - alpha;
    pixel[0] = blend_rgba_channel(color[0], pixel[0], alpha, inverse_alpha);
    pixel[1] = blend_rgba_channel(color[1], pixel[1], alpha, inverse_alpha);
    pixel[2] = blend_rgba_channel(color[2], pixel[2], alpha, inverse_alpha);
    pixel[3] = (alpha + u16::from(pixel[3]) * inverse_alpha / 255).min(255) as u8;
}

fn blend_rgba_channel(source: u8, destination: u8, alpha: u16, inverse_alpha: u16) -> u8 {
    ((u16::from(source) * alpha + u16::from(destination) * inverse_alpha + 127) / 255) as u8
}

fn preview_frame_checksum(
    frame: &CompositorRenderedFrame,
    request: &PreviewFrameRequest,
) -> String {
    rendered_frame_checksum(frame, request.width, request.height)
}

fn rendered_frame_checksum(frame: &CompositorRenderedFrame, width: u32, height: u32) -> String {
    let visible_nodes = frame
        .targets
        .iter()
        .flat_map(|target| target.nodes.iter())
        .count();
    format!(
        "contract:{}:{}:{}x{}:{}",
        frame.scene_id, frame.clock.frame_index, width, height, visible_nodes
    )
}

fn preview_image_data(
    frame: &SoftwareCompositorFrame,
    encoding: &PreviewFrameEncoding,
) -> Option<String> {
    match encoding {
        PreviewFrameEncoding::None => None,
        PreviewFrameEncoding::Base64 => Some(base64_encode(&frame.pixels)),
        PreviewFrameEncoding::DataUrl => {
            let bytes = bmp_bytes(frame);
            Some(format!("data:image/bmp;base64,{}", base64_encode(&bytes)))
        }
    }
}

fn bmp_bytes(frame: &SoftwareCompositorFrame) -> Vec<u8> {
    let width = frame.width.max(1);
    let height = frame.height.max(1);
    let pixel_bytes = width as usize * height as usize * 4;
    let file_size = 14 + 40 + pixel_bytes;
    let mut bytes = Vec::with_capacity(file_size);

    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&(file_size as u32).to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    bytes.extend_from_slice(&(54_u32).to_le_bytes());
    bytes.extend_from_slice(&(40_u32).to_le_bytes());
    bytes.extend_from_slice(&(width as i32).to_le_bytes());
    bytes.extend_from_slice(&(-(height as i32)).to_le_bytes());
    bytes.extend_from_slice(&(1_u16).to_le_bytes());
    bytes.extend_from_slice(&(32_u16).to_le_bytes());
    bytes.extend_from_slice(&(0_u32).to_le_bytes());
    bytes.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
    bytes.extend_from_slice(&(2835_i32).to_le_bytes());
    bytes.extend_from_slice(&(2835_i32).to_le_bytes());
    bytes.extend_from_slice(&(0_u32).to_le_bytes());
    bytes.extend_from_slice(&(0_u32).to_le_bytes());

    for pixel in frame.pixels.chunks_exact(4) {
        bytes.push(pixel[2]);
        bytes.push(pixel[1]);
        bytes.push(pixel[0]);
        bytes.push(pixel[3]);
    }

    bytes
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut index = 0;

    while index < bytes.len() {
        let first = bytes[index];
        let second = bytes.get(index + 1).copied().unwrap_or(0);
        let third = bytes.get(index + 2).copied().unwrap_or(0);
        let triple = ((u32::from(first)) << 16) | ((u32::from(second)) << 8) | u32::from(third);

        encoded.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        encoded.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        if index + 1 < bytes.len() {
            encoded.push(TABLE[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            encoded.push('=');
        }
        if index + 2 < bytes.len() {
            encoded.push(TABLE[(triple & 0x3f) as usize] as char);
        } else {
            encoded.push('=');
        }

        index += 3;
    }

    encoded
}

fn validate_scene_runtime_collection(
    collection: &SceneCollection,
) -> SceneRuntimeContractValidation {
    let collection_validation = validate_scene_collection(collection);
    runtime_validation(
        Vec::new(),
        if collection_validation.ok {
            Vec::new()
        } else {
            vec![format!(
                "scene collection has {} validation issue(s)",
                collection_validation.issues.len()
            )]
        },
    )
}

fn compositor_validation_to_runtime(
    validation: crate::CompositorValidation,
) -> SceneRuntimeContractValidation {
    SceneRuntimeContractValidation {
        ready: validation.ready,
        warnings: validation.warnings,
        errors: validation.errors,
    }
}

fn runtime_validation(
    errors: Vec<String>,
    warnings: Vec<String>,
) -> SceneRuntimeContractValidation {
    SceneRuntimeContractValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

fn validate_runtime_envelope(version: u32, id: &str, label: &str, errors: &mut Vec<String>) {
    if version == 0 {
        errors.push(format!("{label} version must be greater than zero"));
    }
    if id.trim().is_empty() {
        errors.push(format!("{label} id is required"));
    }
}

fn validate_optional_positive(value: Option<u32>, label: &str, errors: &mut Vec<String>) {
    if matches!(value, Some(0)) {
        errors.push(format!("{label} must be greater than zero"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        process::Command,
    };

    #[test]
    fn scene_runtime_snapshot_tracks_active_scene_and_transition() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let snapshot = scene_runtime_snapshot(&collection);

        assert_eq!(snapshot.collection_id, collection.id);
        assert_eq!(snapshot.active_scene_id, "scene-main");
        assert_eq!(snapshot.active_transition_id, "transition-fade");
        assert_eq!(snapshot.status, SceneRuntimeStatus::Active);
        assert!(snapshot.validation.ready);
    }

    #[test]
    fn preview_frame_response_returns_contract_frame_metadata() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let request = PreviewFrameRequest {
            version: 1,
            request_id: "preview-test".to_string(),
            scene_id: scene.id.clone(),
            width: 1280,
            height: 720,
            framerate: 30,
            frame_format: CompositorFrameFormat::Rgba8,
            scale_mode: CompositorScaleMode::Fit,
            encoding: PreviewFrameEncoding::DataUrl,
            include_debug_overlay: true,
            requested_at: crate::now_utc(),
        };
        let response = create_preview_frame_response(&request, &collection, 3);

        assert_eq!(response.scene_id, "scene-main");
        assert_eq!(response.frame_index, 3);
        assert!(response.checksum.is_some());
        assert!(response
            .image_data
            .as_deref()
            .is_some_and(|data| data.starts_with("data:image/bmp;base64,")));
        let rendered_frame = response.rendered_frame.as_ref().unwrap();
        assert_eq!(rendered_frame.renderer, CompositorRendererKind::Software);
        assert_eq!(response.runtime_session.target, "runtime_preview");
        assert_eq!(
            response.runtime_session_id,
            response.runtime_session.runtime_session_id
        );
        assert!(!response.runtime_session.sources.is_empty());
        assert_eq!(response.session_state, DesignerRuntimeSessionState::Blocked);
        assert!(response.render_time_ms.is_finite());
        assert!(
            response.validation.ready,
            "{:?}",
            response.validation.errors
        );
    }

    #[test]
    fn program_preview_frame_response_renders_active_scene_as_program_target() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let request = ProgramPreviewFrameRequest {
            version: 1,
            request_id: "program-preview-test".to_string(),
            collection_id: collection.id.clone(),
            width: 1920,
            height: 1080,
            framerate: 60,
            frame_format: CompositorFrameFormat::Rgba8,
            scale_mode: CompositorScaleMode::Fit,
            encoding: PreviewFrameEncoding::DataUrl,
            include_debug_overlay: true,
            requested_at: crate::now_utc(),
        };
        let response = create_program_preview_frame_response(&request, &collection, 7);

        assert_eq!(response.collection_id, collection.id);
        assert_eq!(response.scene_id, collection.active_scene_id);
        assert_eq!(response.program_target_id, "target-program-preview");
        assert_eq!(response.frame_index, 7);
        assert_eq!(response.framerate, 60);
        assert!(response
            .checksum
            .as_deref()
            .is_some_and(|checksum| checksum.starts_with("software-program:")));
        assert!(response
            .image_data
            .as_deref()
            .is_some_and(|data| data.starts_with("data:image/bmp;base64,")));
        let target = &response.rendered_frame.as_ref().unwrap().targets[0];
        assert_eq!(target.target_id, "target-program-preview");
        assert_eq!(target.target_kind, CompositorRenderTargetKind::Program);
        assert_eq!(response.runtime_session.target, "program_preview");
        assert_eq!(response.runtime_session.target_framerate, 60);
        assert!(!response.runtime_session.sources.is_empty());
        assert!(
            response.validation.ready,
            "{:?}",
            response.validation.errors
        );
    }

    #[test]
    fn transition_preview_response_reports_runtime_session_metadata() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let request = TransitionPreviewFrameRequest {
            version: 1,
            request_id: "transition-runtime-session-test".to_string(),
            collection_id: collection.id.clone(),
            transition_id: collection.active_transition_id.clone(),
            from_scene_id: collection.active_scene_id.clone(),
            to_scene_id: collection.active_scene_id.clone(),
            frame_index: 1,
            width: 1280,
            height: 720,
            framerate: 30,
            frame_format: CompositorFrameFormat::Rgba8,
            scale_mode: CompositorScaleMode::Fit,
            encoding: PreviewFrameEncoding::None,
            include_debug_overlay: false,
            requested_at: crate::now_utc(),
        };
        let response = create_transition_preview_frame_response(&request, &collection);

        assert_eq!(response.runtime_session.target, "transition_preview");
        assert_eq!(
            response.runtime_session_id,
            response.runtime_session.runtime_session_id
        );
        assert_eq!(response.runtime_session.sources.len(), 1);
        assert_eq!(response.runtime_session.target_framerate, 30);
    }

    #[test]
    fn runtime_binding_contracts_describe_default_scene_sources() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let bindings = build_scene_runtime_bindings_snapshot(scene);

        assert_eq!(bindings.scene_id, "scene-main");
        assert!(bindings
            .capture
            .bindings
            .iter()
            .any(|binding| binding.media_kind == CaptureFrameMediaKind::Video));
        assert!(bindings
            .capture
            .bindings
            .iter()
            .any(|binding| binding.media_kind == CaptureFrameMediaKind::Audio));
        assert!(bindings
            .audio
            .buses
            .iter()
            .any(|bus| bus.kind == AudioMixBusKind::Master));
    }

    #[test]
    fn pixel_transition_preview_renders_cut_fade_and_swipe_frames() {
        for (kind, config) in [
            (SceneTransitionKind::Cut, serde_json::json!({})),
            (
                SceneTransitionKind::Fade,
                serde_json::json!({ "color": "#000000" }),
            ),
            (
                SceneTransitionKind::Swipe,
                serde_json::json!({ "direction": "left", "edge_softness": 0.0 }),
            ),
        ] {
            let collection = transition_test_collection(kind.clone(), config);
            let request = transition_preview_request(&collection, 8);
            let response = create_transition_preview_frame_response(&request, &collection);

            assert_eq!(response.transition_kind, kind);
            assert!(
                response.validation.ready,
                "{:?}",
                response.validation.errors
            );
            assert!(response.stinger.is_none());
            assert!(response
                .checksum
                .as_deref()
                .is_some_and(|checksum| checksum.starts_with("software-transition:")));
            assert!(response
                .image_data
                .as_deref()
                .is_some_and(|data| data.starts_with("data:image/bmp;base64,")));
            assert!((0.0..=1.0).contains(&response.linear_progress));
            assert!((0.0..=1.0).contains(&response.eased_progress));
        }
    }

    #[test]
    fn stinger_transition_preview_uses_placeholder_without_asset() {
        let collection = stinger_test_collection(None);
        let request = stinger_preview_request(&collection, 0);
        let response = create_transition_preview_frame_response(&request, &collection);

        assert!(
            response.validation.ready,
            "{:?}",
            response.validation.errors
        );
        assert!(response
            .image_data
            .as_deref()
            .is_some_and(|data| data.starts_with("data:image/bmp;base64,")));
        assert_eq!(
            response.stinger.as_ref().unwrap().status,
            StingerTransitionRuntimeStatus::NoAsset
        );
        assert!(response
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("placeholder")));
    }

    #[test]
    fn stinger_transition_preview_reports_missing_and_unsupported_assets() {
        let dir = tempfile::tempdir().unwrap();
        let missing_path = dir.path().join("missing.mp4");
        let missing_collection = stinger_test_collection(Some(missing_path.display().to_string()));
        let missing = create_transition_preview_frame_response(
            &stinger_preview_request(&missing_collection, 0),
            &missing_collection,
        );
        assert_eq!(
            missing.stinger.as_ref().unwrap().status,
            StingerTransitionRuntimeStatus::MissingFile
        );

        let unsupported_path = dir.path().join("stinger.txt");
        std::fs::write(&unsupported_path, b"not video").unwrap();
        let unsupported_collection =
            stinger_test_collection(Some(unsupported_path.display().to_string()));
        let unsupported = create_transition_preview_frame_response(
            &stinger_preview_request(&unsupported_collection, 0),
            &unsupported_collection,
        );
        assert_eq!(
            unsupported.stinger.as_ref().unwrap().status,
            StingerTransitionRuntimeStatus::UnsupportedExtension
        );
        assert!(unsupported.checksum.is_some());
    }

    #[test]
    fn stinger_transition_preview_switches_scene_at_trigger_time() {
        let collection = stinger_test_collection(None);
        let before = create_transition_preview_frame_response(
            &stinger_preview_request(&collection, 0),
            &collection,
        );
        let after = create_transition_preview_frame_response(
            &stinger_preview_request(&collection, 20),
            &collection,
        );

        assert!(!before.triggered);
        assert!(after.triggered);
        assert_ne!(before.checksum, after.checksum);
    }

    #[test]
    fn stinger_transition_preview_decodes_and_caches_video_when_ffmpeg_is_available() {
        let Some(ffmpeg_path) = find_ffmpeg_for_test() else {
            eprintln!("skipping stinger video preview test because ffmpeg is unavailable");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stinger.mp4");
        if !write_test_video(&ffmpeg_path, &path, "cyan") {
            eprintln!("skipping stinger video preview test because fixture generation failed");
            return;
        }

        let collection = stinger_test_collection(Some(path.display().to_string()));
        let request = stinger_preview_request(&collection, 0);
        let first = create_transition_preview_frame_response(&request, &collection);
        let second = create_transition_preview_frame_response(&request, &collection);

        assert_eq!(
            first.stinger.as_ref().unwrap().status,
            StingerTransitionRuntimeStatus::Rendered
        );
        assert_eq!(
            first.stinger.as_ref().unwrap().decoder_name.as_deref(),
            Some("ffmpeg")
        );
        assert!(!first.stinger.as_ref().unwrap().cache_hit);
        assert!(second.stinger.as_ref().unwrap().cache_hit);
        assert_ne!(first.checksum, None);
    }

    fn stinger_test_collection(asset_uri: Option<String>) -> SceneCollection {
        let mut collection = SceneCollection::default_collection(crate::now_utc());
        let mut to_scene = collection.scenes[0].clone();
        to_scene.id = "scene-to".to_string();
        to_scene.name = "To Scene".to_string();
        to_scene.canvas.background_color = "#123824".to_string();
        collection.scenes.push(to_scene);
        collection.active_transition_id = "transition-stinger".to_string();
        collection.transitions.push(SceneTransition {
            id: "transition-stinger".to_string(),
            name: "Stinger".to_string(),
            kind: SceneTransitionKind::Stinger,
            duration_ms: 1_000,
            easing: crate::SceneTransitionEasing::Linear,
            config: serde_json::json!({
                "asset_uri": asset_uri,
                "trigger_time_ms": 500
            }),
        });
        collection
    }

    fn transition_test_collection(
        kind: SceneTransitionKind,
        config: serde_json::Value,
    ) -> SceneCollection {
        let mut collection = SceneCollection::default_collection(crate::now_utc());
        let mut to_scene = collection.scenes[0].clone();
        to_scene.id = "scene-to".to_string();
        to_scene.name = "To Scene".to_string();
        to_scene.canvas.background_color = "#123824".to_string();
        collection.scenes.push(to_scene);
        collection.active_transition_id = "transition-test".to_string();
        collection.transitions.push(SceneTransition {
            id: "transition-test".to_string(),
            name: "Pixel Transition".to_string(),
            kind,
            duration_ms: 600,
            easing: crate::SceneTransitionEasing::EaseInOut,
            config,
        });
        collection
    }

    fn stinger_preview_request(
        collection: &SceneCollection,
        frame_index: u64,
    ) -> TransitionPreviewFrameRequest {
        transition_preview_request(collection, frame_index)
    }

    fn transition_preview_request(
        collection: &SceneCollection,
        frame_index: u64,
    ) -> TransitionPreviewFrameRequest {
        TransitionPreviewFrameRequest {
            version: 1,
            request_id: format!("transition-{frame_index}"),
            collection_id: collection.id.clone(),
            transition_id: collection.active_transition_id.clone(),
            from_scene_id: "scene-main".to_string(),
            to_scene_id: "scene-to".to_string(),
            frame_index,
            width: 320,
            height: 180,
            framerate: 30,
            frame_format: CompositorFrameFormat::Rgba8,
            scale_mode: CompositorScaleMode::Fit,
            encoding: PreviewFrameEncoding::DataUrl,
            include_debug_overlay: false,
            requested_at: crate::now_utc(),
        }
    }

    fn find_ffmpeg_for_test() -> Option<PathBuf> {
        [
            std::env::var_os("VAEXCORE_FFMPEG_PATH").map(PathBuf::from),
            Some(PathBuf::from("ffmpeg")),
            Some(PathBuf::from("/opt/homebrew/bin/ffmpeg")),
            Some(PathBuf::from("/usr/local/bin/ffmpeg")),
        ]
        .into_iter()
        .flatten()
        .find(|candidate| {
            Command::new(candidate)
                .arg("-version")
                .output()
                .is_ok_and(|output| output.status.success())
        })
    }

    fn write_test_video(ffmpeg_path: &Path, path: &Path, color: &str) -> bool {
        Command::new(ffmpeg_path)
            .arg("-v")
            .arg("error")
            .arg("-y")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg(format!("color=c={color}:s=16x16:d=1:r=30"))
            .arg("-frames:v")
            .arg("30")
            .arg("-c:v")
            .arg("mpeg4")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg(path)
            .status()
            .is_ok_and(|status| status.success())
    }
}
