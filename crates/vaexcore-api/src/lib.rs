mod auth;
mod client_registry;
mod event_bus;
mod store;

use std::{
    future::Future,
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use vaexcore_core::{
    new_id, now_utc, scene_capture_sources, ApiResponse, AuditLogEntry, AuditLogSnapshot,
    CommandStatus, ConnectedClientsSnapshot, HealthResponse, LocalRuntimeDependency,
    LocalRuntimeHealth, Marker, MarkersSnapshot, MediaPipelinePlan, MediaPipelinePlanRequest,
    MediaPipelineValidation, MediaProfileInput, PipelineIntent, ProfilesSnapshot,
    RecentRecordingsSnapshot, SceneCollection, SceneValidationResult, SecretStore,
    StreamDestinationInput, StudioEvent, StudioEventKind, StudioStatus, APP_NAME,
};
use vaexcore_media::{
    build_dry_run_pipeline_plan, DryRunMediaEngine, MediaEngine, MediaError, MediaRunnerSupervisor,
    RecordingLaunchRequest, SidecarMediaEngine, StreamLaunchRequest,
};

pub use auth::{AuthConfig, SharedAuthConfig};
use client_registry::{ClientRegistry, ClientSeen};
pub use event_bus::EventBus;
use store::{MarkerCreateInput, MarkerFilters};
pub use store::{ProfileStore, StoreError};

const REQUEST_ID_HEADER: &str = "x-vaexcore-request-id";
const CLIENT_ID_HEADER: &str = "x-vaexcore-client-id";
const CLIENT_NAME_HEADER: &str = "x-vaexcore-client-name";
const DEFAULT_EVENT_REPLAY_LIMIT: usize = 100;

#[derive(Clone, Debug)]
pub struct ApiServerConfig {
    pub bind_addr: SocketAddr,
    pub database_path: PathBuf,
    pub auth: SharedAuthConfig,
    pub media_runner: Option<MediaRunnerSupervisor>,
    pub pipeline_plan_path: Option<PathBuf>,
    pub pipeline_config_path: Option<PathBuf>,
}

pub struct ApiState {
    pub auth: SharedAuthConfig,
    pub store: ProfileStore,
    pub database_path: PathBuf,
    pub engine: Arc<dyn MediaEngine>,
    pub events: EventBus,
    pub clients: ClientRegistry,
    pub media_runner: Option<MediaRunnerSupervisor>,
    pub pipeline_plan_path: Option<PathBuf>,
    pub pipeline_config_path: Option<PathBuf>,
}

impl ApiState {
    pub fn new(config: &ApiServerConfig) -> Result<Arc<Self>, StoreError> {
        let events = EventBus::new();
        let event_sink = {
            let events = events.clone();
            Arc::new(move |event: StudioEvent| events.emit(event))
        };
        let engine: Arc<dyn MediaEngine> = match config.media_runner.clone() {
            Some(runner) => Arc::new(SidecarMediaEngine::new(runner, Some(event_sink))),
            None => Arc::new(DryRunMediaEngine::new(Some(event_sink))),
        };
        let monitor_events = events.clone();
        let state = Arc::new(Self {
            auth: config.auth.clone(),
            store: ProfileStore::open(&config.database_path)?,
            database_path: config.database_path.clone(),
            engine,
            events,
            clients: ClientRegistry::new(),
            media_runner: config.media_runner.clone(),
            pipeline_plan_path: config.pipeline_plan_path.clone(),
            pipeline_config_path: config.pipeline_config_path.clone(),
        });

        state
            .events
            .emit(StudioEvent::simple(StudioEventKind::AppReady));
        state
            .events
            .emit(StudioEvent::simple(StudioEventKind::MediaEngineReady));
        if let Some(runner) = config.media_runner.clone() {
            spawn_media_runner_monitor(runner, monitor_events);
        }

        Ok(state)
    }

    pub fn new_in_memory(auth: AuthConfig) -> Result<Arc<Self>, StoreError> {
        let events = EventBus::new();
        let event_sink = {
            let events = events.clone();
            Arc::new(move |event: StudioEvent| events.emit(event))
        };
        let engine: Arc<dyn MediaEngine> = Arc::new(DryRunMediaEngine::new(Some(event_sink)));
        let state = Arc::new(Self {
            auth: SharedAuthConfig::new(auth),
            store: ProfileStore::open_memory()?,
            database_path: PathBuf::from(":memory:"),
            engine,
            events,
            clients: ClientRegistry::new(),
            media_runner: None,
            pipeline_plan_path: None,
            pipeline_config_path: None,
        });

        state
            .events
            .emit(StudioEvent::simple(StudioEventKind::AppReady));
        state
            .events
            .emit(StudioEvent::simple(StudioEventKind::MediaEngineReady));

        Ok(state)
    }
}

fn spawn_media_runner_monitor(runner: MediaRunnerSupervisor, events: EventBus) {
    tokio::spawn(async move {
        let mut ready = false;
        let mut restart_backoff = Duration::from_secs(1);

        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            match runner.status().await {
                Ok(status) if status.ready => {
                    if !ready {
                        events.emit(StudioEvent::new(
                            StudioEventKind::MediaEngineReady,
                            json!({
                                "engine": "SidecarMediaEngine",
                                "runner_service": status.service,
                                "runner_status_addr": runner.status_addr().to_string(),
                                "dry_run": status.dry_run,
                                "pipeline_name": status.pipeline_name,
                            }),
                        ));
                    }
                    ready = true;
                    restart_backoff = Duration::from_secs(1);
                }
                Ok(status) => {
                    if ready {
                        events.emit(StudioEvent::error(format!(
                            "media runner reported not ready: {}",
                            status.service
                        )));
                    }
                    ready = false;
                    restart_backoff = restart_media_runner(&runner, &events, restart_backoff).await;
                }
                Err(error) => {
                    if ready {
                        events.emit(StudioEvent::error(format!(
                            "media runner unavailable: {error}"
                        )));
                    }
                    ready = false;
                    restart_backoff = restart_media_runner(&runner, &events, restart_backoff).await;
                }
            }
        }
    });
}

