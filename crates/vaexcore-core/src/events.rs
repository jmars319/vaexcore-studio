use crate::{new_id, now_utc};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct StudioEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: StudioEventKind,
    pub timestamp: DateTime<Utc>,
    pub payload: Value,
}

impl StudioEvent {
    pub fn new(kind: StudioEventKind, payload: Value) -> Self {
        Self {
            id: new_id("evt"),
            kind,
            timestamp: now_utc(),
            payload,
        }
    }

    pub fn simple(kind: StudioEventKind) -> Self {
        Self::new(kind, json!({}))
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(StudioEventKind::Error, json!({ "message": message.into() }))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum StudioEventKind {
    #[serde(rename = "app.ready")]
    AppReady,
    #[serde(rename = "media.engine.ready")]
    MediaEngineReady,
    #[serde(rename = "recording.started")]
    RecordingStarted,
    #[serde(rename = "recording.stopped")]
    RecordingStopped,
    #[serde(rename = "stream.started")]
    StreamStarted,
    #[serde(rename = "stream.stopped")]
    StreamStopped,
    #[serde(rename = "marker.created")]
    MarkerCreated,
    #[serde(rename = "error")]
    Error,
}
