use serde::{Deserialize, Serialize};
use vaexcore_core::{PlatformKind, StreamDestinationInput};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PlatformProfile {
    pub kind: PlatformKind,
    pub display_name: String,
    pub default_ingest_url: Option<String>,
    pub protocol: String,
    pub requires_stream_key: bool,
    pub notes: String,
}

pub fn platform_profiles() -> Vec<PlatformProfile> {
    vec![
        PlatformProfile {
            kind: PlatformKind::Twitch,
            display_name: "Twitch".to_string(),
            default_ingest_url: Some("rtmp://live.twitch.tv/app".to_string()),
            protocol: "RTMP".to_string(),
            requires_stream_key: true,
            notes: "Uses the stream key from Twitch Creator Dashboard. OAuth is intentionally out of scope for MVP.".to_string(),
        },
        PlatformProfile {
            kind: PlatformKind::YouTube,
            display_name: "YouTube".to_string(),
            default_ingest_url: Some("rtmps://a.rtmps.youtube.com/live2".to_string()),
            protocol: "RTMPS".to_string(),
            requires_stream_key: true,
            notes: "Uses YouTube's manual RTMPS ingest and stream key. OAuth is intentionally out of scope for MVP.".to_string(),
        },
        PlatformProfile {
            kind: PlatformKind::Kick,
            display_name: "Kick".to_string(),
            default_ingest_url: None,
            protocol: "RTMP/RTMPS".to_string(),
            requires_stream_key: true,
            notes: "Kick is represented as a named custom RTMP/RTMPS profile for MVP; paste the current ingest URL from Kick.".to_string(),
        },
        PlatformProfile {
            kind: PlatformKind::CustomRtmp,
            display_name: "Custom RTMP".to_string(),
            default_ingest_url: Some("rtmp://localhost/live".to_string()),
            protocol: "RTMP/RTMPS".to_string(),
            requires_stream_key: false,
            notes: "Full manual control for local or third-party RTMP/RTMPS endpoints.".to_string(),
        },
    ]
}

pub fn apply_platform_defaults(mut input: StreamDestinationInput) -> StreamDestinationInput {
    if input
        .ingest_url
        .as_ref()
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        input.ingest_url = platform_profiles()
            .into_iter()
            .find(|profile| profile.kind == input.platform)
            .and_then(|profile| profile.default_ingest_url);
    }

    input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_rtmp_has_a_local_dry_run_default() {
        let profile = platform_profiles()
            .into_iter()
            .find(|profile| profile.kind == PlatformKind::CustomRtmp)
            .unwrap();

        assert_eq!(
            profile.default_ingest_url.as_deref(),
            Some("rtmp://localhost/live")
        );
    }
}