async fn restart_media_runner(
    runner: &MediaRunnerSupervisor,
    events: &EventBus,
    restart_backoff: Duration,
) -> Duration {
    events.emit(StudioEvent::new(
        StudioEventKind::MediaEngineReady,
        json!({
            "engine": "SidecarMediaEngine",
            "restart": "attempting",
            "backoff_ms": restart_backoff.as_millis(),
        }),
    ));

    tokio::time::sleep(restart_backoff).await;
    match runner.restart().await {
        Ok(()) => {
            events.emit(StudioEvent::new(
                StudioEventKind::MediaEngineReady,
                json!({
                    "engine": "SidecarMediaEngine",
                    "restart": "complete",
                    "runner_status_addr": runner.status_addr().to_string(),
                }),
            ));
            Duration::from_secs(1)
        }
        Err(error) => {
            events.emit(StudioEvent::error(format!(
                "media runner restart failed: {error}"
            )));
            (restart_backoff * 2).min(Duration::from_secs(30))
        }
    }
}

pub async fn serve(config: ApiServerConfig) -> anyhow::Result<()> {
    serve_with_shutdown(config, std::future::pending::<()>()).await
}

pub async fn serve_with_shutdown<F>(config: ApiServerConfig, shutdown: F) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind(config.bind_addr).await?;
    serve_listener_with_shutdown(config, listener, shutdown).await
}

pub async fn serve_listener_with_shutdown<F>(
    mut config: ApiServerConfig,
    listener: TcpListener,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let local_addr = listener.local_addr()?;
    config.bind_addr = local_addr;
    let state = ApiState::new(&config)?;
    if let Ok(request) = default_pipeline_plan_request(&state) {
        let _ = plan_pipeline(&state, request).await;
    }

    let auth_snapshot = config.auth.get();
    tracing::info!(
        service = APP_NAME,
        %local_addr,
        dev_auth_bypass = auth_snapshot.dev_mode,
        "local API listening"
    );

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

pub fn router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/clients", get(get_clients))
        .route("/audit-log", get(get_audit_log))
        .route("/recordings/recent", get(get_recent_recordings))
        .route("/recording/start", post(start_recording))
        .route("/recording/stop", post(stop_recording))
        .route("/stream/start", post(start_stream))
        .route("/stream/stop", post(stop_stream))
        .route("/scenes", get(get_scenes).put(put_scenes))
        .route("/scenes/validate", post(post_scenes_validate))
        .route(
            "/media/plan",
            get(default_pipeline_plan).post(post_pipeline_plan),
        )
        .route(
            "/media/validate",
            get(default_pipeline_validation).post(post_pipeline_validation),
        )
        .route("/markers", get(get_markers))
        .route("/marker/create", post(create_marker))
        .route("/profiles", get(get_profiles).post(post_profiles))
        .route(
            "/profiles/recording/{id}",
            put(put_recording_profile).delete(delete_recording_profile),
        )
        .route(
            "/profiles/destinations/{id}",
            put(put_stream_destination).delete(delete_stream_destination),
        )
        .route("/events", get(events_ws))
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([Method::DELETE, Method::GET, Method::POST, Method::PUT])
                .allow_headers(tower_http::cors::Any),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_context_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn request_context_middleware(
    State(state): State<Arc<ApiState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| is_valid_request_id(value))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| new_id("req"));
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let client = client_seen_from_headers(request.headers(), &path, Some(request_id.clone()));
    state.clients.register(client.clone());

    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }

    if let Some(action) = command_action(&method, &path) {
        let status_code = response.status().as_u16();
        let entry = AuditLogEntry {
            id: new_id("audit"),
            request_id: request_id.clone(),
            method: method.to_string(),
            path: path.clone(),
            action,
            status_code,
            ok: response.status().is_success(),
            client_id: client.client_id,
            client_name: Some(client.name),
            created_at: now_utc(),
        };
        if let Err(error) = state.store.insert_audit_log_entry(&entry) {
            tracing::warn!(%error, "failed to write command audit entry");
        }
    }

    tracing::info!(
        %request_id,
        %method,
        %path,
        status = response.status().as_u16(),
        "handled API request"
    );
    response
}

fn is_valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn client_seen_from_headers(
    headers: &HeaderMap,
    path: &str,
    request_id: Option<String>,
) -> ClientSeen {
    let user_agent = header_string(headers, axum::http::header::USER_AGENT.as_str(), 240);
    let client_id = header_string(headers, CLIENT_ID_HEADER, 128).filter(|value| {
        value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    });
    let name = header_string(headers, CLIENT_NAME_HEADER, 96)
        .or_else(|| user_agent.as_ref().map(|value| compact_user_agent(value)))
        .unwrap_or_else(|| {
            if path == "/events" {
                "WebSocket client".to_string()
            } else {
                "Local HTTP client".to_string()
            }
        });

    ClientSeen {
        client_id,
        name,
        kind: if path == "/events" {
            "websocket".to_string()
        } else {
            "http".to_string()
        },
        user_agent,
        request_id,
        path: Some(path.to_string()),
    }
}

fn header_string(headers: &HeaderMap, name: &str, max_len: usize) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max_len).collect())
}

