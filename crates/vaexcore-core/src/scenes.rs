use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Resolution;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScenePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneCrop {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneSourceKind {
    Display,
    Window,
    Camera,
    AudioMeter,
    ImageMedia,
    BrowserOverlay,
    Text,
    Group,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneSourceFilterKind {
    ColorCorrection,
    ChromaKey,
    CropPad,
    MaskBlend,
    Blur,
    Sharpen,
    Lut,
    AudioGain,
    NoiseGate,
    Compressor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneSourceFilter {
    pub id: String,
    pub name: String,
    pub kind: SceneSourceFilterKind,
    pub enabled: bool,
    pub order: i32,
    pub config: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneSource {
    pub id: String,
    pub name: String,
    pub kind: SceneSourceKind,
    pub position: ScenePoint,
    pub size: SceneSize,
    pub crop: SceneCrop,
    pub rotation_degrees: f64,
    pub opacity: f64,
    pub visible: bool,
    pub locked: bool,
    pub z_index: i32,
    #[serde(default)]
    pub filters: Vec<SceneSourceFilter>,
    pub config: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneCanvas {
    pub width: u32,
    pub height: u32,
    pub background_color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub canvas: SceneCanvas,
    pub sources: Vec<SceneSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneTransitionKind {
    Cut,
    Fade,
    Swipe,
    Stinger,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneTransitionEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneTransition {
    pub id: String,
    pub name: String,
    pub kind: SceneTransitionKind,
    pub duration_ms: u32,
    pub easing: SceneTransitionEasing,
    pub config: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneCollection {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub active_scene_id: String,
    #[serde(default = "default_active_transition_id")]
    pub active_transition_id: String,
    #[serde(default = "default_scene_transitions")]
    pub transitions: Vec<SceneTransition>,
    pub scenes: Vec<Scene>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneCollectionBundle {
    pub version: u32,
    pub exported_at: chrono::DateTime<chrono::Utc>,
    pub collection: SceneCollection,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneCollectionImportResult {
    pub imported_scenes: usize,
    pub imported_transitions: usize,
    pub collection: SceneCollection,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SceneValidationIssue {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SceneValidationResult {
    pub ok: bool,
    pub issues: Vec<SceneValidationIssue>,
}

impl SceneCollection {
    pub fn default_collection(now: chrono::DateTime<chrono::Utc>) -> Self {
        let scene = Scene {
            id: "scene-main".to_string(),
            name: "Main Scene".to_string(),
            canvas: SceneCanvas::default_1080p(),
            sources: vec![
                SceneSource::new(
                    "source-main-display",
                    "Main Display Placeholder",
                    SceneSourceKind::Display,
                    ScenePoint { x: 0.0, y: 0.0 },
                    SceneSize {
                        width: 1920.0,
                        height: 1080.0,
                    },
                    0,
                    json!({
                        "display_id": "display:main",
                        "resolution": { "width": 1920, "height": 1080 },
                        "capture_cursor": true,
                        "availability": {
                            "state": "permission_required",
                            "detail": "Screen Recording permission has not been verified."
                        }
                    }),
                ),
                SceneSource::new(
                    "source-camera-placeholder",
                    "Camera Placeholder",
                    SceneSourceKind::Camera,
                    ScenePoint {
                        x: 1460.0,
                        y: 700.0,
                    },
                    SceneSize {
                        width: 380.0,
                        height: 214.0,
                    },
                    10,
                    json!({
                        "device_id": null,
                        "resolution": { "width": 1280, "height": 720 },
                        "framerate": 30,
                        "availability": {
                            "state": "permission_required",
                            "detail": "Camera permission has not been verified."
                        }
                    }),
                ),
                SceneSource::new(
                    "source-mic-meter",
                    "Microphone Meter",
                    SceneSourceKind::AudioMeter,
                    ScenePoint { x: 80.0, y: 900.0 },
                    SceneSize {
                        width: 420.0,
                        height: 72.0,
                    },
                    20,
                    json!({
                        "device_id": null,
                        "channel": "microphone",
                        "meter_style": "bar",
                        "gain_db": 0.0,
                        "muted": false,
                        "monitor_enabled": false,
                        "meter_enabled": true,
                        "sync_offset_ms": 0,
                        "availability": {
                            "state": "permission_required",
                            "detail": "Microphone permission has not been verified."
                        }
                    }),
                ),
                SceneSource::new(
                    "source-alert-overlay",
                    "Alerts Browser Overlay",
                    SceneSourceKind::BrowserOverlay,
                    ScenePoint { x: 1240.0, y: 72.0 },
                    SceneSize {
                        width: 560.0,
                        height: 170.0,
                    },
                    30,
                    json!({
                        "url": null,
                        "viewport": { "width": 1280, "height": 720 },
                        "custom_css": null,
                        "availability": {
                            "state": "unavailable",
                            "detail": "No browser overlay URL has been configured."
                        }
                    }),
                ),
                SceneSource::new(
                    "source-title-text",
                    "Scene Title",
                    SceneSourceKind::Text,
                    ScenePoint { x: 640.0, y: 84.0 },
                    SceneSize {
                        width: 640.0,
                        height: 110.0,
                    },
                    40,
                    json!({
                        "text": "vaexcore studio",
                        "font_family": "Inter",
                        "font_size": 64,
                        "color": "#f4f8ff",
                        "align": "center"
                    }),
                ),
            ],
        };

        Self {
            id: "collection-default".to_string(),
            name: "Default Studio Scenes".to_string(),
            version: 1,
            active_scene_id: scene.id.clone(),
            active_transition_id: default_active_transition_id(),
            transitions: default_scene_transitions(),
            scenes: vec![scene],
            created_at: now,
            updated_at: now,
        }
    }

    pub fn active_scene(&self) -> Option<&Scene> {
        self.scenes
            .iter()
            .find(|scene| scene.id == self.active_scene_id)
            .or_else(|| self.scenes.first())
    }

    pub fn active_transition(&self) -> Option<&SceneTransition> {
        self.transitions
            .iter()
            .find(|transition| transition.id == self.active_transition_id)
            .or_else(|| self.transitions.first())
    }

    pub fn validation(&self) -> SceneValidationResult {
        validate_scene_collection(self)
    }
}

impl SceneCollectionBundle {
    pub fn new(collection: SceneCollection, exported_at: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            version: 1,
            exported_at,
            collection,
        }
    }
}

impl SceneCanvas {
    pub fn default_1080p() -> Self {
        Self {
            width: 1920,
            height: 1080,
            background_color: "#050711".to_string(),
        }
    }
}

fn default_active_transition_id() -> String {
    "transition-fade".to_string()
}

fn default_scene_transitions() -> Vec<SceneTransition> {
    vec![
        SceneTransition {
            id: "transition-cut".to_string(),
            name: "Cut".to_string(),
            kind: SceneTransitionKind::Cut,
            duration_ms: 0,
            easing: SceneTransitionEasing::Linear,
            config: json!({}),
        },
        SceneTransition {
            id: "transition-fade".to_string(),
            name: "Fade".to_string(),
            kind: SceneTransitionKind::Fade,
            duration_ms: 300,
            easing: SceneTransitionEasing::EaseInOut,
            config: json!({ "color": "#000000" }),
        },
    ]
}

impl SceneSource {
    fn new(
        id: &str,
        name: &str,
        kind: SceneSourceKind,
        position: ScenePoint,
        size: SceneSize,
        z_index: i32,
        config: serde_json::Value,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            kind,
            position,
            size,
            crop: SceneCrop {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
            rotation_degrees: 0.0,
            opacity: 1.0,
            visible: true,
            locked: false,
            z_index,
            filters: Vec::new(),
            config,
        }
    }

    pub fn capture_identity(&self) -> Option<String> {
        match self.kind {
            SceneSourceKind::Display => self.config_string("display_id"),
            SceneSourceKind::Window => self.config_string("window_id"),
            SceneSourceKind::Camera | SceneSourceKind::AudioMeter => {
                self.config_string("device_id")
            }
            _ => None,
        }
    }

    fn config_string(&self, key: &str) -> Option<String> {
        self.config
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }
}

pub fn validate_scene_collection(collection: &SceneCollection) -> SceneValidationResult {
    let mut issues = Vec::new();
    let mut scene_ids = HashSet::new();

    if collection.id.trim().is_empty() {
        issue(&mut issues, "id", "Scene collection id is required.");
    }
    if collection.name.trim().is_empty() {
        issue(&mut issues, "name", "Scene collection name is required.");
    }
    if collection.version == 0 {
        issue(
            &mut issues,
            "version",
            "Scene collection version must be greater than zero.",
        );
    }
    if collection.scenes.is_empty() {
        issue(&mut issues, "scenes", "At least one scene is required.");
    }

    for (scene_index, scene) in collection.scenes.iter().enumerate() {
        let scene_path = format!("scenes[{scene_index}]");
        if !scene_ids.insert(scene.id.as_str()) {
            issue(
                &mut issues,
                format!("{scene_path}.id"),
                format!("Duplicate scene id \"{}\".", scene.id),
            );
        }
        if scene.name.trim().is_empty() {
            issue(
                &mut issues,
                format!("{scene_path}.name"),
                "Scene name is required.",
            );
        }
        if scene.canvas.width == 0 {
            issue(
                &mut issues,
                format!("{scene_path}.canvas.width"),
                "Canvas width must be greater than zero.",
            );
        }
        if scene.canvas.height == 0 {
            issue(
                &mut issues,
                format!("{scene_path}.canvas.height"),
                "Canvas height must be greater than zero.",
            );
        }
        validate_scene_sources(scene, &scene_path, &mut issues);
    }

    validate_scene_transitions(collection, &mut issues);

    if !collection.scenes.is_empty()
        && !collection
            .scenes
            .iter()
            .any(|scene| scene.id == collection.active_scene_id)
    {
        issue(
            &mut issues,
            "active_scene_id",
            "Active scene id must match a scene in the collection.",
        );
    }

    SceneValidationResult {
        ok: issues.is_empty(),
        issues,
    }
}

fn validate_scene_sources(scene: &Scene, scene_path: &str, issues: &mut Vec<SceneValidationIssue>) {
    let mut source_ids = HashSet::new();
    let visible_sources = scene.sources.iter().filter(|source| source.visible).count();
    if visible_sources == 0 {
        issue(
            issues,
            format!("{scene_path}.sources"),
            "Scene must contain at least one visible source.",
        );
    }

    for (source_index, source) in scene.sources.iter().enumerate() {
        let source_path = format!("{scene_path}.sources[{source_index}]");
        if !source_ids.insert(source.id.as_str()) {
            issue(
                issues,
                format!("{source_path}.id"),
                format!("Duplicate source id \"{}\".", source.id),
            );
        }
        if source.name.trim().is_empty() {
            issue(
                issues,
                format!("{source_path}.name"),
                "Source name is required.",
            );
        }
        finite(
            source.position.x,
            format!("{source_path}.position.x"),
            issues,
        );
        finite(
            source.position.y,
            format!("{source_path}.position.y"),
            issues,
        );
        positive(
            source.size.width,
            format!("{source_path}.size.width"),
            issues,
        );
        positive(
            source.size.height,
            format!("{source_path}.size.height"),
            issues,
        );
        non_negative(source.crop.top, format!("{source_path}.crop.top"), issues);
        non_negative(
            source.crop.right,
            format!("{source_path}.crop.right"),
            issues,
        );
        non_negative(
            source.crop.bottom,
            format!("{source_path}.crop.bottom"),
            issues,
        );
        non_negative(source.crop.left, format!("{source_path}.crop.left"), issues);
        finite(
            source.rotation_degrees,
            format!("{source_path}.rotation_degrees"),
            issues,
        );
        if !source.opacity.is_finite() || source.opacity < 0.0 || source.opacity > 1.0 {
            issue(
                issues,
                format!("{source_path}.opacity"),
                "Source opacity must be between 0 and 1.",
            );
        }
        validate_source_filters(&source.filters, &source_path, issues);
    }
}

fn validate_source_filters(
    filters: &[SceneSourceFilter],
    source_path: &str,
    issues: &mut Vec<SceneValidationIssue>,
) {
    let mut filter_ids = HashSet::new();
    for (filter_index, filter) in filters.iter().enumerate() {
        let filter_path = format!("{source_path}.filters[{filter_index}]");
        if !filter_ids.insert(filter.id.as_str()) {
            issue(
                issues,
                format!("{filter_path}.id"),
                format!("Duplicate source filter id \"{}\".", filter.id),
            );
        }
        if filter.id.trim().is_empty() {
            issue(
                issues,
                format!("{filter_path}.id"),
                "Source filter id is required.",
            );
        }
        if filter.name.trim().is_empty() {
            issue(
                issues,
                format!("{filter_path}.name"),
                "Source filter name is required.",
            );
        }
    }
}

fn validate_scene_transitions(
    collection: &SceneCollection,
    issues: &mut Vec<SceneValidationIssue>,
) {
    let mut transition_ids = HashSet::new();
    if collection.transitions.is_empty() {
        issue(
            issues,
            "transitions",
            "At least one scene transition is required.",
        );
    }

    for (transition_index, transition) in collection.transitions.iter().enumerate() {
        let transition_path = format!("transitions[{transition_index}]");
        if !transition_ids.insert(transition.id.as_str()) {
            issue(
                issues,
                format!("{transition_path}.id"),
                format!("Duplicate transition id \"{}\".", transition.id),
            );
        }
        if transition.id.trim().is_empty() {
            issue(
                issues,
                format!("{transition_path}.id"),
                "Transition id is required.",
            );
        }
        if transition.name.trim().is_empty() {
            issue(
                issues,
                format!("{transition_path}.name"),
                "Transition name is required.",
            );
        }
        if transition.duration_ms > 60_000 {
            issue(
                issues,
                format!("{transition_path}.duration_ms"),
                "Transition duration must be 60 seconds or less.",
            );
        }
        if transition.kind == SceneTransitionKind::Cut && transition.duration_ms != 0 {
            issue(
                issues,
                format!("{transition_path}.duration_ms"),
                "Cut transitions must use a zero millisecond duration.",
            );
        }
    }

    if !collection.transitions.is_empty()
        && !collection
            .transitions
            .iter()
            .any(|transition| transition.id == collection.active_transition_id)
    {
        issue(
            issues,
            "active_transition_id",
            "Active transition id must match a transition in the collection.",
        );
    }
}

fn finite(value: f64, path: String, issues: &mut Vec<SceneValidationIssue>) {
    if !value.is_finite() {
        issue(issues, path, "Value must be a finite number.");
    }
}

fn positive(value: f64, path: String, issues: &mut Vec<SceneValidationIssue>) {
    if !value.is_finite() || value <= 0.0 {
        issue(issues, path, "Value must be greater than zero.");
    }
}

fn non_negative(value: f64, path: String, issues: &mut Vec<SceneValidationIssue>) {
    if !value.is_finite() || value < 0.0 {
        issue(issues, path, "Value must be zero or greater.");
    }
}

fn issue(
    issues: &mut Vec<SceneValidationIssue>,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(SceneValidationIssue {
        path: path.into(),
        message: message.into(),
    });
}

pub fn scene_capture_sources(scene: &Scene) -> Vec<crate::CaptureSourceSelection> {
    scene
        .sources
        .iter()
        .filter(|source| source.visible)
        .filter_map(|source| {
            let id = source.capture_identity()?;
            let kind = match source.kind {
                SceneSourceKind::Display => crate::CaptureSourceKind::Display,
                SceneSourceKind::Window => crate::CaptureSourceKind::Window,
                SceneSourceKind::Camera => crate::CaptureSourceKind::Camera,
                SceneSourceKind::AudioMeter => crate::CaptureSourceKind::Microphone,
                _ => return None,
            };
            Some(crate::CaptureSourceSelection {
                id,
                kind,
                name: source.name.clone(),
                enabled: true,
            })
        })
        .collect()
}

pub fn scene_resolution(scene: &Scene) -> Resolution {
    Resolution {
        width: scene.canvas.width,
        height: scene.canvas.height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scene_collection_is_valid() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let validation = collection.validation();

        assert!(validation.ok, "{:?}", validation.issues);
        assert_eq!(collection.active_scene().unwrap().sources.len(), 5);
        assert_eq!(
            collection.active_transition().unwrap().id,
            "transition-fade"
        );
    }

    #[test]
    fn scene_validation_rejects_duplicate_source_ids() {
        let mut collection = SceneCollection::default_collection(crate::now_utc());
        let scene = collection.scenes.first_mut().unwrap();
        scene.sources[1].id = scene.sources[0].id.clone();
        scene.sources[0].opacity = 2.0;
        scene.sources[0].filters = vec![
            SceneSourceFilter {
                id: "filter-duplicate".to_string(),
                name: "Color".to_string(),
                kind: SceneSourceFilterKind::ColorCorrection,
                enabled: true,
                order: 0,
                config: json!({ "brightness": 0.1 }),
            },
            SceneSourceFilter {
                id: "filter-duplicate".to_string(),
                name: "Chroma".to_string(),
                kind: SceneSourceFilterKind::ChromaKey,
                enabled: false,
                order: 10,
                config: json!({ "key_color": "#00ff00" }),
            },
        ];

        let validation = collection.validation();

        assert!(!validation.ok);
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.message.contains("Duplicate source id")));
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.path.ends_with("opacity")));
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.message.contains("Duplicate source filter")));
    }

    #[test]
    fn scene_validation_rejects_invalid_transitions() {
        let mut collection = SceneCollection::default_collection(crate::now_utc());
        collection.active_transition_id = "missing-transition".to_string();
        collection.transitions[0].id = collection.transitions[1].id.clone();
        collection.transitions[0].duration_ms = 120;

        let validation = collection.validation();

        assert!(!validation.ok);
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.path == "active_transition_id"));
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.message.contains("Duplicate transition")));
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.message.contains("Cut transitions")));
    }
}
