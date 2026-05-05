use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    env, fmt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};
use tokio::sync::Mutex;
use vaexcore_core::{
    new_id, CaptureSourceKind, EngineMode, EngineStatus, MediaPipelineConfig, MediaPipelinePlan,
    MediaPipelinePlanRequest, MediaPipelineStep, MediaProfile, PipelineIntent, PipelineStepStatus,
    PlatformKind, RecordingContainer, RecordingSession, StreamDestination, StreamSession,
    StudioEvent, StudioEventKind,
};

mod sidecar;

pub use sidecar::{
    MediaRunnerConfig, MediaRunnerStatus, MediaRunnerSupervisor, SidecarError, SidecarMediaEngine,
};

pub type MediaEventSink = Arc<dyn Fn(StudioEvent) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("{0}")]
    InvalidCommand(String),
    #[error("media engine is unavailable: {0}")]
    Unavailable(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MediaTransition<T> {
    pub changed: bool,
    pub session: Option<T>,
    pub status: EngineStatus,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct StreamLaunchRequest {
    pub destination: StreamDestination,
    #[serde(default)]
    pub stream_key: Option<String>,
    #[serde(default)]
    pub bandwidth_test: bool,
}

impl fmt::Debug for StreamLaunchRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StreamLaunchRequest")
            .field("destination", &self.destination)
            .field(
                "stream_key",
                &self.stream_key.as_ref().map(|_| "[redacted]"),
            )
            .field("bandwidth_test", &self.bandwidth_test)
            .finish()
    }
}

impl StreamLaunchRequest {
    pub fn new(destination: StreamDestination) -> Self {
        Self {
            destination,
            stream_key: None,
            bandwidth_test: false,
        }
    }
}

pub fn build_dry_run_pipeline_plan(request: MediaPipelinePlanRequest) -> MediaPipelinePlan {
    let config = request.into_config();
    let mut steps = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    validate_capture_sources(&config, &mut steps, &mut warnings, &mut errors);
    validate_recording(&config, &mut steps, &mut errors);
    validate_streaming(&config, &mut steps, &mut warnings, &mut errors);
    steps.push(MediaPipelineStep {
        id: "engine.dry_run".to_string(),
        label: "Dry-run engine".to_string(),
        status: PipelineStepStatus::Ready,
        detail: "Pipeline will be simulated without starting a real capture backend.".to_string(),
    });

    MediaPipelinePlan {
        pipeline_name: pipeline_name(&config),
        dry_run: config.dry_run,
        ready: errors.is_empty(),
        config,
        steps,
        warnings,
        errors,
    }
}

#[async_trait]
pub trait MediaEngine: Send + Sync {
    async fn start_recording(
        &self,
        profile: MediaProfile,
    ) -> Result<MediaTransition<RecordingSession>, MediaError>;

    async fn stop_recording(&self) -> Result<MediaTransition<RecordingSession>, MediaError>;

    async fn start_stream(
        &self,
        request: StreamLaunchRequest,
    ) -> Result<MediaTransition<StreamSession>, MediaError>;

    async fn stop_stream(&self) -> Result<MediaTransition<StreamSession>, MediaError>;

    async fn status(&self) -> EngineStatus;
}

#[derive(Clone, Debug, Default)]
struct DryRunState {
    recording: Option<RecordingSession>,
    stream: Option<StreamSession>,
}

#[derive(Clone)]
pub struct DryRunMediaEngine {
    state: Arc<Mutex<DryRunState>>,
    event_sink: Option<MediaEventSink>,
}

impl DryRunMediaEngine {
    pub fn new(event_sink: Option<MediaEventSink>) -> Self {
        Self {
            state: Arc::new(Mutex::new(DryRunState::default())),
            event_sink,
        }
    }

    fn emit(&self, event: StudioEvent) {
        if let Some(sink) = &self.event_sink {
            sink(event);
        }
    }