fn normalize_optional_query(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn compact_user_agent(value: &str) -> String {
    value
        .split_whitespace()
        .next()
        .unwrap_or("Local client")
        .chars()
        .take(96)
        .collect()
}

fn command_action(method: &Method, path: &str) -> Option<String> {
    let action = match (method.as_str(), path) {
        ("POST", "/recording/start") => "recording.start",
        ("POST", "/recording/stop") => "recording.stop",
        ("POST", "/stream/start") => "stream.start",
        ("POST", "/stream/stop") => "stream.stop",
        ("PUT", "/scenes") => "scenes.save",
        ("POST", "/scenes/validate") => "scenes.validate",
        ("POST", "/media/plan") => "media.plan",
        ("POST", "/media/validate") => "media.validate",
        ("POST", "/marker/create") => "marker.create",
        ("POST", "/profiles") => "profiles.create",
        ("PUT", path) if path.starts_with("/profiles/recording/") => "profiles.recording.update",
        ("DELETE", path) if path.starts_with("/profiles/recording/") => "profiles.recording.delete",
        ("PUT", path) if path.starts_with("/profiles/destinations/") => {
            "profiles.destination.update"
        }
        ("DELETE", path) if path.starts_with("/profiles/destinations/") => {
            "profiles.destination.delete"
        }
        _ => return None,
    };
    Some(action.to_string())
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body: ApiResponse<serde_json::Value> = ApiResponse::error(self.code, self.message);
        (self.status, Json(body)).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(error: StoreError) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "store_error",
            error.to_string(),
        )
    }
}

impl From<MediaError> for ApiError {
    fn from(error: MediaError) -> Self {
        let status = match error {
            MediaError::InvalidCommand(_) => StatusCode::BAD_REQUEST,
            MediaError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        Self::new(status, "media_error", error.to_string())
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct StartRecordingRequest {
    pub profile_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct StartStreamRequest {
    pub destination_id: Option<String>,
    #[serde(default)]
    pub bandwidth_test: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CreateMarkerRequest {
    pub label: Option<String>,
    pub source_app: Option<String>,
    pub source_event_id: Option<String>,
    pub recording_session_id: Option<String>,
    pub media_path: Option<String>,
    pub start_seconds: Option<f64>,
    pub end_seconds: Option<f64>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MarkerQuery {
    pub source_app: Option<String>,
    pub source_event_id: Option<String>,
    pub recording_session_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum CreateProfileRequest {
    RecordingProfile(MediaProfileInput),
    StreamDestination(StreamDestinationInput),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum CreatedProfile {
    RecordingProfile(vaexcore_core::MediaProfile),
    StreamDestination(vaexcore_core::StreamDestination),
}

#[derive(Clone, Debug, Serialize)]
pub struct DeletedProfile {
    pub id: String,
    pub deleted: bool,
}

async fn health(State(state): State<Arc<ApiState>>) -> Json<ApiResponse<HealthResponse>> {
    let auth = state.auth.get();
    Json(ApiResponse::ok(HealthResponse {
        service: APP_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        ok: true,
        auth_required: auth.auth_required(),
        dev_auth_bypass: auth.dev_mode,
        local_runtime: studio_local_runtime_health(&state),
    }))
}

fn studio_local_runtime_health(state: &ApiState) -> LocalRuntimeHealth {
    let app_storage_dir = state
        .database_path
        .parent()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| state.database_path.display().to_string());
    let secret_storage = state.store.secret_storage_report().ok();
    let secure_storage = secret_storage
        .as_ref()
        .map(|report| report.secure_storage.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let secret_storage_state = secret_storage
        .as_ref()
        .map(|report| report.secret_storage_state.clone())
        .unwrap_or_else(|| "unavailable".to_string());

    LocalRuntimeHealth {
        contract_version: 1,
        mode: "local-first".to_string(),
        state: "ready".to_string(),
        app_storage_dir,
        suite_dir: suite_discovery_dir().display().to_string(),
        secure_storage,
        secret_storage_state,
        durable_storage: vec![
            "SQLite profiles, destinations, markers, and app settings".to_string(),
            "SQLite scene collections".to_string(),
            "Stream keys in app-owned secure storage".to_string(),
            "api-discovery.json".to_string(),
            "pipeline-plan.json and pipeline-config.json".to_string(),
        ],
        network_policy: "localhost-only".to_string(),
        dependencies: vec![LocalRuntimeDependency {
            name: "media-runner".to_string(),
            kind: "managed-sidecar".to_string(),
            state: if state.media_runner.is_some() {
                "managed".to_string()
            } else {
                "dry-run-fallback".to_string()
            },
            detail: if state.media_runner.is_some() {
                "Studio launched the bundled media-runner sidecar.".to_string()
            } else {
                "Studio is using the in-process dry-run media engine.".to_string()
            },
        }],
    }
}

async fn status(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<StudioStatus>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(StudioStatus {
        status: state.engine.status().await,
        recent_events: state.events.recent(),
    })))
}

async fn get_profiles(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<ProfilesSnapshot>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(state.store.profiles_snapshot()?)))
}

async fn get_scenes(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<SceneCollection>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(state.store.scene_collection()?)))
}

async fn put_scenes(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(collection): Json<SceneCollection>,
) -> Result<Json<ApiResponse<SceneCollection>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let saved = state.store.save_scene_collection(collection)?;
    refresh_default_pipeline_contract(&state).await;
    Ok(Json(ApiResponse::ok(saved)))
}

async fn post_scenes_validate(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(collection): Json<SceneCollection>,
) -> Result<Json<ApiResponse<SceneValidationResult>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(
        state.store.validate_scene_collection(&collection),
    )))
}

async fn get_clients(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<ConnectedClientsSnapshot>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(ConnectedClientsSnapshot {
        clients: state.clients.recent(),
    })))
}

async fn get_audit_log(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<AuditLogSnapshot>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(AuditLogSnapshot {
        entries: state.store.list_audit_log_entries(100)?,
    })))
}

async fn get_recent_recordings(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<RecentRecordingsSnapshot>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(RecentRecordingsSnapshot {
        recordings: state.store.list_recent_recordings(20)?,
    })))
}

async fn default_pipeline_plan(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<MediaPipelinePlan>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let request = default_pipeline_plan_request(&state)?;
    Ok(Json(ApiResponse::ok(plan_pipeline(&state, request).await)))
}

