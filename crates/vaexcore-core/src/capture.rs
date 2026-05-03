use serde::{Deserialize, Serialize};

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

pub fn default_capture_sources() -> Vec<CaptureSourceSelection> {
    vec![
        CaptureSourceSelection::display_main(),
        CaptureSourceSelection::microphone_default(),
    ]
}