    fn status_from_state(state: &DryRunState) -> EngineStatus {
        EngineStatus {
            engine: "DryRunMediaEngine".to_string(),
            mode: EngineMode::DryRun,
            recording: state.recording.clone(),
            stream: state.stream.clone(),
            recording_active: state.recording.is_some(),
            stream_active: state.stream.is_some(),
            recording_path: state
                .recording
                .as_ref()
                .map(|session| session.output_path.clone()),
            active_destination: state
                .stream
                .as_ref()
                .map(|session| session.destination.clone()),
            updated_at: Utc::now(),
        }
    }
}

#[async_trait]
impl MediaEngine for DryRunMediaEngine {
    async fn start_recording(
        &self,
        profile: MediaProfile,
    ) -> Result<MediaTransition<RecordingSession>, MediaError> {
        let mut state = self.state.lock().await;

        if let Some(existing) = state.recording.clone() {
            return Ok(MediaTransition {
                changed: false,
                session: Some(existing),
                status: Self::status_from_state(&state),
            });
        }

        let session = RecordingSession {
            id: new_id("recording"),
            output_path: build_output_path(&profile),
            profile,
            started_at: Utc::now(),
        };

        state.recording = Some(session.clone());
        let status = Self::status_from_state(&state);
        drop(state);

        self.emit(StudioEvent::new(
            StudioEventKind::RecordingStarted,
            json!({
                "session_id": session.id,
                "output_path": session.output_path,
                "profile_id": session.profile.id,
            }),
        ));

        Ok(MediaTransition {
            changed: true,
            session: Some(session),
            status,
        })
    }

    async fn stop_recording(&self) -> Result<MediaTransition<RecordingSession>, MediaError> {
        let mut state = self.state.lock().await;
        let stopped = state.recording.take();
        let status = Self::status_from_state(&state);
        drop(state);

        if let Some(session) = stopped.clone() {
            self.emit(StudioEvent::new(
                StudioEventKind::RecordingStopped,
                json!({
                    "session_id": session.id,
                    "output_path": session.output_path,
                    "profile_id": session.profile.id,
                }),
            ));
        }

        Ok(MediaTransition {
            changed: stopped.is_some(),
            session: stopped,
            status,
        })
    }

    async fn start_stream(
        &self,
        request: StreamLaunchRequest,
    ) -> Result<MediaTransition<StreamSession>, MediaError> {
        let destination = request.destination;
        if destination.ingest_url.trim().is_empty() {
            return Err(MediaError::InvalidCommand(
                "stream destination requires an ingest URL".to_string(),
            ));
        }

        let mut state = self.state.lock().await;

        if let Some(existing) = state.stream.clone() {
            return Ok(MediaTransition {
                changed: false,
                session: Some(existing),
                status: Self::status_from_state(&state),
            });
        }

        let session = StreamSession {
            id: new_id("stream"),
            destination,
            started_at: Utc::now(),
        };

        state.stream = Some(session.clone());
        let status = Self::status_from_state(&state);
        drop(state);

        self.emit(StudioEvent::new(
            StudioEventKind::StreamStarted,
            json!({
                "session_id": session.id,
                "destination_id": session.destination.id,
                "destination_name": session.destination.name,
                "platform": session.destination.platform,
            }),
        ));

        Ok(MediaTransition {
            changed: true,
            session: Some(session),
            status,
        })
    }

    async fn stop_stream(&self) -> Result<MediaTransition<StreamSession>, MediaError> {
        let mut state = self.state.lock().await;
        let stopped = state.stream.take();
        let status = Self::status_from_state(&state);
        drop(state);

        if let Some(session) = stopped.clone() {
            self.emit(StudioEvent::new(
                StudioEventKind::StreamStopped,
                json!({
                    "session_id": session.id,
                    "destination_id": session.destination.id,
                    "destination_name": session.destination.name,
                    "platform": session.destination.platform,
                }),
            ));
        }

        Ok(MediaTransition {
            changed: stopped.is_some(),
            session: stopped,
            status,
        })
    }