async fn post_pipeline_plan(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<MediaPipelinePlanRequest>,
) -> Result<Json<ApiResponse<MediaPipelinePlan>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(plan_pipeline(&state, request).await)))
}

async fn default_pipeline_validation(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<MediaPipelineValidation>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let request = default_pipeline_plan_request(&state)?;
    Ok(Json(ApiResponse::ok(
        plan_pipeline(&state, request).await.validation(),
    )))
}

async fn post_pipeline_validation(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<MediaPipelinePlanRequest>,
) -> Result<Json<ApiResponse<MediaPipelineValidation>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(
        plan_pipeline(&state, request).await.validation(),
    )))
}

async fn post_profiles(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<CreateProfileRequest>,
) -> Result<Json<ApiResponse<CreatedProfile>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;

    let created = match request {
        CreateProfileRequest::RecordingProfile(input) => {
            CreatedProfile::RecordingProfile(state.store.insert_recording_profile(input)?)
        }
        CreateProfileRequest::StreamDestination(input) => {
            CreatedProfile::StreamDestination(state.store.insert_stream_destination(input)?)
        }
    };
    refresh_default_pipeline_contract(&state).await;

    Ok(Json(ApiResponse::ok(created)))
}

async fn put_recording_profile(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<MediaProfileInput>,
) -> Result<Json<ApiResponse<CreatedProfile>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let profile = state
        .store
        .update_recording_profile(&id, input)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "recording_profile_not_found",
                "recording profile not found",
            )
        })?;
    refresh_default_pipeline_contract(&state).await;

    Ok(Json(ApiResponse::ok(CreatedProfile::RecordingProfile(
        profile,
    ))))
}

async fn delete_recording_profile(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DeletedProfile>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let status = state.engine.status().await;
    if status
        .recording
        .as_ref()
        .is_some_and(|session| session.profile.id == id)
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "recording_profile_in_use",
            "cannot delete the active recording profile",
        ));
    }

    if !state.store.delete_recording_profile(&id)? {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "recording_profile_not_found",
            "recording profile not found",
        ));
    }
    refresh_default_pipeline_contract(&state).await;

    Ok(Json(ApiResponse::ok(DeletedProfile { id, deleted: true })))
}

async fn put_stream_destination(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<StreamDestinationInput>,
) -> Result<Json<ApiResponse<CreatedProfile>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let destination = state
        .store
        .update_stream_destination(&id, input)?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "stream_destination_not_found",
                "stream destination not found",
            )
        })?;
    refresh_default_pipeline_contract(&state).await;

    Ok(Json(ApiResponse::ok(CreatedProfile::StreamDestination(
        destination,
    ))))
}

async fn delete_stream_destination(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DeletedProfile>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let status = state.engine.status().await;
    if status
        .stream
        .as_ref()
        .is_some_and(|session| session.destination.id == id)
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "stream_destination_in_use",
            "cannot delete the active stream destination",
        ));
    }

    if !state.store.delete_stream_destination(&id)? {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "stream_destination_not_found",
            "stream destination not found",
        ));
    }
    refresh_default_pipeline_contract(&state).await;

    Ok(Json(ApiResponse::ok(DeletedProfile { id, deleted: true })))
}

async fn start_recording(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    payload: Option<Json<StartRecordingRequest>>,
) -> Result<Json<ApiResponse<CommandStatus>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let request = payload.map(|Json(payload)| payload).unwrap_or_default();
    let profile = state
        .store
        .recording_profile_by_id(request.profile_id.as_deref())?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "recording_profile_not_found",
                "recording profile not found",
            )
        })?;
    let settings = state.store.app_settings()?;
    let active_scene = state.store.scene_collection()?.active_scene().cloned();
    let scene_capture_sources = active_scene
        .as_ref()
        .map(scene_capture_sources)
        .unwrap_or_default();
    let launch_request = RecordingLaunchRequest {
        profile,
        capture_sources: if scene_capture_sources.is_empty() {
            settings.capture_sources
        } else {
            scene_capture_sources
        },
        active_scene,
    };

    match state.engine.start_recording(launch_request).await {
        Ok(transition) => Ok(Json(ApiResponse::ok(CommandStatus {
            changed: transition.changed,
            message: if transition.changed {
                "recording started"
            } else {
                "recording already active"
            }
            .to_string(),
            status: transition.status,
        }))),
        Err(error) => {
            state.events.emit(StudioEvent::error(error.to_string()));
            Err(error.into())
        }
    }
}

async fn stop_recording(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<CommandStatus>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;

    match state.engine.stop_recording().await {
        Ok(transition) => {
            if transition.changed {
                if let Some(session) = &transition.session {
                    state.store.record_stopped_recording(session)?;
                }
            }

            Ok(Json(ApiResponse::ok(CommandStatus {
                changed: transition.changed,
                message: if transition.changed {
                    "recording stopped"
                } else {
                    "recording already stopped"
                }
                .to_string(),
                status: transition.status,
            })))
        }
        Err(error) => {
            state.events.emit(StudioEvent::error(error.to_string()));
            Err(error.into())
        }
    }
}

async fn start_stream(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    payload: Option<Json<StartStreamRequest>>,
) -> Result<Json<ApiResponse<CommandStatus>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let request = payload.map(|Json(payload)| payload).unwrap_or_default();
    let destination = state
        .store
        .stream_destination_by_id(request.destination_id.as_deref())?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "stream_destination_not_found",
                "enabled stream destination not found",
            )
        })?;

    let stream_key = destination
        .stream_key_ref
        .as_ref()
        .map(|reference| state.store.get_secret(reference).map_err(StoreError::from))
        .transpose()?
        .flatten()
        .map(|secret| secret.expose_secret().to_string());
    let settings = state.store.app_settings()?;
    let profile = state.store.recording_profile_by_id(None)?;
    let active_scene = state.store.scene_collection()?.active_scene().cloned();
    let scene_capture_sources = active_scene
        .as_ref()
        .map(scene_capture_sources)
        .unwrap_or_default();
    let launch_request = StreamLaunchRequest {
        destination,
        stream_key,
        bandwidth_test: request.bandwidth_test,
        capture_sources: if scene_capture_sources.is_empty() {
            settings.capture_sources
        } else {
            scene_capture_sources
        },
        profile,
        active_scene,
    };

    match state.engine.start_stream(launch_request).await {
        Ok(transition) => Ok(Json(ApiResponse::ok(CommandStatus {
            changed: transition.changed,
            message: if transition.changed {
                "stream started"
            } else {
                "stream already active"
            }
            .to_string(),
            status: transition.status,
        }))),
        Err(error) => {
            state.events.emit(StudioEvent::error(error.to_string()));
            Err(error.into())
        }
    }
}

