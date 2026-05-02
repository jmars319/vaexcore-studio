use crate::{new_id, now_utc, SecretRef, SensitiveString};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformKind {
    Twitch,
    YouTube,
    Kick,
    CustomRtmp,
}

impl PlatformKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Twitch => "twitch",
            Self::YouTube => "youtube",
            Self::Kick => "kick",
            Self::CustomRtmp => "custom_rtmp",
        }
    }
}

impl TryFrom<&str> for PlatformKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "twitch" => Ok(Self::Twitch),
            "youtube" => Ok(Self::YouTube),
            "kick" => Ok(Self::Kick),
            "custom_rtmp" => Ok(Self::CustomRtmp),
            other => Err(format!("unknown platform kind '{other}'")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecordingContainer {
    Mkv,
    Mp4,
}

impl RecordingContainer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mkv => "mkv",
            Self::Mp4 => "mp4",
        }
    }
}

impl TryFrom<&str> for RecordingContainer {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "mkv" => Ok(Self::Mkv),
            "mp4" => Ok(Self::Mp4),
            other => Err(format!("unknown recording container '{other}'")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EncoderPreference {
    Auto,
    Hardware,
    Software,
    Named(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Default for Resolution {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MediaProfile {
    pub id: String,
    pub name: String,
    pub output_folder: String,
    pub filename_pattern: String,
    pub container: RecordingContainer,
    pub resolution: Resolution,
    pub framerate: u32,
    pub bitrate_kbps: u32,
    pub encoder_preference: EncoderPreference,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MediaProfile {
    pub fn from_input(input: MediaProfileInput) -> Self {
        let now = now_utc();
        Self {
            id: new_id("rec_profile"),
            name: input.name,
            output_folder: input.output_folder,
            filename_pattern: input.filename_pattern,
            container: input.container,
            resolution: input.resolution,
            framerate: input.framerate,
            bitrate_kbps: input.bitrate_kbps,
            encoder_preference: input.encoder_preference,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn default_local() -> Self {
        Self::from_input(MediaProfileInput::default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MediaProfileInput {
    pub name: String,
    pub output_folder: String,
    pub filename_pattern: String,
    pub container: RecordingContainer,
    pub resolution: Resolution,
    pub framerate: u32,
    pub bitrate_kbps: u32,
    pub encoder_preference: EncoderPreference,
}

impl Default for MediaProfileInput {
    fn default() -> Self {
        Self {
            name: "Default Local Recording".to_string(),
            output_folder: "~/Movies/vaexcore studio".to_string(),
            filename_pattern: "{date}-{time}-{profile}".to_string(),
            container: RecordingContainer::Mkv,
            resolution: Resolution::default(),
            framerate: 60,
            bitrate_kbps: 12_000,
            encoder_preference: EncoderPreference::Auto,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StreamDestination {
    pub id: String,
    pub name: String,
    pub platform: PlatformKind,
    pub ingest_url: String,
    pub stream_key_ref: Option<SecretRef>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl StreamDestination {
    pub fn from_input(input: StreamDestinationInput, stream_key_ref: Option<SecretRef>) -> Self {
        let now = now_utc();
        Self {
            id: new_id("stream_dest"),
            name: input.name,
            platform: input.platform,
            ingest_url: input.ingest_url.unwrap_or_default(),
            stream_key_ref,
            enabled: input.enabled.unwrap_or(true),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn has_stream_key(&self) -> bool {
        self.stream_key_ref.is_some()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StreamDestinationInput {
    pub name: String,
    pub platform: PlatformKind,
    pub ingest_url: Option<String>,
    pub stream_key: Option<SensitiveString>,
    pub enabled: Option<bool>,
}