    async fn status(&self) -> EngineStatus {
        let state = self.state.lock().await;
        Self::status_from_state(&state)
    }
}

fn build_output_path(profile: &MediaProfile) -> String {
    let timestamp = Utc::now();
    let date = timestamp.format("%Y-%m-%d").to_string();
    let time = timestamp.format("%H-%M-%S").to_string();
    let extension = match profile.container {
        RecordingContainer::Mkv => "mkv",
        RecordingContainer::Mp4 => "mp4",
    };
    let filename = profile
        .filename_pattern
        .replace("{date}", &date)
        .replace("{time}", &time)
        .replace("{profile}", &slug(&profile.name));

    format!("{}/{}.{}", profile.output_folder, filename, extension)
}

fn validate_capture_sources(
    config: &MediaPipelineConfig,
    steps: &mut Vec<MediaPipelineStep>,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    let enabled_sources = config
        .capture_sources
        .iter()
        .filter(|source| source.enabled)
        .collect::<Vec<_>>();

    if enabled_sources.is_empty() {
        errors.push("at least one capture source must be enabled".to_string());
        steps.push(MediaPipelineStep {
            id: "capture.sources".to_string(),
            label: "Capture sources".to_string(),
            status: PipelineStepStatus::Blocked,
            detail: "No enabled capture sources were provided.".to_string(),
        });
        return;
    }

    let has_video = enabled_sources.iter().any(|source| {
        matches!(
            source.kind,
            CaptureSourceKind::Display | CaptureSourceKind::Window | CaptureSourceKind::Camera
        )
    });
    if !has_video {
        warnings.push("pipeline has audio sources but no video source".to_string());
    }

    steps.push(MediaPipelineStep {
        id: "capture.sources".to_string(),
        label: "Capture sources".to_string(),
        status: if has_video {
            PipelineStepStatus::Ready
        } else {
            PipelineStepStatus::Warning
        },
        detail: format!("{} enabled source(s).", enabled_sources.len()),
    });
}

fn validate_recording(
    config: &MediaPipelineConfig,
    steps: &mut Vec<MediaPipelineStep>,
    errors: &mut Vec<String>,
) {
    let needs_recording = matches!(
        config.intent,
        PipelineIntent::Recording | PipelineIntent::RecordingAndStream
    );
    if !needs_recording {
        return;
    }

    let Some(profile) = &config.recording_profile else {
        errors.push("recording intent requires a recording profile".to_string());
        steps.push(MediaPipelineStep {
            id: "recording.profile".to_string(),
            label: "Recording profile".to_string(),
            status: PipelineStepStatus::Blocked,
            detail: "No recording profile was provided.".to_string(),
        });
        return;
    };

    let mut local_errors = Vec::new();
    if profile.output_folder.trim().is_empty() {
        local_errors.push("output folder is empty");
    }
    if profile.filename_pattern.trim().is_empty() {
        local_errors.push("filename pattern is empty");
    }
    if profile.resolution.width == 0 || profile.resolution.height == 0 {
        local_errors.push("resolution is invalid");
    }
    if profile.framerate == 0 {
        local_errors.push("framerate is invalid");
    }
    if profile.bitrate_kbps == 0 {
        local_errors.push("bitrate is invalid");
    }

    if local_errors.is_empty() {
        steps.push(MediaPipelineStep {
            id: "recording.profile".to_string(),
            label: "Recording profile".to_string(),
            status: PipelineStepStatus::Ready,
            detail: format!(
                "{} {}x{} {}fps {}kbps",
                profile.name,
                profile.resolution.width,
                profile.resolution.height,
                profile.framerate,
                profile.bitrate_kbps
            ),
        });
    } else {
        errors.extend(local_errors.into_iter().map(str::to_string));
        steps.push(MediaPipelineStep {
            id: "recording.profile".to_string(),
            label: "Recording profile".to_string(),
            status: PipelineStepStatus::Blocked,
            detail: "Recording profile has invalid required fields.".to_string(),
        });
    }
}