async fn stop_stream(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<CommandStatus>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;

    match state.engine.stop_stream().await {
        Ok(transition) => Ok(Json(ApiResponse::ok(CommandStatus {
            changed: transition.changed,
            message: if transition.changed {
                "stream stopped"
            } else {
                "stream already stopped"
            }
            .to_string(),
            status: transition.status,
        }))),
        Err(error) => {
            state.events.emit(StudioEvent::error(error.to_string()));
            Err(error.into())
        }
    }
}

async fn get_markers(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<MarkerQuery>,
) -> Result<Json<ApiResponse<MarkersSnapshot>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    Ok(Json(ApiResponse::ok(MarkersSnapshot {
        markers: state.store.list_markers(MarkerFilters {
            source_app: normalize_optional_query(query.source_app),
            source_event_id: normalize_optional_query(query.source_event_id),
            recording_session_id: normalize_optional_query(query.recording_session_id),
            limit: query.limit,
        })?,
    })))
}

async fn create_marker(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    payload: Option<Json<CreateMarkerRequest>>,
) -> Result<Json<ApiResponse<Marker>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let request = payload.map(|Json(payload)| payload).unwrap_or_default();
    let result = state.store.create_marker(MarkerCreateInput {
        label: request.label,
        source_app: request.source_app,
        source_event_id: request.source_event_id,
        recording_session_id: request.recording_session_id,
        media_path: request.media_path,
        start_seconds: request.start_seconds,
        end_seconds: request.end_seconds,
        metadata: request.metadata,
    })?;
    let marker = result.marker;
    if result.created {
        state.events.emit(StudioEvent::new(
            StudioEventKind::MarkerCreated,
            json!({
                "marker_id": marker.id,
                "label": marker.label,
                "source_app": marker.source_app,
                "source_event_id": marker.source_event_id,
                "recording_session_id": marker.recording_session_id,
                "media_path": marker.media_path,
                "start_seconds": marker.start_seconds,
                "end_seconds": marker.end_seconds,
                "metadata": marker.metadata,
                "created_at": marker.created_at,
            }),
        ));
    }

    Ok(Json(ApiResponse::ok(marker)))
}

