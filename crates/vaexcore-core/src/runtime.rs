use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    build_audio_mixer_plan, build_capture_frame_plan, build_compositor_graph,
    build_compositor_render_plan, build_scene_transition_preview_plan, evaluate_compositor_frame,
    validate_compositor_render_plan, validate_scene_collection, AudioMixBus, AudioMixBusKind,
    AudioMixSourceStatus, CaptureFrameBindingStatus, CaptureFrameFormat, CaptureFrameMediaKind,
    CaptureSourceKind, CompositorFrameClock, CompositorFrameFormat, CompositorRenderPlan,
    CompositorRenderTarget, CompositorRenderedFrame, CompositorRendererKind, CompositorScaleMode,
    Scene, SceneCollection, SceneSourceKind, SceneTransitionPreviewPlan,
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

pub fn create_preview_frame_response(
    request: &PreviewFrameRequest,
    collection: &SceneCollection,
    frame_index: u64,
) -> PreviewFrameResponse {
    let mut validation = validate_preview_frame_request(request, collection);
    let scene = collection
        .scenes
        .iter()
        .find(|scene| scene.id == request.scene_id)
        .or_else(|| collection.active_scene())
        .or_else(|| collection.scenes.first());
    let rendered_frame = scene.map(|scene| preview_rendered_frame(scene, request, frame_index));

    if let Some(frame) = &rendered_frame {
        validation
            .warnings
            .extend(frame.validation.warnings.clone());
        validation.errors.extend(frame.validation.errors.clone());
        validation.ready = validation.errors.is_empty();
    }

    let checksum = rendered_frame
        .as_ref()
        .map(|frame| preview_frame_checksum(frame, request));

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
        image_data: None,
        checksum,
        render_time_ms: 0.0,
        generated_at: crate::now_utc(),
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

fn preview_rendered_frame(
    scene: &Scene,
    request: &PreviewFrameRequest,
    frame_index: u64,
) -> CompositorRenderedFrame {
    let graph = build_compositor_graph(scene);
    let target = CompositorRenderTarget {
        id: "target-runtime-preview".to_string(),
        name: "Runtime Preview".to_string(),
        kind: crate::CompositorRenderTargetKind::Preview,
        width: request.width,
        height: request.height,
        framerate: request.framerate,
        frame_format: request.frame_format.clone(),
        scale_mode: request.scale_mode.clone(),
        enabled: true,
    };
    let plan = build_compositor_render_plan(&graph, vec![target]);
    evaluate_compositor_frame(&plan, frame_index)
}

fn preview_frame_checksum(
    frame: &CompositorRenderedFrame,
    request: &PreviewFrameRequest,
) -> String {
    let visible_nodes = frame
        .targets
        .iter()
        .flat_map(|target| target.nodes.iter())
        .count();
    format!(
        "contract:{}:{}:{}x{}:{}",
        frame.scene_id, frame.clock.frame_index, request.width, request.height, visible_nodes
    )
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
        assert!(response.rendered_frame.is_some());
        assert!(
            response.validation.ready,
            "{:?}",
            response.validation.errors
        );
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
}
