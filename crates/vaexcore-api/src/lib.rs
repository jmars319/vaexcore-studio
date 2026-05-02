mod auth;
mod event_bus;
mod store;

use std::{future::Future, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{net::TcpListener, sync::broadcast};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use vaexcore_core::{
    new_id, ApiResponse, CommandStatus, HealthResponse, Marker, MediaProfileInput,
    ProfilesSnapshot, StreamDestinationInput, StudioEvent, StudioEventKind, StudioStatus, APP_NAME,
};
use vaexcore_media::{
    DryRunMediaEngine, MediaEngine, MediaError, MediaRunnerSupervisor, SidecarMediaEngine,
};

pub use auth::{AuthConfig, SharedAuthConfig};
pub use event_bus::EventBus;
pub use store::{ProfileStore, StoreError};

#[derive(Clone, Debug)]
pub struct ApiServerConfig {
    pub bind_addr: SocketAddr,
    pub database_path: PathBuf,
    pub auth: SharedAuthConfig,
    pub media_runner: Option<MediaRunnerSupervisor>,
}

pub struct ApiState {
    pub auth: SharedAuthConfig,
    pub store: ProfileStore,
    pub engine: Arc<dyn MediaEngine>,
    pub events: EventBus,
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
            engine,
            events,
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
            engine,
            events,
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
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
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
                }
                Ok(status) => {
                    if ready {
                        events.emit(StudioEvent::error(format!(
                            "media runner reported not ready: {}",
                            status.service
                        )));
                    }
                    ready = false;
                }
                Err(error) => {
                    if ready {
                        events.emit(StudioEvent::error(format!(
                            "media runner unavailable: {error}"
                        )));
                    }
                    ready = false;
                }
            }
        }
    });
}

pub async fn serve(config: ApiServerConfig) -> anyhow::Result<()> {
    serve_with_shutdown(config, std::future::pending::<()>()).await
}

pub async fn serve_with_shutdown<F>(config: ApiServerConfig, shutdown: F) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind(config.bind_addr).await?;
    let local_addr = listener.local_addr()?;
    let state = ApiState::new(&config)?;

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
        .route("/recording/start", post(start_recording))
        .route("/recording/stop", post(stop_recording))
        .route("/stream/start", post(start_stream))
        .route("/stream/stop", post(stop_stream))
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
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CreateMarkerRequest {
    pub label: Option<String>,
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
    }))
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

    match state.engine.start_recording(profile).await {
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
        Ok(transition) => Ok(Json(ApiResponse::ok(CommandStatus {
            changed: transition.changed,
            message: if transition.changed {
                "recording stopped"
            } else {
                "recording already stopped"
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

    match state.engine.start_stream(destination).await {
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

async fn create_marker(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    payload: Option<Json<CreateMarkerRequest>>,
) -> Result<Json<ApiResponse<Marker>>, ApiError> {
    auth::authorize_headers(&headers, &state.auth)?;
    let request = payload.map(|Json(payload)| payload).unwrap_or_default();
    let marker = state.store.create_marker(request.label)?;
    state.events.emit(StudioEvent::new(
        StudioEventKind::MarkerCreated,
        json!({
            "marker_id": marker.id,
            "label": marker.label,
            "created_at": marker.created_at,
        }),
    ));

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

    websocket
        .on_upgrade(move |socket| stream_events(socket, state))
        .into_response()
}

async fn stream_events(mut socket: WebSocket, state: Arc<ApiState>) {
    for event in state.events.recent() {
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
}