async fn events_ws(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<auth::TokenQuery>,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> Response {
    if let Err(error) = auth::authorize_query(&query, &state.auth)
        .or_else(|_| auth::authorize_headers(&headers, &state.auth))
    {
        return error.into_response();
    }

    if query.client_id.is_some() || query.client_name.is_some() {
        state.clients.register(ClientSeen {
            client_id: query.client_id.clone(),
            name: query
                .client_name
                .clone()
                .unwrap_or_else(|| "WebSocket client".to_string()),
            kind: "websocket".to_string(),
            user_agent: header_string(&headers, axum::http::header::USER_AGENT.as_str(), 240),
            request_id: header_string(&headers, REQUEST_ID_HEADER, 128),
            path: Some("/events".to_string()),
        });
    }

    let replay_limit = query
        .limit
        .unwrap_or(DEFAULT_EVENT_REPLAY_LIMIT)
        .min(event_bus::RECENT_EVENT_LIMIT);

    websocket
        .on_upgrade(move |socket| stream_events(socket, state, replay_limit))
        .into_response()
}

async fn stream_events(mut socket: WebSocket, state: Arc<ApiState>, replay_limit: usize) {
    for event in state.events.recent_limit(replay_limit) {
        match serde_json::to_string(&event) {
            Ok(serialized) => {
                if socket.send(Message::Text(serialized.into())).await.is_err() {
                    return;
                }
            }
            Err(error) => {
                tracing::warn!(%error, "failed to serialize recent event");
            }
        }
    }

    let mut receiver = state.events.subscribe();

    loop {
        tokio::select! {
            event = receiver.recv() => {
                match event {
                    Ok(event) => {
                        match serde_json::to_string(&event) {
                            Ok(serialized) => {
                                if socket.send(Message::Text(serialized.into())).await.is_err() {
                                    break;
                                }
                            }
                            Err(error) => tracing::warn!(%error, "failed to serialize event"),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let event = StudioEvent::new(
                            StudioEventKind::Error,
                            json!({ "message": format!("event stream lagged by {skipped} events") }),
                        );
                        if let Ok(serialized) = serde_json::to_string(&event) {
                            let _ = socket.send(Message::Text(serialized.into())).await;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            inbound = socket.recv() => {
                match inbound {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

fn default_pipeline_plan_request(state: &ApiState) -> Result<MediaPipelinePlanRequest, StoreError> {
    let settings = state.store.app_settings()?;
    let scene_collection = state.store.scene_collection()?;
    let active_scene = scene_collection.active_scene().cloned();
    let scene_capture_sources = active_scene
        .as_ref()
        .map(scene_capture_sources)
        .unwrap_or_default();
    let capture_sources = if scene_capture_sources.is_empty() {
        settings.capture_sources
    } else {
        scene_capture_sources
    };
    let recording_profile = state.store.recording_profile_by_id(None)?;
    let stream_destinations = state
        .store
        .list_stream_destinations()?
        .into_iter()
        .filter(|destination| destination.enabled)
        .collect::<Vec<_>>();
    let intent = if stream_destinations.is_empty() {
        PipelineIntent::Recording
    } else {
        PipelineIntent::RecordingAndStream
    };

    Ok(MediaPipelinePlanRequest {
        dry_run: true,
        intent,
        capture_sources,
        active_scene,
        recording_profile,
        stream_destinations,
    })
}

async fn plan_pipeline(state: &ApiState, request: MediaPipelinePlanRequest) -> MediaPipelinePlan {
    let plan = if let Some(runner) = &state.media_runner {
        match runner.plan_pipeline(request.clone()).await {
            Ok(plan) => plan,
            Err(error) => {
                let mut plan = build_dry_run_pipeline_plan(request);
                plan.warnings.push(format!(
                    "media runner plan unavailable; using local dry-run planner: {error}"
                ));
                plan
            }
        }
    } else {
        build_dry_run_pipeline_plan(request)
    };

    if let Err(error) = write_pipeline_files(state, &plan) {
        tracing::warn!(%error, "failed to write pipeline contract files");
    }

    plan
}

async fn refresh_default_pipeline_contract(state: &ApiState) {
    match default_pipeline_plan_request(state) {
        Ok(request) => {
            let _ = plan_pipeline(state, request).await;
        }
        Err(error) => tracing::warn!(%error, "failed to refresh default pipeline contract"),
    }
}

#[derive(Serialize)]
struct RunnerPipelineConfig<'a> {
    dry_run: bool,
    status_addr: Option<SocketAddr>,
    pipeline_name: &'a str,
    pipeline: &'a vaexcore_core::MediaPipelineConfig,
}

fn write_pipeline_files(state: &ApiState, plan: &MediaPipelinePlan) -> anyhow::Result<()> {
    if let Some(path) = &state.pipeline_plan_path {
        write_json_file(path, plan)?;
    }

    if let Some(path) = &state.pipeline_config_path {
        write_json_file(
            path,
            &RunnerPipelineConfig {
                dry_run: plan.config.dry_run,
                status_addr: state
                    .media_runner
                    .as_ref()
                    .map(MediaRunnerSupervisor::status_addr),
                pipeline_name: &plan.pipeline_name,
                pipeline: &plan.config,
            },
        )?;
    }

    Ok(())
}

fn write_json_file<T>(path: &FsPath, value: &T) -> anyhow::Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

pub fn generate_token() -> String {
    new_id("token")
}

pub fn default_auth_from_env() -> AuthConfig {
    AuthConfig {
        token: std::env::var("VAEXCORE_API_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| Some(generate_token())),
        dev_mode: cfg!(debug_assertions)
            || std::env::var("VAEXCORE_DEV_AUTH_BYPASS")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
    }
}

pub fn default_bind_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], vaexcore_core::DEFAULT_API_PORT))
}

pub fn default_database_path() -> PathBuf {
    directories::ProjectDirs::from("com", "vaexcore", "vaexcore studio")
        .map(|dirs| dirs.data_dir().join("studio.sqlite"))
        .unwrap_or_else(|| PathBuf::from("vaexcore studio.sqlite"))
}

fn suite_discovery_dir() -> PathBuf {
    vaexcore_shared_data_dir().join("suite")
}

fn vaexcore_shared_data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("AppData")
                    .join("Roaming")
            })
            .join("vaexcore");
    }

    if cfg!(target_os = "macos") {
        return std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
            .join("vaexcore");
    }

    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("share")
        })
        .join("vaexcore")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> Router {
        let state = ApiState::new_in_memory(AuthConfig {
            token: Some("test-token".to_string()),
            dev_mode: true,
        })
        .unwrap();
        router(state)
    }

    async fn response_body(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn request_json(
        app: Router,
        method: &str,
        uri: String,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut request = Request::builder().method(method).uri(uri);
        let body = match body {
            Some(body) => {
                request = request.header("content-type", "application/json");
                Body::from(body.to_string())
            }
            None => Body::empty(),
        };
        let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
        let status = response.status();
        (status, response_body(response).await)
    }

    async fn first_recording_profile_id(app: Router) -> String {
        let (status, body) = request_json(app, "GET", "/profiles".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);
        body["data"]["recording_profiles"][0]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    async fn first_stream_destination_id(app: Router) -> String {
        let (status, body) = request_json(app, "GET", "/profiles".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);
        body["data"]["stream_destinations"][0]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn health_check_returns_ok() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response_body(response).await;
        assert_eq!(body["ok"], true);
        assert_eq!(body["data"]["service"], APP_NAME);
    }

    #[tokio::test]
    async fn request_id_header_is_preserved() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header("x-vaexcore-request-id", "client.req-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("x-vaexcore-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("client.req-1")
        );
    }

    #[tokio::test]
    async fn request_id_header_is_generated() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let request_id = response
            .headers()
            .get("x-vaexcore-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap();
        assert!(request_id.starts_with("req_"));
    }

    #[tokio::test]
    async fn client_registry_records_request_headers() {
        let app = test_app();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .header("x-vaexcore-client-id", "test-client")
                    .header("x-vaexcore-client-name", "Test Client")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let (status, body) = request_json(app, "GET", "/clients".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);
        let clients = body["data"]["clients"].as_array().unwrap();
        assert!(clients.iter().any(|client| {
            client["id"].as_str() == Some("test-client")
                && client["name"].as_str() == Some("Test Client")
        }));
    }

    #[tokio::test]
    async fn command_audit_log_records_mutating_requests() {
        let app = test_app();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/recording/start")
                    .header("content-type", "application/json")
                    .header("x-vaexcore-request-id", "audit.req-1")
                    .header("x-vaexcore-client-name", "Audit Test Client")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let (status, body) = request_json(app, "GET", "/audit-log".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["entries"][0]["action"], "recording.start");
        assert_eq!(body["data"]["entries"][0]["request_id"], "audit.req-1");
        assert_eq!(
            body["data"]["entries"][0]["client_name"],
            "Audit Test Client"
        );
    }

    #[tokio::test]
    async fn recording_lifecycle_smoke_test() {
        let app = test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/recording/start")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_body(response).await;
        assert_eq!(body["data"]["status"]["recording_active"], true);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/recording/stop")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_body(response).await;
        assert_eq!(body["data"]["status"]["recording_active"], false);
    }

    #[tokio::test]
    async fn stream_lifecycle_smoke_test() {
        let app = test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/stream/start")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_body(response).await;
        assert_eq!(body["data"]["status"]["stream_active"], true);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/stream/stop")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_body(response).await;
        assert_eq!(body["data"]["status"]["stream_active"], false);
    }

    #[tokio::test]
    async fn marker_create_is_idempotent_and_listable() {
        let app = test_app();
        let marker_body = json!({
            "label": "Pulse keep: opener",
            "source_app": "vaexcore-pulse",
            "source_event_id": "pulse:session:candidate",
            "recording_session_id": "rec_123",
            "media_path": "/tmp/recording.mkv",
            "start_seconds": 12.5,
            "end_seconds": 24.0,
            "metadata": {
                "confidenceBand": "high"
            }
        });

        let (status, first) = request_json(
            app.clone(),
            "POST",
            "/marker/create".to_string(),
            Some(marker_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let first_id = first["data"]["id"].as_str().unwrap().to_string();

        let (status, duplicate) = request_json(
            app.clone(),
            "POST",
            "/marker/create".to_string(),
            Some(json!({
                "label": "Pulse keep: duplicate",
                "source_app": "vaexcore-pulse",
                "source_event_id": "pulse:session:candidate",
                "recording_session_id": "rec_123"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(duplicate["data"]["id"].as_str().unwrap(), first_id);
        assert_eq!(duplicate["data"]["label"], "Pulse keep: opener");

        let (status, markers) = request_json(
            app.clone(),
            "GET",
            "/markers?source_app=vaexcore-pulse".to_string(),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(markers["data"]["markers"].as_array().unwrap().len(), 1);
        assert_eq!(markers["data"]["markers"][0]["id"], first_id);

        let (status, status_body) = request_json(app, "GET", "/status".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);
        let marker_events = status_body["data"]["recent_events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event["type"] == "marker.created")
            .count();
        assert_eq!(marker_events, 1);
    }

    #[tokio::test]
    async fn default_pipeline_plan_smoke_test() {
        let app = test_app();

        let (status, body) = request_json(app, "GET", "/media/plan".to_string(), None).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["dry_run"], true);
        assert!(body["data"]["steps"].as_array().unwrap().len() >= 2);
        assert!(body["data"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["id"] == serde_json::json!("capture.frames")));
        assert!(body["data"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["id"] == serde_json::json!("audio.mixer")));
        assert!(body["data"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["id"] == serde_json::json!("scene.render_runtime")));
        assert!(body["data"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["id"] == serde_json::json!("scene.software_renderer")));
        assert_eq!(
            body["data"]["config"]["active_scene"]["id"],
            serde_json::json!("scene-main")
        );
        assert_eq!(
            body["data"]["config"]["compositor_graph"]["scene_id"],
            serde_json::json!("scene-main")
        );
        assert_eq!(
            body["data"]["config"]["capture_frame_plan"]["scene_id"],
            serde_json::json!("scene-main")
        );
        assert_eq!(
            body["data"]["config"]["audio_mixer_plan"]["scene_id"],
            serde_json::json!("scene-main")
        );
        assert_eq!(
            body["data"]["config"]["compositor_render_plan"]["graph"]["scene_id"],
            serde_json::json!("scene-main")
        );
    }

    #[tokio::test]
    async fn scene_collection_api_round_trip() {
        let app = test_app();
        let (status, body) = request_json(app.clone(), "GET", "/scenes".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);

        let mut collection = body["data"].clone();
        collection["name"] = serde_json::json!("API Scenes");
        collection["scenes"][0]["name"] = serde_json::json!("API Main");

        let (status, saved) =
            request_json(app.clone(), "PUT", "/scenes".to_string(), Some(collection)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(saved["data"]["name"], "API Scenes");

        let (status, validation) = request_json(
            app,
            "POST",
            "/scenes/validate".to_string(),
            Some(saved["data"].clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(validation["data"]["ok"], true);
    }

    #[tokio::test]
    async fn default_pipeline_plan_writes_contract_files() {
        let temp = tempfile::tempdir().unwrap();
        let database_path = temp.path().join("studio.sqlite");
        let plan_path = temp.path().join("pipeline-plan.json");
        let config_path = temp.path().join("pipeline-config.json");
        let state = ApiState::new(&ApiServerConfig {
            bind_addr: default_bind_addr(),
            database_path,
            auth: SharedAuthConfig::new(AuthConfig {
                token: Some("test-token".to_string()),
                dev_mode: true,
            }),
            media_runner: None,
            pipeline_plan_path: Some(plan_path.clone()),
            pipeline_config_path: Some(config_path.clone()),
        })
        .unwrap();
        state
            .store
            .initialize_app_settings(vaexcore_core::AppSettings::default())
            .unwrap();

        let request = default_pipeline_plan_request(&state).unwrap();
        let plan = plan_pipeline(&state, request).await;

        let plan_json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&plan_path).unwrap()).unwrap();
        let config_json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();

        assert_eq!(plan_json["pipeline_name"], plan.pipeline_name);
        assert_eq!(config_json["pipeline_name"], plan.pipeline_name);
        assert_eq!(config_json["dry_run"], true);
        assert_eq!(config_json["pipeline"]["version"], 1);
        assert_eq!(config_json["pipeline"]["active_scene"]["id"], "scene-main");
        assert_eq!(
            config_json["pipeline"]["compositor_graph"]["scene_id"],
            "scene-main"
        );
        assert_eq!(
            config_json["pipeline"]["capture_frame_plan"]["scene_id"],
            "scene-main"
        );
        assert_eq!(
            config_json["pipeline"]["audio_mixer_plan"]["scene_id"],
            "scene-main"
        );
        assert_eq!(
            config_json["pipeline"]["compositor_render_plan"]["targets"][0]["kind"],
            "preview"
        );
        assert!(config_json["pipeline"]["capture_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["id"].as_str() == Some("display:main")));
    }

    #[tokio::test]
    async fn pipeline_validation_blocks_missing_capture_sources() {
        let app = test_app();
        let profile_id = first_recording_profile_id(app.clone()).await;
        let (status, profiles) =
            request_json(app.clone(), "GET", "/profiles".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);
        let profile = profiles["data"]["recording_profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|profile| profile["id"].as_str() == Some(profile_id.as_str()))
            .unwrap()
            .clone();

        let (status, body) = request_json(
            app,
            "POST",
            "/media/validate".to_string(),
            Some(json!({
                "dry_run": true,
                "intent": "recording",
                "capture_sources": [],
                "recording_profile": profile,
                "stream_destinations": []
            })),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["ready"], false);
    }

    #[tokio::test]
    async fn recording_profile_update_and_delete_smoke_test() {
        let app = test_app();
        let profile_id = first_recording_profile_id(app.clone()).await;

        let (status, body) = request_json(
            app.clone(),
            "PUT",
            format!("/profiles/recording/{profile_id}"),
            Some(json!({
                "name": "Updated Local Profile",
                "output_folder": "~/Movies/updated",
                "filename_pattern": "updated-{time}",
                "container": "mp4",
                "resolution": { "width": 1280, "height": 720 },
                "framerate": 30,
                "bitrate_kbps": 6000,
                "encoder_preference": "hardware"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["kind"], "recording_profile");
        assert_eq!(body["data"]["value"]["name"], "Updated Local Profile");
        assert_eq!(body["data"]["value"]["container"], "mp4");

        let (status, body) = request_json(
            app.clone(),
            "DELETE",
            format!("/profiles/recording/{profile_id}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["deleted"], true);

        let (status, body) = request_json(app, "GET", "/profiles".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);
        let still_present = body["data"]["recording_profiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|profile| profile["id"].as_str() == Some(profile_id.as_str()));
        assert!(!still_present);
    }

    #[tokio::test]
    async fn stream_destination_update_and_delete_smoke_test() {
        let app = test_app();
        let destination_id = first_stream_destination_id(app.clone()).await;

        let (status, body) = request_json(
            app.clone(),
            "PUT",
            format!("/profiles/destinations/{destination_id}"),
            Some(json!({
                "name": "Updated RTMPS Destination",
                "platform": "custom_rtmp",
                "ingest_url": "rtmps://example.test/live",
                "stream_key": "test-stream-key",
                "enabled": false
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["kind"], "stream_destination");
        assert_eq!(body["data"]["value"]["name"], "Updated RTMPS Destination");
        assert_eq!(body["data"]["value"]["enabled"], false);
        assert!(body["data"]["value"]["stream_key_ref"].is_object());

        let (status, body) = request_json(
            app.clone(),
            "DELETE",
            format!("/profiles/destinations/{destination_id}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["deleted"], true);

        let (status, body) = request_json(app, "GET", "/profiles".to_string(), None).await;
        assert_eq!(status, StatusCode::OK);
        let still_present = body["data"]["stream_destinations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|destination| destination["id"].as_str() == Some(destination_id.as_str()));
        assert!(!still_present);
    }

    #[tokio::test]
    async fn deleting_active_recording_profile_conflicts() {
        let app = test_app();
        let profile_id = first_recording_profile_id(app.clone()).await;

        let (status, _) = request_json(
            app.clone(),
            "POST",
            "/recording/start".to_string(),
            Some(json!({ "profile_id": profile_id.clone() })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = request_json(
            app,
            "DELETE",
            format!("/profiles/recording/{profile_id}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"]["code"], "recording_profile_in_use");
    }

    #[tokio::test]
    async fn deleting_active_stream_destination_conflicts() {
        let app = test_app();
        let destination_id = first_stream_destination_id(app.clone()).await;

        let (status, _) = request_json(
            app.clone(),
            "POST",
            "/stream/start".to_string(),
            Some(json!({ "destination_id": destination_id.clone() })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = request_json(
            app,
            "DELETE",
            format!("/profiles/destinations/{destination_id}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"]["code"], "stream_destination_in_use");
    }

    #[tokio::test]
    async fn live_auth_updates_are_used() {
        let state = ApiState::new_in_memory(AuthConfig {
            token: Some("old-token".to_string()),
            dev_mode: false,
        })
        .unwrap();
        let app = router(state.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .header("x-vaexcore-token", "old-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        state.auth.update(AuthConfig {
            token: Some("new-token".to_string()),
            dev_mode: false,
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .header("x-vaexcore-token", "old-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .header("x-vaexcore-token", "new-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn events_query_auth_rejects_missing_token() {
        let auth = SharedAuthConfig::new(AuthConfig {
            token: Some("test-token".to_string()),
            dev_mode: false,
        });

        let result = auth::authorize_query(
            &auth::TokenQuery {
                token: None,
                limit: Some(1),
                ..Default::default()
            },
            &auth,
        );
        assert!(result.is_err());
    }

    #[test]
    fn events_query_auth_accepts_token_with_replay_limit() {
        let auth = SharedAuthConfig::new(AuthConfig {
            token: Some("test-token".to_string()),
            dev_mode: false,
        });

        auth::authorize_query(
            &auth::TokenQuery {
                token: Some("test-token".to_string()),
                limit: Some(1),
                ..Default::default()
            },
            &auth,
        )
        .unwrap();
    }
}
