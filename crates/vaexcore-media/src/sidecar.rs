use crate::{DryRunMediaEngine, MediaEngine, MediaError, MediaEventSink, MediaTransition};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vaexcore_core::{
    ApiResponse, EngineMode, EngineStatus, RecordingSession, StreamDestination, StreamSession,
};

#[derive(Clone, Debug)]
pub struct MediaRunnerConfig {
    pub executable_path: PathBuf,
    pub status_addr: SocketAddr,
    pub dry_run: bool,
    pub startup_timeout: Duration,
}

impl MediaRunnerConfig {
    pub fn dry_run(executable_path: PathBuf, status_addr: SocketAddr) -> Self {
        Self {
            executable_path,
            status_addr,
            dry_run: true,
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
    require_child: bool,
}

impl MediaRunnerSupervisor {
    pub fn start(config: MediaRunnerConfig) -> Result<Self, SidecarError> {
        if !config.executable_path.is_file() {
            return Err(SidecarError::MissingExecutable(config.executable_path));
        }

        let mut command = Command::new(&config.executable_path);
        command
            .arg("--status-addr")
            .arg(config.status_addr.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if config.dry_run {
            command.arg("--dry-run");
        }

        let child = command.spawn().map_err(|source| SidecarError::Spawn {
            path: config.executable_path.clone(),
            source,
        })?;

        let supervisor = Self {
            inner: Arc::new(MediaRunnerSupervisorInner {
                child: Mutex::new(Some(child)),
                executable_path: config.executable_path,
                status_addr: config.status_addr,
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

    pub fn shutdown(&self) {
        self.inner.shutdown();
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
        let body: serde_json::Value =
            get_api_response(self.status_addr(), "/health", Duration::from_millis(500))?;
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
        get_api_response(self.status_addr(), "/status", Duration::from_millis(500))
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
    dry_run: DryRunMediaEngine,
    runner: MediaRunnerSupervisor,
}

impl SidecarMediaEngine {
    pub fn new(runner: MediaRunnerSupervisor, event_sink: Option<MediaEventSink>) -> Self {
        Self {
            dry_run: DryRunMediaEngine::new(event_sink),
            runner,
        }
    }

    pub fn runner(&self) -> MediaRunnerSupervisor {
        self.runner.clone()
    }

    async fn sidecar_status(&self, mut status: EngineStatus) -> EngineStatus {
        match self.runner.status().await {
            Ok(runner_status) if runner_status.ready => {
                status.engine = format!(
                    "SidecarMediaEngine ({})",
                    runner_status
                        .pipeline_name
                        .unwrap_or_else(|| runner_status.service.clone())
                );
                status.mode = if runner_status.dry_run {
                    EngineMode::DryRun
                } else {
                    runner_status.engine_status.mode
                };
                status
            }
            _ => {
                status.engine = "DryRunMediaEngine".to_string();
                status.mode = EngineMode::DryRun;
                status
            }
        }
    }
}

#[async_trait]
impl MediaEngine for SidecarMediaEngine {
    async fn start_recording(
        &self,
        profile: vaexcore_core::MediaProfile,
    ) -> Result<MediaTransition<RecordingSession>, MediaError> {
        let mut transition = self.dry_run.start_recording(profile).await?;
        transition.status = self.sidecar_status(transition.status).await;
        Ok(transition)
    }

    async fn stop_recording(&self) -> Result<MediaTransition<RecordingSession>, MediaError> {
        let mut transition = self.dry_run.stop_recording().await?;
        transition.status = self.sidecar_status(transition.status).await;
        Ok(transition)
    }

    async fn start_stream(
        &self,
        destination: StreamDestination,
    ) -> Result<MediaTransition<StreamSession>, MediaError> {
        let mut transition = self.dry_run.start_stream(destination).await?;
        transition.status = self.sidecar_status(transition.status).await;
        Ok(transition)
    }

    async fn stop_stream(&self) -> Result<MediaTransition<StreamSession>, MediaError> {
        let mut transition = self.dry_run.stop_stream().await?;
        transition.status = self.sidecar_status(transition.status).await;
        Ok(transition)
    }

    async fn status(&self) -> EngineStatus {
        let status = self.dry_run.status().await;
        self.sidecar_status(status).await
    }
}

fn get_api_response<T: DeserializeOwned>(
    addr: SocketAddr,
    path: &str,
    timeout: Duration,
) -> Result<T, SidecarError> {
    let mut stream = TcpStream::connect_timeout(&addr, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").as_bytes(),
    )?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| SidecarError::Http("malformed HTTP response".to_string()))?;
    let status_line = head.lines().next().unwrap_or_default();
    if !status_line.contains(" 200 ") {
        return Err(SidecarError::Http(status_line.to_string()));
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
    use std::{
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread::{self, JoinHandle},
    };
    use vaexcore_core::{MediaProfile, PlatformKind, StreamDestination, StreamDestinationInput};

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

        let first = engine.start_stream(destination.clone()).await.unwrap();
        let second = engine.start_stream(destination).await.unwrap();
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
                listener.set_nonblocking(true).unwrap();
                while !thread_stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let mut request = [0_u8; 1024];
                            let bytes = stream.read(&mut request).unwrap_or(0);
                            let request = String::from_utf8_lossy(&request[..bytes]);
                            let body = if request.starts_with("GET /status ") {
                                serde_json::to_string(&ApiResponse::ok(MediaRunnerStatus {
                                    service: "fake-media-runner".to_string(),
                                    ready: true,
                                    dry_run: true,
                                    pipeline_name: Some("test".to_string()),
                                    engine_status: EngineStatus::idle(
                                        "fake-media-runner",
                                        EngineMode::DryRun,
                                    ),
                                }))
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
                                body.len(),
                                body
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
}
