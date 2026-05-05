use std::{env, io::Read, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use vaexcore_core::{
    ApiResponse, EngineMode, EngineStatus, MediaPipelineConfig, MediaPipelinePlan,
    MediaPipelinePlanRequest, MediaPipelineValidation, MediaProfile, RecordingSession,
    StreamSession, APP_NAME,
};
use vaexcore_media::{
    build_dry_run_pipeline_plan, find_ffmpeg_binary, DryRunMediaEngine, FfmpegRtmpEngine,
    MediaEngine, MediaError, MediaTransition, StreamLaunchRequest,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunnerConfig {
    #[serde(default = "default_true")]
    pub dry_run: bool,
    pub status_addr: Option<SocketAddr>,
    #[serde(default)]
    pub pipeline_name: Option<String>,
    #[serde(default)]
    pub pipeline: Option<MediaPipelineConfig>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            dry_run: true,
            status_addr: None,
            pipeline_name: Some("dry-run".to_string()),
            pipeline: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunnerStatus {
    pub service: String,
    pub ready: bool,
    pub dry_run: bool,
    pub pipeline_name: Option<String>,
    pub engine_status: EngineStatus,
}

#[derive(Clone)]
struct RunnerState {
    config: RunnerConfig,
    engine: Arc<dyn MediaEngine>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "media_runner=info,vaexcore=info".into()),
        )
        .init();

    let cli = CliArgs::parse(env::args().skip(1))?;
    let mut config = read_config(cli.config_path)?;

    if cli.dry_run {
        config.dry_run = true;
    }

    if let Some(status_addr) = cli.status_addr {
        config.status_addr = Some(status_addr);
    }

    let state = RunnerState {
        config: config.clone(),
        engine: media_engine_for_config(&config),
    };

    if let Some(addr) = config.status_addr {
        let app = Router::new()
            .route("/health", get(health))
            .route("/status", get(status))
            .route("/recording/start", post(start_recording))
            .route("/recording/stop", post(stop_recording))
            .route("/stream/start", post(start_stream))
            .route("/stream/stop", post(stop_stream))
            .route("/plan", post(plan_pipeline))
            .route("/validate", post(validate_pipeline))
            .with_state(state);

        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!(%local_addr, dry_run = config.dry_run, "media-runner status endpoint listening");
        axum::serve(listener, app).await?;
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&ApiResponse::ok(status_from_config(&config)))?
        );
    }

    Ok(())
}

async fn health() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "service": "vaexcore-media-runner",
        "ok": true
    })))
}

async fn status(State(state): State<RunnerState>) -> Json<ApiResponse<RunnerStatus>> {
    Json(ApiResponse::ok(runner_status(&state).await))
}

async fn start_recording(
    State(state): State<RunnerState>,
    Json(profile): Json<MediaProfile>,
) -> Result<Json<ApiResponse<MediaTransition<RecordingSession>>>, RunnerApiError> {
    let transition = state.engine.start_recording(profile).await?;
    Ok(Json(ApiResponse::ok(transition)))
}

async fn stop_recording(
    State(state): State<RunnerState>,
) -> Result<Json<ApiResponse<MediaTransition<RecordingSession>>>, RunnerApiError> {
    let transition = state.engine.stop_recording().await?;
    Ok(Json(ApiResponse::ok(transition)))
}

async fn start_stream(
    State(state): State<RunnerState>,
    Json(request): Json<StreamLaunchRequest>,
) -> Result<Json<ApiResponse<MediaTransition<StreamSession>>>, RunnerApiError> {
    let transition = state.engine.start_stream(request).await?;
    Ok(Json(ApiResponse::ok(transition)))
}

async fn stop_stream(
    State(state): State<RunnerState>,
) -> Result<Json<ApiResponse<MediaTransition<StreamSession>>>, RunnerApiError> {
    let transition = state.engine.stop_stream().await?;
    Ok(Json(ApiResponse::ok(transition)))
}

