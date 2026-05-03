use crate::{
    EngineStatus, MediaProfile, MediaProfileInput, PlatformKind, StreamDestination, StudioEvent,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<ApiErrorBody>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ApiErrorBody {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct HealthResponse {
    pub service: String,
    pub version: String,
    pub ok: bool,
    pub auth_required: bool,
    pub dev_auth_bypass: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProfilesSnapshot {
    pub recording_profiles: Vec<MediaProfile>,
    pub stream_destinations: Vec<StreamDestination>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CommandStatus {
    pub changed: bool,
    pub message: String,
    pub status: EngineStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Marker {
    pub id: String,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StudioStatus {
    pub status: EngineStatus,
    pub recent_events: Vec<StudioEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ConnectedClient {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub user_agent: Option<String>,
    pub last_request_id: Option<String>,
    pub last_path: Option<String>,
    pub request_count: u64,
    pub connected_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ConnectedClientsSnapshot {
    pub clients: Vec<ConnectedClient>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AuditLogEntry {
    pub id: String,
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub action: String,
    pub status_code: u16,
    pub ok: bool,
    pub client_id: Option<String>,
    pub client_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AuditLogSnapshot {
    pub entries: Vec<AuditLogEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StreamDestinationBundleItem {
    pub name: String,
    pub platform: PlatformKind,
    pub ingest_url: String,
    pub enabled: bool,
    pub has_stream_key: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProfileBundle {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub recording_profiles: Vec<MediaProfileInput>,
    pub stream_destinations: Vec<StreamDestinationBundleItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProfileBundleImportResult {
    pub recording_profiles: usize,
    pub stream_destinations: usize,
}
