pub mod events;
pub mod profiles;
pub mod responses;
pub mod security;
pub mod status;

pub use events::{StudioEvent, StudioEventKind};
pub use profiles::{
    EncoderPreference, MediaProfile, MediaProfileInput, PlatformKind, RecordingContainer,
    Resolution, StreamDestination, StreamDestinationInput,
};
pub use responses::{
    ApiErrorBody, ApiResponse, CommandStatus, HealthResponse, Marker, ProfilesSnapshot,
    StudioStatus,
};
pub use security::{SecretRef, SecretStore, SecretStoreError, SensitiveString};
pub use status::{EngineMode, EngineStatus, RecordingSession, StreamSession};

pub const APP_NAME: &str = "vaexcore-studio";
pub const DEFAULT_API_PORT: u16 = 51287;
pub const DEFAULT_API_HOST: &str = "127.0.0.1";

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
}

pub fn now_utc() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