fn validate_streaming(
    config: &MediaPipelineConfig,
    steps: &mut Vec<MediaPipelineStep>,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    let needs_stream = matches!(
        config.intent,
        PipelineIntent::Stream | PipelineIntent::RecordingAndStream
    );
    if !needs_stream {
        return;
    }

    let enabled_destinations = config
        .stream_destinations
        .iter()
        .filter(|destination| destination.enabled)
        .collect::<Vec<_>>();
    if enabled_destinations.is_empty() {
        errors.push("stream intent requires at least one enabled destination".to_string());
        steps.push(MediaPipelineStep {
            id: "stream.destinations".to_string(),
            label: "Stream destinations".to_string(),
            status: PipelineStepStatus::Blocked,
            detail: "No enabled stream destinations were provided.".to_string(),
        });
        return;
    }

    let missing_ingest = enabled_destinations
        .iter()
        .filter(|destination| destination.ingest_url.trim().is_empty())
        .count();
    let missing_keys = enabled_destinations
        .iter()
        .filter(|destination| destination.stream_key_ref.is_none())
        .count();

    if missing_ingest > 0 {
        errors.push(format!(
            "{missing_ingest} enabled stream destination(s) are missing ingest URLs"
        ));
    }
    if missing_keys > 0 {
        warnings.push(format!(
            "{missing_keys} enabled stream destination(s) do not have stored stream keys"
        ));
    }

    steps.push(MediaPipelineStep {
        id: "stream.destinations".to_string(),
        label: "Stream destinations".to_string(),
        status: if missing_ingest > 0 {
            PipelineStepStatus::Blocked
        } else if missing_keys > 0 {
            PipelineStepStatus::Warning
        } else {
            PipelineStepStatus::Ready
        },
        detail: format!("{} enabled destination(s).", enabled_destinations.len()),
    });
}

fn pipeline_name(config: &MediaPipelineConfig) -> String {
    let intent = match config.intent {
        PipelineIntent::Recording => "recording",
        PipelineIntent::Stream => "stream",
        PipelineIntent::RecordingAndStream => "recording-stream",
    };
    format!("dry-run-{intent}-v{}", config.version)
}

fn slug(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }

    slug.trim_matches('-').to_string()
}

#[derive(Clone)]
pub struct FfmpegRtmpEngine {
    inner: Arc<FfmpegRtmpEngineInner>,
}

struct FfmpegRtmpEngineInner {
    stream: StdMutex<Option<FfmpegActiveStream>>,
    recording_engine: DryRunMediaEngine,
    ffmpeg_path: Option<PathBuf>,
    event_sink: Option<MediaEventSink>,
}

struct FfmpegActiveStream {
    session: StreamSession,
    child: Child,
}

impl FfmpegRtmpEngine {
    pub fn new(ffmpeg_path: Option<PathBuf>, event_sink: Option<MediaEventSink>) -> Self {
        Self {
            inner: Arc::new(FfmpegRtmpEngineInner {
                stream: StdMutex::new(None),
                recording_engine: DryRunMediaEngine::new(event_sink.clone()),
                ffmpeg_path,
                event_sink,
            }),
        }
    }

    fn emit(&self, event: StudioEvent) {
        if let Some(sink) = &self.inner.event_sink {
            sink(event);
        }
    }

    async fn status_from_state(&self) -> EngineStatus {
        let recording_status = self.inner.recording_engine.status().await;
        let stream = self
            .inner
            .stream
            .lock()
            .expect("ffmpeg stream state mutex poisoned")
            .as_ref()
            .map(|active| active.session.clone());
        EngineStatus {
            engine: if self.inner.ffmpeg_path.is_some() {
                "FfmpegRtmpEngine".to_string()
            } else {
                "FfmpegRtmpEngine unavailable".to_string()
            },
            mode: EngineMode::ExternalSidecar,
            recording: recording_status.recording,
            stream: stream.clone(),
            recording_active: recording_status.recording_active,
            stream_active: stream.is_some(),
            recording_path: recording_status.recording_path,
            active_destination: stream.map(|session| session.destination),
            updated_at: Utc::now(),
        }
    }
}

