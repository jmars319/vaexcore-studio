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
pub struct CaptureFramePlan {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub bindings: Vec<CaptureFrameBinding>,
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
}
