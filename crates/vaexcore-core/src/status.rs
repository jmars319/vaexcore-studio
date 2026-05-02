use crate::{MediaProfile, StreamDestination};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EngineMode {
    DryRun,
    GStreamer,
    ExternalSidecar,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RecordingSession {
    pub id: String,
    pub profile: MediaProfile,
    pub output_path: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StreamSession {
    pub id: String,
    pub destination: StreamDestination,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EngineStatus {
    pub engine: String,
    pub mode: EngineMode,
    pub recording: Option<RecordingSession>,
    pub stream: Option<StreamSession>,
    pub recording_active: bool,
    pub stream_active: bool,
    pub recording_path: Option<String>,
    pub active_destination: Option<StreamDestination>,
    pub updated_at: DateTime<Utc>,
}

impl EngineStatus {
    pub fn idle(engine: impl Into<String>, mode: EngineMode) -> Self {
        Self {
            engine: engine.into(),
            mode,
            recording: None,
            stream: None,
            recording_active: false,
            stream_active: false,
            recording_path: None,
            active_destination: None,
            updated_at: Utc::now(),
        }
    }
}