#[async_trait]
impl MediaEngine for FfmpegRtmpEngine {
    async fn start_recording(
        &self,
        profile: MediaProfile,
    ) -> Result<MediaTransition<RecordingSession>, MediaError> {
        self.inner.recording_engine.start_recording(profile).await
    }

    async fn stop_recording(&self) -> Result<MediaTransition<RecordingSession>, MediaError> {
        self.inner.recording_engine.stop_recording().await
    }

    async fn start_stream(
        &self,
        request: StreamLaunchRequest,
    ) -> Result<MediaTransition<StreamSession>, MediaError> {
        if request.destination.ingest_url.trim().is_empty() {
            return Err(MediaError::InvalidCommand(
                "stream destination requires an ingest URL".to_string(),
            ));
        }

        let existing_session = {
            let mut state = self
                .inner
                .stream
                .lock()
                .expect("ffmpeg stream state mutex poisoned");
            if let Some(active) = state.as_mut() {
                match active.child.try_wait() {
                    Ok(None) => Some(active.session.clone()),
                    Ok(Some(status)) => {
                        let session = active.session.clone();
                        *state = None;
                        return Err(MediaError::Unavailable(format!(
                            "ffmpeg exited before the stream could be reused ({status}); previous session {} was cleared",
                            session.id
                        )));
                    }
                    Err(error) => {
                        *state = None;
                        return Err(MediaError::Unavailable(format!(
                            "could not inspect ffmpeg stream process: {error}"
                        )));
                    }
                }
            } else {
                None
            }
        };

        if let Some(session) = existing_session {
            return Ok(MediaTransition {
                changed: false,
                session: Some(session),
                status: self.status_from_state().await,
            });
        }

        let ffmpeg_path = self.inner.ffmpeg_path.clone().ok_or_else(|| {
            MediaError::Unavailable(
                "ffmpeg was not found; install ffmpeg or set PATH before starting a real stream"
                    .to_string(),
            )
        })?;
        let publish_url = build_rtmp_publish_url(&request)?;

        let mut child = spawn_ffmpeg_stream(&ffmpeg_path, &publish_url)?;
        std::thread::sleep(Duration::from_millis(500));
        if let Some(status) = child
            .try_wait()
            .map_err(|error| MediaError::Unavailable(format!("ffmpeg startup failed: {error}")))?
        {
            return Err(MediaError::Unavailable(format!(
                "ffmpeg exited during stream startup ({status})"
            )));
        }

        let session = StreamSession {
            id: new_id("stream"),
            destination: request.destination,
            started_at: Utc::now(),
        };

        {
            let mut state = self
                .inner
                .stream
                .lock()
                .expect("ffmpeg stream state mutex poisoned");
            *state = Some(FfmpegActiveStream {
                session: session.clone(),
                child,
            });
        }
        let status = self.status_from_state().await;

        self.emit(StudioEvent::new(
            StudioEventKind::StreamStarted,
            json!({
                "session_id": session.id,
                "destination_id": session.destination.id,
                "destination_name": session.destination.name,
                "platform": session.destination.platform,
            }),
        ));

        Ok(MediaTransition {
            changed: true,
            session: Some(session),
            status,
        })
    }

    async fn stop_stream(&self) -> Result<MediaTransition<StreamSession>, MediaError> {
        let stopped = {
            let mut state = self
                .inner
                .stream
                .lock()
                .expect("ffmpeg stream state mutex poisoned");
            state.take().map(|mut active| {
                let _ = active.child.kill();
                let _ = active.child.wait();
                active.session
            })
        };
        let status = self.status_from_state().await;

        if let Some(session) = stopped.clone() {
            self.emit(StudioEvent::new(
                StudioEventKind::StreamStopped,
                json!({
                    "session_id": session.id,
                    "destination_id": session.destination.id,
                    "destination_name": session.destination.name,
                    "platform": session.destination.platform,
                }),
            ));
        }

        Ok(MediaTransition {
            changed: stopped.is_some(),
            session: stopped,
            status,
        })
    }

