pub mod audio;
pub mod capture;
pub mod compositor;
pub mod events;
pub mod performance;
pub mod pipeline;
pub mod profiles;
pub mod responses;
pub mod runtime;
pub mod scenes;
pub mod security;
pub mod settings;
pub mod status;

pub use audio::{
    build_audio_graph_runtime_snapshot, build_audio_mixer_plan,
    build_live_audio_graph_runtime_snapshot, validate_audio_graph_runtime_snapshot,
    validate_audio_mixer_plan, AudioFilterRuntimeMetadata, AudioFilterRuntimeStatus,
    AudioGraphInputMode, AudioGraphRuntimeBus, AudioGraphRuntimeSnapshot, AudioGraphRuntimeSource,
    AudioGraphRuntimeValidation, AudioMixBus, AudioMixBusKind, AudioMixSource,
    AudioMixSourceStatus, AudioMixerPlan, AudioMixerValidation,
};
pub use capture::{
    build_capture_frame_plan, build_capture_provider_runtime_snapshot, default_capture_sources,
    validate_capture_frame_plan, validate_capture_provider_runtime_snapshot, CaptureAudioPacket,
    CaptureFrameBinding, CaptureFrameBindingStatus, CaptureFrameFormat, CaptureFrameMediaKind,
    CaptureFramePlan, CaptureFrameTransport, CaptureFrameValidation, CaptureProvider,
    CaptureProviderLifecycleState, CaptureProviderRuntimeSnapshot, CaptureProviderStatus,
    CaptureSourceCandidate, CaptureSourceInventory, CaptureSourceKind, CaptureSourceSelection,
    CaptureVideoFramePacket, MockCaptureProvider,
};
pub use compositor::{
    build_compositor_graph, build_compositor_render_plan, build_software_compositor_input_frames,
    build_software_compositor_input_frames_at_clock, checksum_software_pixels,
    compositor_render_target, evaluate_compositor_frame, render_software_compositor_frame,
    stinger_video_input_frame, validate_compositor_graph, validate_compositor_render_plan,
    CompositorBlendMode, CompositorEvaluatedNode, CompositorFrameClock, CompositorFrameFormat,
    CompositorGraph, CompositorNode, CompositorNodeRole, CompositorNodeStatus, CompositorOutput,
    CompositorRect, CompositorRenderPlan, CompositorRenderTarget, CompositorRenderTargetKind,
    CompositorRenderedFrame, CompositorRenderedTarget, CompositorRendererKind, CompositorScaleMode,
    CompositorTransform, CompositorValidation, SoftwareCompositorAssetMetadata,
    SoftwareCompositorAssetStatus, SoftwareCompositorBrowserMetadata,
    SoftwareCompositorBrowserStatus, SoftwareCompositorFilterMetadata,
    SoftwareCompositorFilterStatus, SoftwareCompositorFrame, SoftwareCompositorInputFrame,
    SoftwareCompositorMediaPlaybackState, SoftwareCompositorRenderResult,
    SoftwareCompositorTextMetadata, SoftwareCompositorTextStatus,
};
pub use events::{StudioEvent, StudioEventKind};
pub use performance::{
    build_performance_telemetry_plan, validate_performance_telemetry_plan, PerformanceTargetBudget,
    PerformanceTelemetryPlan, PerformanceTelemetryValidation,
};
pub use pipeline::{
    build_output_preflight_plan, validate_output_preflight_plan, MediaPipelineConfig,
    MediaPipelinePlan, MediaPipelinePlanRequest, MediaPipelineStep, MediaPipelineValidation,
    OutputPreflightPlan, OutputPreflightValidation, PipelineIntent, PipelineStepStatus,
    RecordingTargetContract, RenderTargetProfile, StreamingTargetContract,
};
pub use profiles::{
    EncoderPreference, MediaProfile, MediaProfileInput, PlatformKind, RecordingContainer,
    Resolution, StreamDestination, StreamDestinationInput,
};
pub use responses::{
    ApiErrorBody, ApiResponse, AuditLogEntry, AuditLogSnapshot, CommandStatus, ConnectedClient,
    ConnectedClientsSnapshot, HealthResponse, LocalRuntimeDependency, LocalRuntimeHealth, Marker,
    MarkersSnapshot, PreflightCheck, PreflightSnapshot, PreflightStatus, ProfileBundle,
    ProfileBundleImportResult, ProfilesSnapshot, RecentRecordingsSnapshot, RecordingHistoryEntry,
    StreamDestinationBundleItem, StudioStatus,
};
pub use runtime::{
    build_runtime_audio_source_binding_contract, build_runtime_capture_source_binding_contract,
    build_scene_runtime_bindings_snapshot, create_compositor_render_response,
    create_preview_frame_response, create_program_preview_frame_response,
    create_scene_activation_response, create_scene_runtime_state_update_response,
    create_transition_execution_response, create_transition_preview_frame_response,
    scene_runtime_snapshot, scene_runtime_snapshot_with_options,
    validate_compositor_render_request, validate_preview_frame_request,
    validate_program_preview_frame_request, validate_runtime_audio_source_binding_contract,
    validate_runtime_capture_source_binding_contract, validate_scene_activation_request,
    validate_scene_runtime_state_update_request, validate_transition_execution_request,
    validate_transition_preview_frame_request, CompositorRenderRequest, CompositorRenderResponse,
    CompositorRenderTargetResult, PreviewFrameEncoding, PreviewFrameRequest, PreviewFrameResponse,
    ProgramPreviewFrameRequest, ProgramPreviewFrameResponse, RuntimeAudioSourceBinding,
    RuntimeAudioSourceBindingContract, RuntimeCaptureSourceBinding,
    RuntimeCaptureSourceBindingContract, SceneActivationRequest, SceneActivationResponse,
    SceneActivationStatus, SceneRuntimeBindingsSnapshot, SceneRuntimeCommand,
    SceneRuntimeCommandKind, SceneRuntimeContractValidation, SceneRuntimeSnapshot,
    SceneRuntimeStatePatch, SceneRuntimeStateUpdateRequest, SceneRuntimeStateUpdateResponse,
    SceneRuntimeStatus, StingerTransitionRuntimeMetadata, StingerTransitionRuntimeStatus,
    TransitionExecutionRequest, TransitionExecutionResponse, TransitionPreviewFrameRequest,
    TransitionPreviewFrameResponse,
};
pub use scenes::{
    build_scene_transition_preview_plan, scene_capture_sources, scene_resolution,
    validate_scene_collection, validate_scene_transition_preview_plan, Scene, SceneCanvas,
    SceneCollection, SceneCollectionBundle, SceneCollectionImportResult, SceneCrop, ScenePoint,
    SceneSize, SceneSource, SceneSourceBoundsMode, SceneSourceFilter, SceneSourceFilterKind,
    SceneSourceKind, SceneTransition, SceneTransitionEasing, SceneTransitionKind,
    SceneTransitionPreviewPlan, SceneTransitionPreviewSample, SceneTransitionPreviewValidation,
    SceneValidationIssue, SceneValidationResult,
};
pub use security::{
    SecretRef, SecretStore, SecretStoreError, SensitiveString, LOCAL_SQLITE_SECRET_PROVIDER,
    MACOS_KEYCHAIN_SECRET_PROVIDER, WINDOWS_CREDENTIAL_MANAGER_SECRET_PROVIDER,
};
pub use settings::AppSettings;
pub use status::{EngineMode, EngineStatus, RecordingSession, StreamSession};

pub const APP_NAME: &str = "vaexcore studio";
pub const DEFAULT_API_PORT: u16 = 51287;
pub const DEFAULT_API_HOST: &str = "127.0.0.1";

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
}

pub fn now_utc() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
