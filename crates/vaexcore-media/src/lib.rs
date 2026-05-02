use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use vaexcore_core::{
    new_id, EngineMode, EngineStatus, MediaProfile, RecordingContainer, RecordingSession,
    StreamDestination, StreamSession, StudioEvent, StudioEventKind,
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

#[async_trait]
pub trait MediaEngine: Send + Sync {
    async fn start_recording(
        &self,
        profile: MediaProfile,
    ) -> Result<MediaTransition<RecordingSession>, MediaError>;

    async fn stop_recording(&self) -> Result<MediaTransition<RecordingSession>, MediaError>;

    async fn start_stream(
        &self,
        destination: StreamDestination,
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
        destination: StreamDestination,
    ) -> Result<MediaTransition<StreamSession>, MediaError> {
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
        _destination: StreamDestination,
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
    use vaexcore_core::{PlatformKind, StreamDestinationInput};

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

        let first = engine.start_stream(destination.clone()).await.unwrap();
        assert!(first.changed);
        assert!(first.status.stream_active);

        let second = engine.start_stream(destination).await.unwrap();
        assert!(!second.changed);
        assert_eq!(first.session.unwrap().id, second.session.unwrap().id);

        let stopped = engine.stop_stream().await.unwrap();
        assert!(stopped.changed);
        assert!(!stopped.status.stream_active);

        let stopped_again = engine.stop_stream().await.unwrap();
        assert!(!stopped_again.changed);
        assert!(stopped_again.session.is_none());
    }
}