    async fn status(&self) -> EngineStatus {
        self.status_from_state().await
    }
}

impl Drop for FfmpegRtmpEngineInner {
    fn drop(&mut self) {
        let Ok(mut state) = self.stream.lock() else {
            return;
        };
        if let Some(mut active) = state.take() {
            let _ = active.child.kill();
            let _ = active.child.wait();
        }
    }
}

pub fn find_ffmpeg_binary() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            candidates.push(directory.join("ffmpeg"));
        }
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/ffmpeg"),
        PathBuf::from("/usr/local/bin/ffmpeg"),
        PathBuf::from("/usr/bin/ffmpeg"),
    ]);

    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn build_rtmp_publish_url(request: &StreamLaunchRequest) -> Result<String, MediaError> {
    let base = request.destination.ingest_url.trim();
    if base.is_empty() {
        return Err(MediaError::InvalidCommand(
            "stream destination requires an ingest URL".to_string(),
        ));
    }

    let key = request.stream_key.as_deref().unwrap_or("").trim();
    let mut url = if base.contains("{stream_key}") {
        if key.is_empty() {
            return Err(MediaError::InvalidCommand(
                "stream destination requires a stored stream key".to_string(),
            ));
        }
        base.replace("{stream_key}", key)
    } else if !key.is_empty() {
        format!("{}/{}", base.trim_end_matches('/'), key)
    } else if matches!(
        request.destination.platform,
        PlatformKind::Twitch | PlatformKind::YouTube | PlatformKind::Kick
    ) {
        return Err(MediaError::InvalidCommand(
            "stream destination requires a stored stream key".to_string(),
        ));
    } else {
        base.to_string()
    };

    if request.bandwidth_test && matches!(request.destination.platform, PlatformKind::Twitch) {
        url.push_str(if url.contains('?') {
            "&bandwidthtest=true"
        } else {
            "?bandwidthtest=true"
        });
    }

    Ok(url)
}

