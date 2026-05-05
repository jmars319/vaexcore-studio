use crate::StreamLaunchRequest;
use crate::{MediaEngine, MediaError, MediaEventSink, MediaTransition};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vaexcore_core::{
    ApiResponse, EngineMode, EngineStatus, MediaPipelinePlan, MediaPipelinePlanRequest,
    MediaPipelineValidation, MediaProfile, RecordingSession, StreamSession, StudioEvent,
    StudioEventKind,
};

#[derive(Clone, Debug)]
pub struct MediaRunnerConfig {
    pub executable_path: PathBuf,
    pub status_addr: SocketAddr,
    pub dry_run: bool,
    pub config_path: Option<PathBuf>,
    pub startup_timeout: Duration,
}

impl MediaRunnerConfig {
    pub fn dry_run(executable_path: PathBuf, status_addr: SocketAddr) -> Self {
        Self {
            executable_path,
            status_addr,
            dry_run: true,
            config_path: None,
            startup_timeout: Duration::from_secs(2),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MediaRunnerStatus {
    pub service: String,
    pub ready: bool,
    pub dry_run: bool,
    pub pipeline_name: Option<String>,
    pub engine_status: EngineStatus,
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("media runner executable not found: {0}")]
    MissingExecutable(PathBuf),
    #[error("failed to start media runner '{}': {source}", path.display())]
    Spawn {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("media runner exited during startup: {0}")]
    Exited(ExitStatus),
    #[error("media runner did not become ready at {addr} within {timeout:?}")]
    StartupTimeout { addr: SocketAddr, timeout: Duration },
    #[error("media runner HTTP request failed: {0}")]
    Http(String),
    #[error("media runner I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("media runner JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("media runner task join error: {0}")]
    Join(String),
}

#[derive(Clone, Debug)]
pub struct MediaRunnerSupervisor {
    inner: Arc<MediaRunnerSupervisorInner>,
}

#[derive(Debug)]
struct MediaRunnerSupervisorInner {
    child: Mutex<Option<Child>>,
    executable_path: PathBuf,
    status_addr: SocketAddr,
    dry_run: bool,
    config_path: Option<PathBuf>,
    startup_timeout: Duration,
    require_child: bool,
}

impl MediaRunnerSupervisor {
    pub fn start(config: MediaRunnerConfig) -> Result<Self, SidecarError> {
        if !config.executable_path.is_file() {
            return Err(SidecarError::MissingExecutable(config.executable_path));
        }

        let child = spawn_media_runner_child(
            &config.executable_path,
            config.status_addr,
            config.dry_run,
            config.config_path.as_ref(),
        )?;

        let supervisor = Self {
            inner: Arc::new(MediaRunnerSupervisorInner {
                child: Mutex::new(Some(child)),
                executable_path: config.executable_path,
                status_addr: config.status_addr,
                dry_run: config.dry_run,
                config_path: config.config_path,
                startup_timeout: config.startup_timeout,
                require_child: true,
            }),
        };

        supervisor.wait_until_ready(config.startup_timeout)?;
        Ok(supervisor)
    }

    pub fn executable_path(&self) -> PathBuf {
        self.inner.executable_path.clone()
    }

    pub fn status_addr(&self) -> SocketAddr {
        self.inner.status_addr
    }

    pub async fn health(&self) -> Result<(), SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.health_blocking())
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub async fn status(&self) -> Result<MediaRunnerStatus, SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.status_blocking())
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub async fn start_recording(
        &self,
        profile: MediaProfile,
    ) -> Result<MediaTransition<RecordingSession>, SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.post_blocking("/recording/start", &profile))
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub async fn stop_recording(&self) -> Result<MediaTransition<RecordingSession>, SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.post_empty_blocking("/recording/stop"))
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub async fn start_stream(
        &self,
        request: StreamLaunchRequest,
    ) -> Result<MediaTransition<StreamSession>, SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.post_blocking("/stream/start", &request))
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub async fn stop_stream(&self) -> Result<MediaTransition<StreamSession>, SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.post_empty_blocking("/stream/stop"))
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub async fn plan_pipeline(
        &self,
        request: MediaPipelinePlanRequest,
    ) -> Result<MediaPipelinePlan, SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.post_blocking("/plan", &request))
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub async fn validate_pipeline(
        &self,
        request: MediaPipelinePlanRequest,
    ) -> Result<MediaPipelineValidation, SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.post_blocking("/validate", &request))
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub async fn restart(&self) -> Result<(), SidecarError> {
        let supervisor = self.clone();
        tokio::task::spawn_blocking(move || supervisor.restart_blocking())
            .await
            .map_err(|error| SidecarError::Join(error.to_string()))?
    }

    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    fn restart_blocking(&self) -> Result<(), SidecarError> {
        self.inner.shutdown();
        let child = spawn_media_runner_child(
            &self.inner.executable_path,
            self.inner.status_addr,
            self.inner.dry_run,
            self.inner.config_path.as_ref(),
        )?;
        *self
            .inner
            .child
            .lock()
            .expect("media runner child mutex poisoned") = Some(child);
        self.wait_until_ready(self.inner.startup_timeout)
    }

    fn wait_until_ready(&self, timeout: Duration) -> Result<(), SidecarError> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            self.ensure_running()?;
            if self.health_blocking().is_ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        Err(SidecarError::StartupTimeout {
            addr: self.status_addr(),
            timeout,
        })
    }

    fn health_blocking(&self) -> Result<(), SidecarError> {
        self.ensure_running()?;
        let body: serde_json::Value = request_api_response(
            self.status_addr(),
            "GET",
            "/health",
            None,
            Duration::from_millis(500),
        )?;
        let ok = body
            .get("ok")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        ok.then_some(()).ok_or_else(|| {
            SidecarError::Http("media runner health endpoint returned ok=false".to_string())
        })
    }

    fn status_blocking(&self) -> Result<MediaRunnerStatus, SidecarError> {
        self.ensure_running()?;
        request_api_response(
            self.status_addr(),
            "GET",
            "/status",
            None,
            Duration::from_millis(500),
        )
    }

    fn post_blocking<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, SidecarError> {
        self.ensure_running()?;
        request_api_response(
            self.status_addr(),
            "POST",
            path,
            Some(serde_json::to_string(body)?),
            Duration::from_secs(2),
        )
    }

    fn post_empty_blocking<T: DeserializeOwned>(&self, path: &str) -> Result<T, SidecarError> {
        self.ensure_running()?;
        request_api_response(
            self.status_addr(),
            "POST",
            path,
            Some("{}".to_string()),
            Duration::from_secs(2),
        )
    }

    fn ensure_running(&self) -> Result<(), SidecarError> {
        let mut guard = self
            .inner
            .child
            .lock()
            .expect("media runner child mutex poisoned");
        let Some(child) = guard.as_mut() else {
            if !self.inner.require_child {
                return Ok(());
            }
            return Err(SidecarError::Http(
                "media runner is not running".to_string(),
            ));
        };

        if let Some(status) = child.try_wait()? {
            *guard = None;
            return Err(SidecarError::Exited(status));
        }

        Ok(())
    }
}