async fn plan_pipeline(
    Json(request): Json<MediaPipelinePlanRequest>,
) -> Json<ApiResponse<MediaPipelinePlan>> {
    Json(ApiResponse::ok(build_dry_run_pipeline_plan(request)))
}

async fn validate_pipeline(
    Json(request): Json<MediaPipelinePlanRequest>,
) -> Json<ApiResponse<MediaPipelineValidation>> {
    Json(ApiResponse::ok(
        build_dry_run_pipeline_plan(request).validation(),
    ))
}

#[derive(Debug)]
struct RunnerApiError {
    status: StatusCode,
    code: String,
    message: String,
}

impl From<MediaError> for RunnerApiError {
    fn from(error: MediaError) -> Self {
        let status = match error {
            MediaError::InvalidCommand(_) => StatusCode::BAD_REQUEST,
            MediaError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        };
        Self {
            status,
            code: "media_error".to_string(),
            message: error.to_string(),
        }
    }
}

impl IntoResponse for RunnerApiError {
    fn into_response(self) -> Response {
        let body: ApiResponse<serde_json::Value> = ApiResponse::error(self.code, self.message);
        (self.status, Json(body)).into_response()
    }
}

#[derive(Debug, Default)]
struct CliArgs {
    config_path: Option<PathBuf>,
    status_addr: Option<SocketAddr>,
    dry_run: bool,
}

impl CliArgs {
    fn parse(args: impl Iterator<Item = String>) -> anyhow::Result<Self> {
        let mut parsed = Self::default();
        let mut args = args.peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--config requires a path"))?;
                    parsed.config_path = Some(PathBuf::from(value));
                }
                "--status-addr" => {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--status-addr requires host:port"))?;
                    parsed.status_addr = Some(value.parse()?);
                }
                "--dry-run" => parsed.dry_run = true,
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                other => anyhow::bail!("unknown argument '{other}'"),
            }
        }

        Ok(parsed)
    }
}

fn read_config(path: Option<PathBuf>) -> anyhow::Result<RunnerConfig> {
    let content = match path {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let mut content = String::new();
            std::io::stdin().read_to_string(&mut content)?;
            content
        }
    };

    if content.trim().is_empty() {
        return Ok(RunnerConfig::default());
    }

    Ok(serde_json::from_str(&content)?)
}

fn status_from_config(config: &RunnerConfig) -> RunnerStatus {
    RunnerStatus {
        service: "vaexcore-media-runner".to_string(),
        ready: true,
        dry_run: config.dry_run,
        pipeline_name: config.pipeline_name.clone(),
        engine_status: EngineStatus::idle(
            "media-runner",
            if config.dry_run {
                EngineMode::DryRun
            } else {
                EngineMode::ExternalSidecar
            },
        ),
    }
}

async fn runner_status(state: &RunnerState) -> RunnerStatus {
    let mut engine_status = state.engine.status().await;
    engine_status.engine = "media-runner".to_string();
    engine_status.mode = if state.config.dry_run {
        EngineMode::DryRun
    } else {
        EngineMode::ExternalSidecar
    };

    RunnerStatus {
        service: "vaexcore-media-runner".to_string(),
        ready: true,
        dry_run: state.config.dry_run,
        pipeline_name: state.config.pipeline_name.clone(),
        engine_status,
    }
}

fn media_engine_for_config(config: &RunnerConfig) -> Arc<dyn MediaEngine> {
    if config.dry_run {
        return Arc::new(DryRunMediaEngine::new(None));
    }

    Arc::new(FfmpegRtmpEngine::new(find_ffmpeg_binary(), None))
}

fn default_true() -> bool {
    true
}

fn print_help() {
    println!(
        "{APP_NAME} media-runner\n\nUSAGE:\n  media-runner [--config path] [--status-addr 127.0.0.1:51387] [--dry-run]\n\nJSON config can be passed through --config or stdin."
    );
}