fn spawn_ffmpeg_stream(ffmpeg_path: &PathBuf, publish_url: &str) -> Result<Child, MediaError> {
    Command::new(ffmpeg_path)
        .args([
            "-hide_banner",
            "-loglevel",
            "warning",
            "-re",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1280x720:rate=30",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=channel_layout=stereo:sample_rate=44100",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-pix_fmt",
            "yuv420p",
            "-b:v",
            "2500k",
            "-maxrate",
            "2500k",
            "-bufsize",
            "5000k",
            "-g",
            "60",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-ar",
            "44100",
            "-f",
            "flv",
            publish_url,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| MediaError::Unavailable(format!("could not start ffmpeg: {error}")))
}

#[cfg(feature = "gstreamer")]
pub struct GStreamerMediaEngine;

#[cfg(feature = "gstreamer")]
impl GStreamerMediaEngine {
    pub fn new_placeholder() -> Self {
        Self
    }
}

#[cfg(feature = "gstreamer")]
#[async_trait]
impl MediaEngine for GStreamerMediaEngine {
    async fn start_recording(
        &self,
        _profile: MediaProfile,
    ) -> Result<MediaTransition<RecordingSession>, MediaError> {
        Err(MediaError::Unavailable(
            "GStreamerMediaEngine is a feature-gated placeholder; install and wire GStreamer in a future media engine milestone".to_string(),
        ))
    }

    async fn stop_recording(&self) -> Result<MediaTransition<RecordingSession>, MediaError> {
        Err(MediaError::Unavailable(
            "GStreamerMediaEngine is not implemented yet".to_string(),
        ))
    }

    async fn start_stream(
        &self,
        _request: StreamLaunchRequest,
    ) -> Result<MediaTransition<StreamSession>, MediaError> {
        Err(MediaError::Unavailable(
            "GStreamerMediaEngine is not implemented yet".to_string(),
        ))
    }

    async fn stop_stream(&self) -> Result<MediaTransition<StreamSession>, MediaError> {
        Err(MediaError::Unavailable(
            "GStreamerMediaEngine is not implemented yet".to_string(),
        ))
    }

    async fn status(&self) -> EngineStatus {
        EngineStatus::idle("GStreamerMediaEngine", EngineMode::GStreamer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vaexcore_core::{
        default_capture_sources, PipelineIntent, PlatformKind, StreamDestinationInput,
    };

    #[tokio::test]
    async fn recording_lifecycle_is_idempotent() {
        let engine = DryRunMediaEngine::new(None);
        let profile = MediaProfile::default_local();

        let first = engine.start_recording(profile.clone()).await.unwrap();
        assert!(first.changed);
        assert!(first.status.recording_active);

        let second = engine.start_recording(profile).await.unwrap();
        assert!(!second.changed);
        assert_eq!(first.session.unwrap().id, second.session.unwrap().id);

        let stopped = engine.stop_recording().await.unwrap();
        assert!(stopped.changed);
        assert!(!stopped.status.recording_active);

        let stopped_again = engine.stop_recording().await.unwrap();
        assert!(!stopped_again.changed);
        assert!(stopped_again.session.is_none());
    }

    #[tokio::test]
    async fn stream_lifecycle_is_idempotent() {
        let engine = DryRunMediaEngine::new(None);
        let destination = StreamDestination::from_input(
            StreamDestinationInput {
                name: "Dry Run".to_string(),
                platform: PlatformKind::CustomRtmp,
                ingest_url: Some("rtmp://localhost/live".to_string()),
                stream_key: None,
                enabled: Some(true),
            },
            None,
        );

        let first = engine
            .start_stream(StreamLaunchRequest::new(destination.clone()))
            .await
            .unwrap();
        assert!(first.changed);
        assert!(first.status.stream_active);

        let second = engine
            .start_stream(StreamLaunchRequest::new(destination))
            .await
            .unwrap();
        assert!(!second.changed);
        assert_eq!(first.session.unwrap().id, second.session.unwrap().id);

        let stopped = engine.stop_stream().await.unwrap();
        assert!(stopped.changed);
        assert!(!stopped.status.stream_active);

        let stopped_again = engine.stop_stream().await.unwrap();
        assert!(!stopped_again.changed);
        assert!(stopped_again.session.is_none());
    }

    #[test]
    fn dry_run_pipeline_plan_reports_missing_stream_key_as_warning() {
        let destination = StreamDestination::from_input(
            StreamDestinationInput {
                name: "Dry Run".to_string(),
                platform: PlatformKind::CustomRtmp,
                ingest_url: Some("rtmp://localhost/live".to_string()),
                stream_key: None,
                enabled: Some(true),
            },
            None,
        );

        let plan = build_dry_run_pipeline_plan(MediaPipelinePlanRequest {
            dry_run: true,
            intent: PipelineIntent::RecordingAndStream,
            capture_sources: default_capture_sources(),
            recording_profile: Some(MediaProfile::default_local()),
            stream_destinations: vec![destination],
        });

        assert!(plan.ready);
        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.contains("stored stream keys")));
    }

    #[test]
    fn dry_run_pipeline_plan_blocks_without_capture_source() {
        let plan = build_dry_run_pipeline_plan(MediaPipelinePlanRequest {
            dry_run: true,
            intent: PipelineIntent::Recording,
            capture_sources: vec![],
            recording_profile: Some(MediaProfile::default_local()),
            stream_destinations: vec![],
        });

        assert!(!plan.ready);
        assert!(plan
            .errors
            .iter()
            .any(|error| error.contains("capture source")));
    }
}
