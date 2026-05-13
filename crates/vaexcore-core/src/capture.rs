use serde::{Deserialize, Serialize};

use crate::{Scene, SceneSource, SceneSourceKind};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureSourceKind {
    Display,
    Window,
    Camera,
    Microphone,
    SystemAudio,
}

impl CaptureSourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::Window => "window",
            Self::Camera => "camera",
            Self::Microphone => "microphone",
            Self::SystemAudio => "system_audio",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CaptureSourceSelection {
    pub id: String,
    pub kind: CaptureSourceKind,
    pub name: String,
    pub enabled: bool,
}

impl CaptureSourceSelection {
    pub fn display_main() -> Self {
        Self {
            id: "display:main".to_string(),
            kind: CaptureSourceKind::Display,
            name: "Main Display".to_string(),
            enabled: true,
        }
    }

    pub fn microphone_default() -> Self {
        Self {
            id: "microphone:default".to_string(),
            kind: CaptureSourceKind::Microphone,
            name: "Default Microphone".to_string(),
            enabled: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CaptureSourceCandidate {
    pub id: String,
    pub kind: CaptureSourceKind,
    pub name: String,
    pub available: bool,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CaptureSourceInventory {
    pub candidates: Vec<CaptureSourceCandidate>,
    pub selected: Vec<CaptureSourceSelection>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureFrameMediaKind {
    Video,
    Audio,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureFrameFormat {
    Rgba8,
    Bgra8,
    Nv12,
    PcmF32,
    PcmS16,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureFrameTransport {
    Unavailable,
    SharedMemory,
    TextureHandle,
    ExternalProcess,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureFrameBindingStatus {
    Ready,
    Placeholder,
    PermissionRequired,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureProviderLifecycleState {
    Idle,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaptureFrameBinding {
    pub scene_source_id: String,
    pub scene_source_name: String,
    pub capture_source_id: Option<String>,
    pub capture_kind: CaptureSourceKind,
    pub media_kind: CaptureFrameMediaKind,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub framerate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub format: CaptureFrameFormat,
    pub transport: CaptureFrameTransport,
    pub status: CaptureFrameBindingStatus,
    pub status_detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaptureProviderStatus {
    pub provider_id: String,
    pub scene_source_id: String,
    pub scene_source_name: String,
    pub capture_source_id: Option<String>,
    pub capture_kind: CaptureSourceKind,
    pub media_kind: CaptureFrameMediaKind,
    pub lifecycle: CaptureProviderLifecycleState,
    pub binding_status: CaptureFrameBindingStatus,
    pub status_detail: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub framerate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub format: CaptureFrameFormat,
    pub frame_index: u64,
    pub dropped_frames: u64,
    pub latency_ms: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaptureVideoFramePacket {
    pub provider_id: String,
    pub scene_source_id: String,
    pub capture_source_id: Option<String>,
    pub frame_index: u64,
    pub width: u32,
    pub height: u32,
    pub format: CaptureFrameFormat,
    pub pts_nanos: u64,
    pub duration_nanos: u64,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaptureAudioPacket {
    pub provider_id: String,
    pub scene_source_id: String,
    pub capture_source_id: Option<String>,
    pub frame_index: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: CaptureFrameFormat,
    pub pts_nanos: u64,
    pub duration_nanos: u64,
    pub samples_f32: Vec<f32>,
}

pub trait CaptureProvider {
    fn status(&self) -> CaptureProviderStatus;
    fn next_video_frame(&mut self) -> Option<CaptureVideoFramePacket>;
    fn next_audio_packet(&mut self) -> Option<CaptureAudioPacket>;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaptureFramePlan {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub bindings: Vec<CaptureFrameBinding>,
    pub validation: CaptureFrameValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CaptureProviderRuntimeSnapshot {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub providers: Vec<CaptureProviderStatus>,
    pub validation: CaptureFrameValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CaptureFrameValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn default_capture_sources() -> Vec<CaptureSourceSelection> {
    vec![
        CaptureSourceSelection::display_main(),
        CaptureSourceSelection::microphone_default(),
    ]
}

pub fn build_capture_frame_plan(scene: &Scene) -> CaptureFramePlan {
    let bindings = scene
        .sources
        .iter()
        .filter(|source| source.visible)
        .filter_map(capture_frame_binding)
        .collect::<Vec<_>>();
    let mut plan = CaptureFramePlan {
        version: 1,
        scene_id: scene.id.clone(),
        scene_name: scene.name.clone(),
        bindings,
        validation: CaptureFrameValidation {
            ready: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        },
    };
    plan.validation = validate_capture_frame_plan(&plan);
    plan
}

pub fn validate_capture_frame_plan(plan: &CaptureFramePlan) -> CaptureFrameValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if plan.version == 0 {
        errors.push("capture frame plan version must be greater than zero".to_string());
    }
    if plan.scene_id.trim().is_empty() {
        errors.push("capture frame plan scene id is required".to_string());
    }
    if plan.scene_name.trim().is_empty() {
        errors.push("capture frame plan scene name is required".to_string());
    }
    if plan.bindings.is_empty() {
        warnings.push("capture frame plan has no capture-backed scene sources".to_string());
    }

    for binding in &plan.bindings {
        if binding.scene_source_id.trim().is_empty() {
            errors.push("capture frame binding scene source id is required".to_string());
        }
        if binding.scene_source_name.trim().is_empty() {
            errors.push(format!(
                "capture frame binding \"{}\" name is required",
                binding.scene_source_id
            ));
        }
        if binding.capture_source_id.is_none() {
            warnings.push(format!(
                "{} has no assigned capture source",
                binding.scene_source_name
            ));
        }
        match binding.media_kind {
            CaptureFrameMediaKind::Video => {
                validate_optional_positive(
                    binding.width,
                    &format!("{} width", binding.scene_source_id),
                    &mut errors,
                );
                validate_optional_positive(
                    binding.height,
                    &format!("{} height", binding.scene_source_id),
                    &mut errors,
                );
                validate_optional_positive(
                    binding.framerate,
                    &format!("{} framerate", binding.scene_source_id),
                    &mut errors,
                );
            }
            CaptureFrameMediaKind::Audio => {
                validate_optional_positive(
                    binding.sample_rate,
                    &format!("{} sample_rate", binding.scene_source_id),
                    &mut errors,
                );
                if matches!(binding.channels, Some(0)) {
                    errors.push(format!(
                        "{} channels must be greater than zero",
                        binding.scene_source_id
                    ));
                }
            }
        }

        match binding.status {
            CaptureFrameBindingStatus::Ready => {}
            CaptureFrameBindingStatus::Placeholder => warnings.push(format!(
                "{} is waiting for capture assignment: {}",
                binding.scene_source_name, binding.status_detail
            )),
            CaptureFrameBindingStatus::PermissionRequired => warnings.push(format!(
                "{} requires capture permission: {}",
                binding.scene_source_name, binding.status_detail
            )),
            CaptureFrameBindingStatus::Unavailable => warnings.push(format!(
                "{} capture is unavailable: {}",
                binding.scene_source_name, binding.status_detail
            )),
        }
    }

    CaptureFrameValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

pub fn build_capture_provider_runtime_snapshot(scene: &Scene) -> CaptureProviderRuntimeSnapshot {
    let plan = build_capture_frame_plan(scene);
    let providers = plan
        .bindings
        .iter()
        .map(|binding| MockCaptureProvider::new(binding.clone()).status())
        .collect::<Vec<_>>();
    let mut snapshot = CaptureProviderRuntimeSnapshot {
        version: 1,
        scene_id: scene.id.clone(),
        scene_name: scene.name.clone(),
        providers,
        validation: CaptureFrameValidation {
            ready: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        },
    };
    snapshot.validation = validate_capture_provider_runtime_snapshot(&snapshot);
    snapshot
}

pub fn validate_capture_provider_runtime_snapshot(
    snapshot: &CaptureProviderRuntimeSnapshot,
) -> CaptureFrameValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut provider_ids = std::collections::HashSet::new();

    if snapshot.version == 0 {
        errors.push("capture provider snapshot version must be greater than zero".to_string());
    }
    if snapshot.scene_id.trim().is_empty() {
        errors.push("capture provider snapshot scene id is required".to_string());
    }
    if snapshot.scene_name.trim().is_empty() {
        errors.push("capture provider snapshot scene name is required".to_string());
    }
    if snapshot.providers.is_empty() {
        warnings.push("capture provider snapshot has no providers".to_string());
    }

    for provider in &snapshot.providers {
        if !provider_ids.insert(provider.provider_id.as_str()) {
            errors.push(format!(
                "duplicate capture provider \"{}\"",
                provider.provider_id
            ));
        }
        if provider.provider_id.trim().is_empty() {
            errors.push("capture provider id is required".to_string());
        }
        if provider.scene_source_id.trim().is_empty() {
            errors.push(format!(
                "capture provider \"{}\" source id is required",
                provider.provider_id
            ));
        }
        if provider.scene_source_name.trim().is_empty() {
            errors.push(format!(
                "capture provider \"{}\" source name is required",
                provider.provider_id
            ));
        }
        match provider.media_kind {
            CaptureFrameMediaKind::Video => {
                validate_optional_positive(provider.width, &provider.provider_id, &mut errors);
                validate_optional_positive(provider.height, &provider.provider_id, &mut errors);
                validate_optional_positive(provider.framerate, &provider.provider_id, &mut errors);
            }
            CaptureFrameMediaKind::Audio => {
                validate_optional_positive(
                    provider.sample_rate,
                    &provider.provider_id,
                    &mut errors,
                );
                if matches!(provider.channels, Some(0)) {
                    errors.push(format!(
                        "{} channels must be greater than zero",
                        provider.provider_id
                    ));
                }
            }
        }
        if provider.lifecycle != CaptureProviderLifecycleState::Running {
            warnings.push(format!(
                "{} provider is {:?}: {}",
                provider.scene_source_name, provider.lifecycle, provider.status_detail
            ));
        }
    }

    CaptureFrameValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

#[derive(Clone, Debug)]
pub struct MockCaptureProvider {
    binding: CaptureFrameBinding,
    frame_index: u64,
    dropped_frames: u64,
}

impl MockCaptureProvider {
    pub fn new(binding: CaptureFrameBinding) -> Self {
        Self {
            binding,
            frame_index: 0,
            dropped_frames: 0,
        }
    }

    fn provider_id(&self) -> String {
        format!("provider:{}", self.binding.scene_source_id)
    }

    fn ready(&self) -> bool {
        self.binding.status == CaptureFrameBindingStatus::Ready
    }

    fn lifecycle(&self) -> CaptureProviderLifecycleState {
        match self.binding.status {
            CaptureFrameBindingStatus::Ready => CaptureProviderLifecycleState::Running,
            CaptureFrameBindingStatus::Placeholder => CaptureProviderLifecycleState::Idle,
            CaptureFrameBindingStatus::PermissionRequired => CaptureProviderLifecycleState::Idle,
            CaptureFrameBindingStatus::Unavailable => CaptureProviderLifecycleState::Error,
        }
    }

    fn frame_duration_nanos(&self) -> u64 {
        let framerate = self.binding.framerate.unwrap_or(60).max(1);
        1_000_000_000_u64 / u64::from(framerate)
    }

    fn audio_duration_nanos(&self, sample_frames: u32) -> u64 {
        let sample_rate = self.binding.sample_rate.unwrap_or(48_000).max(1);
        (u64::from(sample_frames) * 1_000_000_000_u64) / u64::from(sample_rate)
    }
}

impl CaptureProvider for MockCaptureProvider {
    fn status(&self) -> CaptureProviderStatus {
        CaptureProviderStatus {
            provider_id: self.provider_id(),
            scene_source_id: self.binding.scene_source_id.clone(),
            scene_source_name: self.binding.scene_source_name.clone(),
            capture_source_id: self.binding.capture_source_id.clone(),
            capture_kind: self.binding.capture_kind.clone(),
            media_kind: self.binding.media_kind.clone(),
            lifecycle: self.lifecycle(),
            binding_status: self.binding.status.clone(),
            status_detail: if self.ready() {
                "Mock capture provider is producing deterministic test frames.".to_string()
            } else {
                self.binding.status_detail.clone()
            },
            width: self.binding.width,
            height: self.binding.height,
            framerate: self.binding.framerate,
            sample_rate: self.binding.sample_rate,
            channels: self.binding.channels,
            format: self.binding.format.clone(),
            frame_index: self.frame_index,
            dropped_frames: self.dropped_frames,
            latency_ms: self.ready().then_some(0.0),
        }
    }

    fn next_video_frame(&mut self) -> Option<CaptureVideoFramePacket> {
        if !self.ready() || self.binding.media_kind != CaptureFrameMediaKind::Video {
            return None;
        }
        let width = self.binding.width.unwrap_or(1).max(1);
        let height = self.binding.height.unwrap_or(1).max(1);
        let frame_index = self.frame_index;
        let mut pixels = vec![0; width as usize * height as usize * 4];
        let kind_seed = match self.binding.capture_kind {
            CaptureSourceKind::Display => 23,
            CaptureSourceKind::Window => 53,
            CaptureSourceKind::Camera => 83,
            CaptureSourceKind::Microphone | CaptureSourceKind::SystemAudio => 113,
        };
        for y in 0..height {
            for x in 0..width {
                let offset = (y as usize * width as usize + x as usize) * 4;
                pixels[offset] = ((u64::from(x) + frame_index) % 256) as u8;
                pixels[offset + 1] = ((u64::from(y) + kind_seed) % 256) as u8;
                pixels[offset + 2] =
                    ((u64::from(x + y) + self.binding.scene_source_id.len() as u64) % 256) as u8;
                pixels[offset + 3] = 255;
            }
        }
        self.frame_index += 1;
        Some(CaptureVideoFramePacket {
            provider_id: self.provider_id(),
            scene_source_id: self.binding.scene_source_id.clone(),
            capture_source_id: self.binding.capture_source_id.clone(),
            frame_index,
            width,
            height,
            format: self.binding.format.clone(),
            pts_nanos: frame_index * self.frame_duration_nanos(),
            duration_nanos: self.frame_duration_nanos(),
            pixels,
        })
    }

    fn next_audio_packet(&mut self) -> Option<CaptureAudioPacket> {
        if !self.ready() || self.binding.media_kind != CaptureFrameMediaKind::Audio {
            return None;
        }
        let sample_rate = self.binding.sample_rate.unwrap_or(48_000).max(1);
        let channels = self.binding.channels.unwrap_or(2).max(1);
        let sample_frames = 480_u32;
        let frame_index = self.frame_index;
        let mut samples_f32 = Vec::with_capacity(sample_frames as usize * channels as usize);
        for sample_index in 0..sample_frames {
            for channel in 0..channels {
                let value =
                    ((frame_index as f32 + sample_index as f32 + f32::from(channel)) % 48.0) / 48.0
                        * 2.0
                        - 1.0;
                samples_f32.push(value * 0.25);
            }
        }
        self.frame_index += 1;
        Some(CaptureAudioPacket {
            provider_id: self.provider_id(),
            scene_source_id: self.binding.scene_source_id.clone(),
            capture_source_id: self.binding.capture_source_id.clone(),
            frame_index,
            sample_rate,
            channels,
            format: self.binding.format.clone(),
            pts_nanos: frame_index * self.audio_duration_nanos(sample_frames),
            duration_nanos: self.audio_duration_nanos(sample_frames),
            samples_f32,
        })
    }
}

fn capture_frame_binding(source: &SceneSource) -> Option<CaptureFrameBinding> {
    let capture_kind = scene_capture_kind(source)?;
    let media_kind = match capture_kind {
        CaptureSourceKind::Display | CaptureSourceKind::Window | CaptureSourceKind::Camera => {
            CaptureFrameMediaKind::Video
        }
        CaptureSourceKind::Microphone | CaptureSourceKind::SystemAudio => {
            CaptureFrameMediaKind::Audio
        }
    };
    let capture_source_id = source.capture_identity();
    let (width, height, framerate) = video_shape(source);
    let (sample_rate, channels) = audio_shape(source);
    let (status, status_detail) = capture_binding_status(source, capture_source_id.as_deref());

    Some(CaptureFrameBinding {
        scene_source_id: source.id.clone(),
        scene_source_name: source.name.clone(),
        capture_source_id,
        capture_kind,
        media_kind: media_kind.clone(),
        width: if media_kind == CaptureFrameMediaKind::Video {
            width
        } else {
            None
        },
        height: if media_kind == CaptureFrameMediaKind::Video {
            height
        } else {
            None
        },
        framerate: if media_kind == CaptureFrameMediaKind::Video {
            framerate
        } else {
            None
        },
        sample_rate: if media_kind == CaptureFrameMediaKind::Audio {
            sample_rate
        } else {
            None
        },
        channels: if media_kind == CaptureFrameMediaKind::Audio {
            channels
        } else {
            None
        },
        format: match media_kind {
            CaptureFrameMediaKind::Video => CaptureFrameFormat::Bgra8,
            CaptureFrameMediaKind::Audio => CaptureFrameFormat::PcmF32,
        },
        transport: if status == CaptureFrameBindingStatus::Ready {
            CaptureFrameTransport::SharedMemory
        } else {
            CaptureFrameTransport::Unavailable
        },
        status,
        status_detail,
    })
}

fn scene_capture_kind(source: &SceneSource) -> Option<CaptureSourceKind> {
    match source.kind {
        SceneSourceKind::Display => Some(CaptureSourceKind::Display),
        SceneSourceKind::Window => Some(CaptureSourceKind::Window),
        SceneSourceKind::Camera => Some(CaptureSourceKind::Camera),
        SceneSourceKind::AudioMeter => match source
            .config
            .get("channel")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("microphone")
        {
            "system" => Some(CaptureSourceKind::SystemAudio),
            _ => Some(CaptureSourceKind::Microphone),
        },
        _ => None,
    }
}

fn video_shape(source: &SceneSource) -> (Option<u32>, Option<u32>, Option<u32>) {
    let resolution = source.config.get("resolution");
    let width = resolution
        .and_then(|value| value.get("width"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .or_else(|| Some(source.size.width.round().max(1.0) as u32));
    let height = resolution
        .and_then(|value| value.get("height"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .or_else(|| Some(source.size.height.round().max(1.0) as u32));
    let framerate = source
        .config
        .get("framerate")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .or(Some(60));

    (width, height, framerate)
}

fn audio_shape(source: &SceneSource) -> (Option<u32>, Option<u16>) {
    let sample_rate = source
        .config
        .get("sample_rate")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .or(Some(48_000));
    let channels = source
        .config
        .get("channels")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .or(Some(2));

    (sample_rate, channels)
}

fn capture_binding_status(
    source: &SceneSource,
    capture_source_id: Option<&str>,
) -> (CaptureFrameBindingStatus, String) {
    if capture_source_id.is_none() {
        return (
            CaptureFrameBindingStatus::Placeholder,
            "No capture source has been assigned.".to_string(),
        );
    }

    let Some(availability) = source.config.get("availability") else {
        return (
            CaptureFrameBindingStatus::Ready,
            "Capture source is configured.".to_string(),
        );
    };
    let detail = availability
        .get("detail")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Capture availability has not been checked.")
        .to_string();
    let status = match availability
        .get("state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
    {
        "available" => CaptureFrameBindingStatus::Ready,
        "permission_required" => CaptureFrameBindingStatus::PermissionRequired,
        "unavailable" => CaptureFrameBindingStatus::Unavailable,
        _ => CaptureFrameBindingStatus::Placeholder,
    };

    (status, detail)
}

fn validate_optional_positive(value: Option<u32>, label: &str, errors: &mut Vec<String>) {
    if matches!(value, Some(0)) {
        errors.push(format!("{label} must be greater than zero"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_frame_plan_describes_default_scene_bindings() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let plan = build_capture_frame_plan(scene);

        assert_eq!(plan.scene_id, "scene-main");
        assert_eq!(plan.bindings.len(), 3);
        assert!(plan.validation.ready, "{:?}", plan.validation.errors);
        assert!(plan
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("requires capture permission")));

        let display = plan
            .bindings
            .iter()
            .find(|binding| binding.scene_source_id == "source-main-display")
            .unwrap();
        assert_eq!(display.capture_kind, CaptureSourceKind::Display);
        assert_eq!(display.media_kind, CaptureFrameMediaKind::Video);
        assert_eq!(display.width, Some(1920));
        assert_eq!(display.height, Some(1080));
        assert_eq!(display.format, CaptureFrameFormat::Bgra8);
        assert_eq!(display.transport, CaptureFrameTransport::Unavailable);

        let audio = plan
            .bindings
            .iter()
            .find(|binding| binding.scene_source_id == "source-mic-meter")
            .unwrap();
        assert_eq!(audio.capture_kind, CaptureSourceKind::Microphone);
        assert_eq!(audio.media_kind, CaptureFrameMediaKind::Audio);
        assert_eq!(audio.sample_rate, Some(48_000));
        assert_eq!(audio.channels, Some(2));
    }

    #[test]
    fn capture_provider_snapshot_tracks_binding_lifecycle() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let snapshot = build_capture_provider_runtime_snapshot(scene);

        assert_eq!(snapshot.scene_id, "scene-main");
        assert_eq!(snapshot.providers.len(), 3);
        assert!(snapshot.validation.ready);
        assert!(snapshot.providers.iter().any(|provider| {
            provider.capture_kind == CaptureSourceKind::Display
                && provider.lifecycle == CaptureProviderLifecycleState::Idle
        }));
    }

    #[test]
    fn mock_capture_provider_emits_deterministic_video_and_audio_packets() {
        let mut video_provider = MockCaptureProvider::new(CaptureFrameBinding {
            scene_source_id: "source-video".to_string(),
            scene_source_name: "Video".to_string(),
            capture_source_id: Some("display:test".to_string()),
            capture_kind: CaptureSourceKind::Display,
            media_kind: CaptureFrameMediaKind::Video,
            width: Some(4),
            height: Some(3),
            framerate: Some(30),
            sample_rate: None,
            channels: None,
            format: CaptureFrameFormat::Bgra8,
            transport: CaptureFrameTransport::SharedMemory,
            status: CaptureFrameBindingStatus::Ready,
            status_detail: "ready".to_string(),
        });
        assert_eq!(
            video_provider.status().lifecycle,
            CaptureProviderLifecycleState::Running
        );
        let first_video = video_provider.next_video_frame().unwrap();
        let second_video = video_provider.next_video_frame().unwrap();
        assert_eq!(first_video.width, 4);
        assert_eq!(first_video.height, 3);
        assert_eq!(first_video.pixels.len(), 4 * 3 * 4);
        assert_eq!(first_video.frame_index, 0);
        assert_eq!(second_video.frame_index, 1);
        assert_ne!(first_video.pixels, second_video.pixels);

        let mut audio_provider = MockCaptureProvider::new(CaptureFrameBinding {
            scene_source_id: "source-audio".to_string(),
            scene_source_name: "Audio".to_string(),
            capture_source_id: Some("microphone:test".to_string()),
            capture_kind: CaptureSourceKind::Microphone,
            media_kind: CaptureFrameMediaKind::Audio,
            width: None,
            height: None,
            framerate: None,
            sample_rate: Some(48_000),
            channels: Some(2),
            format: CaptureFrameFormat::PcmF32,
            transport: CaptureFrameTransport::SharedMemory,
            status: CaptureFrameBindingStatus::Ready,
            status_detail: "ready".to_string(),
        });
        let audio = audio_provider.next_audio_packet().unwrap();
        assert_eq!(audio.sample_rate, 48_000);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.samples_f32.len(), 480 * 2);
        assert!(audio.samples_f32.iter().all(|sample| sample.abs() <= 0.25));
    }
}
