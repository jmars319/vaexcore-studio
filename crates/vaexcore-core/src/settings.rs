use serde::{Deserialize, Serialize};

use crate::{
    default_capture_sources, CaptureSourceSelection, MediaProfileInput, DEFAULT_API_HOST,
    DEFAULT_API_PORT,
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AppSettings {
    pub api_host: String,
    pub api_port: u16,
    pub api_token: Option<String>,
    pub dev_auth_bypass: bool,
    pub log_level: String,
    pub default_recording_profile: MediaProfileInput,
    #[serde(default = "default_capture_sources")]
    pub capture_sources: Vec<CaptureSourceSelection>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_host: DEFAULT_API_HOST.to_string(),
            api_port: DEFAULT_API_PORT,
            api_token: None,
            dev_auth_bypass: cfg!(debug_assertions),
            log_level: "info".to_string(),
            default_recording_profile: MediaProfileInput::default(),
            capture_sources: default_capture_sources(),
        }
    }
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.api_host.trim().is_empty() {
            return Err("API host is required".to_string());
        }

        if self.api_port == 0 {
            return Err("API port must be greater than zero".to_string());
        }

        format!("{}:{}", self.api_host, self.api_port)
            .parse::<std::net::SocketAddr>()
            .map_err(|_| "API host must be an IP address that can bind locally".to_string())?;

        if !matches!(
            self.log_level.as_str(),
            "trace" | "debug" | "info" | "warn" | "error"
        ) {
            return Err("log level must be trace, debug, info, warn, or error".to_string());
        }

        if self.default_recording_profile.name.trim().is_empty() {
            return Err("default recording profile name is required".to_string());
        }

        if self
            .default_recording_profile
            .output_folder
            .trim()
            .is_empty()
        {
            return Err("default recording output folder is required".to_string());
        }

        if self
            .default_recording_profile
            .filename_pattern
            .trim()
            .is_empty()
        {
            return Err("default recording filename pattern is required".to_string());
        }

        if self
            .capture_sources
            .iter()
            .any(|source| source.id.trim().is_empty())
        {
            return Err("capture source IDs cannot be empty".to_string());
        }

        Ok(())
    }
}