fn spawn_media_runner_child(
    executable_path: &PathBuf,
    status_addr: SocketAddr,
    dry_run: bool,
    config_path: Option<&PathBuf>,
) -> Result<Child, SidecarError> {
    let mut command = Command::new(executable_path);
    command
        .arg("--status-addr")
        .arg(status_addr.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(config_path) = config_path {
        command.arg("--config").arg(config_path);
    }

    if dry_run {
        command.arg("--dry-run");
    }

    command.spawn().map_err(|source| SidecarError::Spawn {
        path: executable_path.clone(),
        source,
    })
}

impl MediaRunnerSupervisorInner {
    fn shutdown(&self) {
        let Some(mut child) = self
            .child
            .lock()
            .expect("media runner child mutex poisoned")
            .take()
        else {
            return;
        };

        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for MediaRunnerSupervisorInner {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Clone)]
pub struct SidecarMediaEngine {
    runner: MediaRunnerSupervisor,
    event_sink: Option<MediaEventSink>,
}

impl SidecarMediaEngine {
    pub fn new(runner: MediaRunnerSupervisor, event_sink: Option<MediaEventSink>) -> Self {
        Self { runner, event_sink }
    }

    pub fn runner(&self) -> MediaRunnerSupervisor {
        self.runner.clone()
    }

    fn emit(&self, event: StudioEvent) {
        if let Some(sink) = &self.event_sink {
            sink(event);
        }
    }

    fn emit_recording_started(&self, transition: &MediaTransition<RecordingSession>) {
        if !transition.changed {
            return;
        }
        if let Some(session) = &transition.session {
            self.emit(StudioEvent::new(
                StudioEventKind::RecordingStarted,
                json!({
                    "session_id": session.id,
                    "output_path": session.output_path,
                    "profile_id": session.profile.id,
                }),
            ));
        }
    }

    fn emit_recording_stopped(&self, transition: &MediaTransition<RecordingSession>) {
        if !transition.changed {
            return;
        }
        if let Some(session) = &transition.session {
            self.emit(StudioEvent::new(
                StudioEventKind::RecordingStopped,
                json!({
                    "session_id": session.id,
                    "output_path": session.output_path,
                    "profile_id": session.profile.id,
                }),
            ));
        }
    }

    fn emit_stream_started(&self, transition: &MediaTransition<StreamSession>) {
        if !transition.changed {
            return;
        }
        if let Some(session) = &transition.session {
            self.emit(StudioEvent::new(
                StudioEventKind::StreamStarted,
                json!({
                    "session_id": session.id,
                    "destination_id": session.destination.id,
                    "destination_name": session.destination.name,
                    "platform": session.destination.platform,
                }),
            ));
        }
    }

    fn emit_stream_stopped(&self, transition: &MediaTransition<StreamSession>) {
        if !transition.changed {
            return;
        }
        if let Some(session) = &transition.session {
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
    }

    fn sidecar_status_from_runner_status(&self, runner_status: MediaRunnerStatus) -> EngineStatus {
        let mut status = runner_status.engine_status;
        status.engine = format!(
            "SidecarMediaEngine ({})",
            runner_status
                .pipeline_name
                .unwrap_or_else(|| runner_status.service.clone())
        );
        status.mode = if runner_status.dry_run {
            EngineMode::DryRun
        } else {
            status.mode
        };
        status
    }

    async fn sidecar_status(&self) -> EngineStatus {
        match self.runner.status().await {
            Ok(runner_status) if runner_status.ready => {
                self.sidecar_status_from_runner_status(runner_status)
            }
            _ => EngineStatus::idle("SidecarMediaEngine unavailable", EngineMode::DryRun),
        }
    }
}

impl From<SidecarError> for MediaError {
    fn from(error: SidecarError) -> Self {
        Self::Unavailable(error.to_string())
    }
}

#[async_trait]
impl MediaEngine for SidecarMediaEngine {
    async fn start_recording(
        &self,
        profile: MediaProfile,
    ) -> Result<MediaTransition<RecordingSession>, MediaError> {
        let mut transition = self.runner.start_recording(profile).await?;
        transition.status = self
            .runner
            .status()
            .await
            .map(|status| self.sidecar_status_from_runner_status(status))?;
        self.emit_recording_started(&transition);
        Ok(transition)
    }

    async fn stop_recording(&self) -> Result<MediaTransition<RecordingSession>, MediaError> {
        let mut transition = self.runner.stop_recording().await?;
        transition.status = self
            .runner
            .status()
            .await
            .map(|status| self.sidecar_status_from_runner_status(status))?;
        self.emit_recording_stopped(&transition);
        Ok(transition)
    }

    async fn start_stream(
        &self,
        request: StreamLaunchRequest,
    ) -> Result<MediaTransition<StreamSession>, MediaError> {
        let mut transition = self.runner.start_stream(request).await?;
        transition.status = self
            .runner
            .status()
            .await
            .map(|status| self.sidecar_status_from_runner_status(status))?;
        self.emit_stream_started(&transition);
        Ok(transition)
    }

    async fn stop_stream(&self) -> Result<MediaTransition<StreamSession>, MediaError> {
        let mut transition = self.runner.stop_stream().await?;
        transition.status = self
            .runner
            .status()
            .await
            .map(|status| self.sidecar_status_from_runner_status(status))?;
        self.emit_stream_stopped(&transition);
        Ok(transition)
    }

    async fn status(&self) -> EngineStatus {
        self.sidecar_status().await
    }
}

fn request_api_response<T: DeserializeOwned>(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<String>,
    timeout: Duration,
) -> Result<T, SidecarError> {
    let mut stream = TcpStream::connect_timeout(&addr, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let body = body.unwrap_or_default();
    let content_headers = if body.is_empty() {
        String::new()
    } else {
        format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        )
    };
    stream.write_all(
        format!(
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n{content_headers}\r\n{body}"
        )
        .as_bytes(),
    )?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| SidecarError::Http("malformed HTTP response".to_string()))?;
    let status_line = head.lines().next().unwrap_or_default();
    if !status_line.contains(" 200 ") {
        let message = serde_json::from_str::<ApiResponse<serde_json::Value>>(body.trim())
            .ok()
            .and_then(|response| response.error.map(|error| error.message))
            .unwrap_or_else(|| status_line.to_string());
        return Err(SidecarError::Http(message));
    }

    let response: ApiResponse<T> = serde_json::from_str(body.trim())?;
    if !response.ok {
        let message = response
            .error
            .map(|error| error.message)
            .unwrap_or_else(|| "unknown sidecar API error".to_string());
        return Err(SidecarError::Http(message));
    }

    response
        .data
        .ok_or_else(|| SidecarError::Http("missing sidecar response data".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::{
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread::{self, JoinHandle},
    };
    use vaexcore_core::{new_id, PlatformKind, StreamDestination, StreamDestinationInput};

    #[test]
    fn missing_runner_executable_is_reported() {
        let config = MediaRunnerConfig::dry_run(
            PathBuf::from("/definitely/missing/media-runner"),
            "127.0.0.1:1".parse().unwrap(),
        );

        let error = MediaRunnerSupervisor::start(config).unwrap_err();
        assert!(matches!(error, SidecarError::MissingExecutable(_)));
    }

    #[tokio::test]
    async fn sidecar_engine_preserves_dry_run_lifecycle() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let runner = FakeRunner::start(listener);
        let engine = SidecarMediaEngine::new(runner.supervisor(), None);
        let profile = MediaProfile::default_local();

        let first = engine.start_recording(profile.clone()).await.unwrap();
        let second = engine.start_recording(profile).await.unwrap();
        assert!(first.changed);
        assert!(!second.changed);
        assert!(second.status.recording_active);
        assert!(second.status.engine.starts_with("SidecarMediaEngine"));

        let stopped = engine.stop_recording().await.unwrap();
        assert!(stopped.changed);
        assert!(!stopped.status.recording_active);
    }

    #[tokio::test]
    async fn sidecar_engine_can_stream_in_dry_run_mode() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let runner = FakeRunner::start(listener);
        let engine = SidecarMediaEngine::new(runner.supervisor(), None);
        let destination = StreamDestination::from_input(
            StreamDestinationInput {
                name: "Sidecar Dry Run".to_string(),
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
        let second = engine
            .start_stream(StreamLaunchRequest::new(destination))
            .await
            .unwrap();
        assert!(first.changed);
        assert!(!second.changed);
        assert!(second.status.stream_active);

        let stopped = engine.stop_stream().await.unwrap();
        assert!(stopped.changed);
        assert!(!stopped.status.stream_active);
    }

    struct FakeRunner {
        supervisor: MediaRunnerSupervisor,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl FakeRunner {
        fn start(listener: TcpListener) -> Self {
            let addr = listener.local_addr().unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let thread_stop = stop.clone();
            let thread = thread::spawn(move || {
                let mut runner_state = TestRunnerState::default();
                listener.set_nonblocking(true).unwrap();
                while !thread_stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let mut request = [0_u8; 1024];
                            let bytes = stream.read(&mut request).unwrap_or(0);
                            let request = String::from_utf8_lossy(&request[..bytes]);
                            let request_body = request.split("\r\n\r\n").nth(1).unwrap_or("");
                            let response_body = if request.starts_with("GET /status ") {
                                serde_json::to_string(&ApiResponse::ok(MediaRunnerStatus {
                                    service: "fake-media-runner".to_string(),
                                    ready: true,
                                    dry_run: true,
                                    pipeline_name: Some("test".to_string()),
                                    engine_status: runner_state.status(),
                                }))
                                .unwrap()
                            } else if request.starts_with("POST /recording/start ") {
                                let profile: MediaProfile =
                                    serde_json::from_str(request_body).unwrap();
                                serde_json::to_string(&ApiResponse::ok(
                                    runner_state.start_recording(profile),
                                ))
                                .unwrap()
                            } else if request.starts_with("POST /recording/stop ") {
                                serde_json::to_string(&ApiResponse::ok(
                                    runner_state.stop_recording(),
                                ))
                                .unwrap()
                            } else if request.starts_with("POST /stream/start ") {
                                let request: StreamLaunchRequest =
                                    serde_json::from_str(request_body).unwrap();
                                serde_json::to_string(&ApiResponse::ok(
                                    runner_state.start_stream(request.destination),
                                ))
                                .unwrap()
                            } else if request.starts_with("POST /stream/stop ") {
                                serde_json::to_string(&ApiResponse::ok(runner_state.stop_stream()))
                                    .unwrap()
                            } else {
                                serde_json::to_string(&ApiResponse::ok(serde_json::json!({
                                    "service": "fake-media-runner",
                                    "ok": true
                                })))
                                .unwrap()
                            };
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                response_body.len(),
                                response_body
                            );
                            let _ = stream.write_all(response.as_bytes());
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            });
            let supervisor = MediaRunnerSupervisor {
                inner: Arc::new(MediaRunnerSupervisorInner {
                    child: Mutex::new(None),
                    executable_path: PathBuf::from("fake-media-runner"),
                    status_addr: addr,
                    dry_run: true,
                    config_path: None,
                    startup_timeout: Duration::from_secs(2),
                    require_child: false,
                }),
            };
            supervisor
                .wait_until_ready(Duration::from_secs(2))
                .expect("fake runner should become ready");

            Self {
                supervisor,
                stop,
                thread: Some(thread),
            }
        }

        fn supervisor(&self) -> MediaRunnerSupervisor {
            self.supervisor.clone()
        }
    }

    impl Drop for FakeRunner {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect_timeout(
                &self.supervisor.status_addr(),
                Duration::from_millis(50),
            );
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    #[derive(Default)]
    struct TestRunnerState {
        recording: Option<RecordingSession>,
        stream: Option<StreamSession>,
    }

    impl TestRunnerState {
        fn status(&self) -> EngineStatus {
            EngineStatus {
                engine: "fake-media-runner".to_string(),
                mode: EngineMode::DryRun,
                recording: self.recording.clone(),
                stream: self.stream.clone(),
                recording_active: self.recording.is_some(),
                stream_active: self.stream.is_some(),
                recording_path: self
                    .recording
                    .as_ref()
                    .map(|session| session.output_path.clone()),
                active_destination: self
                    .stream
                    .as_ref()
                    .map(|session| session.destination.clone()),
                updated_at: Utc::now(),
            }
        }

        fn start_recording(&mut self, profile: MediaProfile) -> MediaTransition<RecordingSession> {
            if let Some(session) = self.recording.clone() {
                return MediaTransition {
                    changed: false,
                    session: Some(session),
                    status: self.status(),
                };
            }

            let session = RecordingSession {
                id: new_id("recording"),
                profile,
                output_path: "fake-output.mkv".to_string(),
                started_at: Utc::now(),
            };
            self.recording = Some(session.clone());
            MediaTransition {
                changed: true,
                session: Some(session),
                status: self.status(),
            }
        }

        fn stop_recording(&mut self) -> MediaTransition<RecordingSession> {
            let session = self.recording.take();
            MediaTransition {
                changed: session.is_some(),
                session,
                status: self.status(),
            }
        }

        fn start_stream(
            &mut self,
            destination: StreamDestination,
        ) -> MediaTransition<StreamSession> {
            if let Some(session) = self.stream.clone() {
                return MediaTransition {
                    changed: false,
                    session: Some(session),
                    status: self.status(),
                };
            }

            let session = StreamSession {
                id: new_id("stream"),
                destination,
                started_at: Utc::now(),
            };
            self.stream = Some(session.clone());
            MediaTransition {
                changed: true,
                session: Some(session),
                status: self.status(),
            }
        }

        fn stop_stream(&mut self) -> MediaTransition<StreamSession> {
            let session = self.stream.take();
            MediaTransition {
                changed: session.is_some(),
                session,
                status: self.status(),
            }
        }
    }
}
