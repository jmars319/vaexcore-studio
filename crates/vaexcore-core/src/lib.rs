pub mod capture;
pub mod compositor;
pub mod events;
pub mod pipeline;
pub mod profiles;
pub mod responses;
pub mod scenes;
pub mod security;
pub mod settings;
pub mod status;

pub use capture::{
    default_capture_sources, CaptureSourceCandidate, CaptureSourceInventory, CaptureSourceKind,
    CaptureSourceSelection,
};
pub use compositor::{
    build_compositor_graph, build_compositor_render_plan, compositor_render_target,
    evaluate_compositor_frame, validate_compositor_graph, validate_compositor_render_plan,
    CompositorBlendMode, CompositorEvaluatedNode, CompositorFrameClock, CompositorFrameFormat,
    CompositorGraph, CompositorNode, CompositorNodeRole, CompositorNodeStatus, CompositorOutput,
    CompositorRect, CompositorRenderPlan, CompositorRenderTarget, CompositorRenderTargetKind,
    CompositorRenderedFrame, CompositorRenderedTarget, CompositorRendererKind, CompositorScaleMode,
    CompositorTransform, CompositorValidation,
};
pub use events::{StudioEvent, StudioEventKind};
pub use pipeline::{
    MediaPipelineConfig, MediaPipelinePlan, MediaPipelinePlanRequest, MediaPipelineStep,
    MediaPipelineValidation, PipelineIntent, PipelineStepStatus,
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
pub use scenes::{
    scene_capture_sources, scene_resolution, validate_scene_collection, Scene, SceneCanvas,
    SceneCollection, SceneCrop, ScenePoint, SceneSize, SceneSource, SceneSourceKind,
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
