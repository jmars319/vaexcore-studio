use std::collections::{HashMap, HashSet};

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

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SceneSourceBoundsMode {
    Stretch,
    Fit,
    Fill,
    Center,
    OriginalSize,
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
    #[serde(default = "default_scene_source_bounds_mode")]
    pub bounds_mode: SceneSourceBoundsMode,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneTransitionPreviewSample {
    pub frame_index: u32,
    pub elapsed_ms: u32,
    pub linear_progress: f64,
    pub eased_progress: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneTransitionPreviewPlan {
    pub version: u32,
    pub transition: SceneTransition,
    pub from_scene_id: String,
    pub from_scene_name: String,
    pub to_scene_id: String,
    pub to_scene_name: String,
    pub framerate: u32,
    pub duration_ms: u32,
    pub frame_count: u32,
    pub sample_frames: Vec<SceneTransitionPreviewSample>,
    pub validation: SceneTransitionPreviewValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SceneTransitionPreviewValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
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
                        "refresh_interval_ms": 1000,
                        "reload_token": 0,
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

fn default_scene_source_bounds_mode() -> SceneSourceBoundsMode {
    SceneSourceBoundsMode::Stretch
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
            bounds_mode: SceneSourceBoundsMode::Stretch,
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

pub fn build_scene_transition_preview_plan(
    collection: &SceneCollection,
    from_scene_id: Option<&str>,
    to_scene_id: Option<&str>,
    framerate: u32,
) -> SceneTransitionPreviewPlan {
    let fallback_scene = collection
        .active_scene()
        .or_else(|| collection.scenes.first());
    let from_scene = from_scene_id
        .and_then(|id| collection.scenes.iter().find(|scene| scene.id == id))
        .or(fallback_scene);
    let to_scene = to_scene_id
        .and_then(|id| collection.scenes.iter().find(|scene| scene.id == id))
        .or(fallback_scene);
    let transition = collection
        .active_transition()
        .cloned()
        .unwrap_or_else(|| default_scene_transitions().into_iter().next().unwrap());
    let frame_count = transition_frame_count(transition.duration_ms, framerate);
    let mut plan = SceneTransitionPreviewPlan {
        version: 1,
        from_scene_id: from_scene.map(|scene| scene.id.clone()).unwrap_or_default(),
        from_scene_name: from_scene
            .map(|scene| scene.name.clone())
            .unwrap_or_default(),
        to_scene_id: to_scene.map(|scene| scene.id.clone()).unwrap_or_default(),
        to_scene_name: to_scene.map(|scene| scene.name.clone()).unwrap_or_default(),
        framerate,
        duration_ms: transition.duration_ms,
        frame_count,
        sample_frames: transition_sample_frames(&transition, frame_count, framerate),
        transition,
        validation: SceneTransitionPreviewValidation {
            ready: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        },
    };
    plan.validation = validate_scene_transition_preview_plan(&plan);
    plan
}

pub fn validate_scene_transition_preview_plan(
    plan: &SceneTransitionPreviewPlan,
) -> SceneTransitionPreviewValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if plan.version == 0 {
        errors.push("transition preview plan version must be greater than zero".to_string());
    }
    if plan.transition.id.trim().is_empty() {
        errors.push("transition preview transition id is required".to_string());
    }
    if plan.from_scene_id.trim().is_empty() {
        errors.push("transition preview from scene id is required".to_string());
    }
    if plan.to_scene_id.trim().is_empty() {
        errors.push("transition preview to scene id is required".to_string());
    }
    if plan.framerate == 0 {
        errors.push("transition preview framerate must be greater than zero".to_string());
    }
    if plan.frame_count == 0 {
        errors.push("transition preview frame count must be greater than zero".to_string());
    }
    if plan.duration_ms > 60_000 {
        errors.push("transition preview duration must be 60 seconds or less".to_string());
    }
    if plan.transition.kind == SceneTransitionKind::Cut && plan.duration_ms != 0 {
        errors.push("cut transition preview duration must be zero".to_string());
    }
    if plan.from_scene_id == plan.to_scene_id {
        warnings.push("transition preview uses the same from and to scene".to_string());
    }

    for sample in &plan.sample_frames {
        if sample.linear_progress < 0.0 || sample.linear_progress > 1.0 {
            errors.push("transition preview linear progress must be 0-1".to_string());
        }
        if sample.eased_progress < 0.0 || sample.eased_progress > 1.0 {
            errors.push("transition preview eased progress must be 0-1".to_string());
        }
    }

    SceneTransitionPreviewValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

fn transition_frame_count(duration_ms: u32, framerate: u32) -> u32 {
    if duration_ms == 0 || framerate == 0 {
        return 1;
    }
    (u64::from(duration_ms) * u64::from(framerate)).div_ceil(1_000) as u32
}

fn transition_sample_frames(
    transition: &SceneTransition,
    frame_count: u32,
    framerate: u32,
) -> Vec<SceneTransitionPreviewSample> {
    let mut indices = vec![0, frame_count / 2, frame_count.saturating_sub(1)];
    indices.sort_unstable();
    indices.dedup();
    indices
        .into_iter()
        .map(|frame_index| {
            let linear_progress = if frame_count <= 1 {
                1.0
            } else {
                f64::from(frame_index) / f64::from(frame_count - 1)
            };
            SceneTransitionPreviewSample {
                frame_index,
                elapsed_ms: if framerate == 0 {
                    0
                } else {
                    ((u64::from(frame_index) * 1_000) / u64::from(framerate)) as u32
                },
                linear_progress,
                eased_progress: transition_eased_progress(linear_progress, &transition.easing),
            }
        })
        .collect()
}

fn transition_eased_progress(progress: f64, easing: &SceneTransitionEasing) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    match easing {
        SceneTransitionEasing::Linear => progress,
        SceneTransitionEasing::EaseIn => progress * progress,
        SceneTransitionEasing::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
        SceneTransitionEasing::EaseInOut => {
            if progress < 0.5 {
                2.0 * progress * progress
            } else {
                1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0
            }
        }
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

    validate_group_source_children(scene, scene_path, &source_ids, issues);
}

fn validate_group_source_children(
    scene: &Scene,
    scene_path: &str,
    source_ids: &HashSet<&str>,
    issues: &mut Vec<SceneValidationIssue>,
) {
    let group_ids = scene
        .sources
        .iter()
        .filter(|source| source.kind == SceneSourceKind::Group)
        .map(|source| source.id.as_str())
        .collect::<HashSet<_>>();
    let mut parent_by_child = HashMap::<String, String>::new();
    let mut children_by_group = HashMap::<String, Vec<String>>::new();

    for (source_index, source) in scene.sources.iter().enumerate() {
        if source.kind != SceneSourceKind::Group {
            continue;
        }

        let source_path = format!("{scene_path}.sources[{source_index}]");
        let Some(children) = source
            .config
            .get("child_source_ids")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        let mut child_ids = HashSet::new();

        for (child_index, child) in children.iter().enumerate() {
            let child_path = format!("{source_path}.config.child_source_ids[{child_index}]");
            let Some(child_id) = child.as_str().map(str::trim).filter(|id| !id.is_empty()) else {
                issue(issues, child_path, "Group child source id is required.");
                continue;
            };

            if !child_ids.insert(child_id.to_string()) {
                issue(
                    issues,
                    child_path.clone(),
                    format!("Duplicate group child source id \"{child_id}\"."),
                );
            }
            if child_id == source.id {
                issue(issues, child_path.clone(), "Group cannot contain itself.");
            }
            if !source_ids.contains(child_id) {
                issue(
                    issues,
                    child_path.clone(),
                    format!("Group child source id \"{child_id}\" does not exist."),
                );
            }
            if let Some(existing_parent) =
                parent_by_child.insert(child_id.to_string(), source.id.clone())
            {
                issue(
                    issues,
                    child_path.clone(),
                    format!("Source \"{child_id}\" is already grouped by \"{existing_parent}\"."),
                );
            }
            if group_ids.contains(child_id) {
                children_by_group
                    .entry(source.id.clone())
                    .or_default()
                    .push(child_id.to_string());
            }
        }
    }

    let mut visited = HashSet::new();
    for group_id in group_ids {
        let mut visiting = HashSet::new();
        if group_has_cycle(group_id, &children_by_group, &mut visiting, &mut visited) {
            issue(
                issues,
                format!("{scene_path}.sources"),
                format!("Group source \"{group_id}\" creates a cycle."),
            );
        }
    }
}

fn group_has_cycle(
    group_id: &str,
    children_by_group: &HashMap<String, Vec<String>>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> bool {
    if visited.contains(group_id) {
        return false;
    }
    if !visiting.insert(group_id.to_string()) {
        return true;
    }

    if let Some(children) = children_by_group.get(group_id) {
        for child_id in children {
            if group_has_cycle(child_id, children_by_group, visiting, visited) {
                return true;
            }
        }
    }

    visiting.remove(group_id);
    visited.insert(group_id.to_string());
    false
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
        validate_source_filter_config(filter, &filter_path, issues);
    }
}

fn validate_source_filter_config(
    filter: &SceneSourceFilter,
    filter_path: &str,
    issues: &mut Vec<SceneValidationIssue>,
) {
    match filter.kind {
        SceneSourceFilterKind::ColorCorrection => {
            config_number_range(filter, filter_path, "brightness", -1.0, 1.0, issues);
            config_number_range(filter, filter_path, "contrast", 0.0, 4.0, issues);
            config_number_range(filter, filter_path, "saturation", 0.0, 4.0, issues);
            config_number_range(filter, filter_path, "gamma", 0.01, 4.0, issues);
        }
        SceneSourceFilterKind::ChromaKey => {
            config_required_string(filter, filter_path, "key_color", issues);
            config_number_range(filter, filter_path, "similarity", 0.0, 1.0, issues);
            config_number_range(filter, filter_path, "smoothness", 0.0, 1.0, issues);
        }
        SceneSourceFilterKind::CropPad => {
            for key in ["top", "right", "bottom", "left"] {
                config_number_range(filter, filter_path, key, 0.0, 100_000.0, issues);
            }
        }
        SceneSourceFilterKind::MaskBlend => {
            config_optional_uri(filter, filter_path, "mask_uri", issues);
            config_string_enum(
                filter,
                filter_path,
                "blend_mode",
                &["normal", "multiply", "screen", "overlay", "alpha"],
                issues,
            );
        }
        SceneSourceFilterKind::Blur => {
            config_number_range(filter, filter_path, "radius", 0.0, 100.0, issues);
        }
        SceneSourceFilterKind::Sharpen => {
            config_number_range(filter, filter_path, "amount", 0.0, 5.0, issues);
        }
        SceneSourceFilterKind::Lut => {
            config_optional_uri(filter, filter_path, "lut_uri", issues);
            config_number_range(filter, filter_path, "strength", 0.0, 1.0, issues);
        }
        SceneSourceFilterKind::AudioGain => {
            config_number_range(filter, filter_path, "gain_db", -60.0, 24.0, issues);
        }
        SceneSourceFilterKind::NoiseGate => {
            let close = config_number_range(
                filter,
                filter_path,
                "close_threshold_db",
                -100.0,
                0.0,
                issues,
            );
            let open = config_number_range(
                filter,
                filter_path,
                "open_threshold_db",
                -100.0,
                0.0,
                issues,
            );
            if let (Some(close), Some(open)) = (close, open) {
                if close >= open {
                    issue(
                        issues,
                        format!("{filter_path}.config.open_threshold_db"),
                        "Noise gate open threshold must be greater than close threshold.",
                    );
                }
            }
            config_number_range(filter, filter_path, "attack_ms", 0.0, 5_000.0, issues);
            config_number_range(filter, filter_path, "release_ms", 0.0, 5_000.0, issues);
        }
        SceneSourceFilterKind::Compressor => {
            config_number_range(filter, filter_path, "threshold_db", -100.0, 0.0, issues);
            config_number_range(filter, filter_path, "ratio", 1.0, 20.0, issues);
            config_number_range(filter, filter_path, "attack_ms", 0.0, 5_000.0, issues);
            config_number_range(filter, filter_path, "release_ms", 0.0, 5_000.0, issues);
            config_number_range(filter, filter_path, "makeup_gain_db", -24.0, 24.0, issues);
        }
    }
}

fn config_number_range(
    filter: &SceneSourceFilter,
    filter_path: &str,
    key: &str,
    min: f64,
    max: f64,
    issues: &mut Vec<SceneValidationIssue>,
) -> Option<f64> {
    let path = format!("{filter_path}.config.{key}");
    let Some(value) = filter.config.get(key).and_then(serde_json::Value::as_f64) else {
        issue(
            issues,
            path,
            format!("Filter config {key} must be a number."),
        );
        return None;
    };
    if !value.is_finite() || value < min || value > max {
        issue(
            issues,
            path,
            format!("Filter config {key} must be between {min} and {max}."),
        );
        return None;
    }
    Some(value)
}

fn config_required_string(
    filter: &SceneSourceFilter,
    filter_path: &str,
    key: &str,
    issues: &mut Vec<SceneValidationIssue>,
) {
    let path = format!("{filter_path}.config.{key}");
    if filter
        .config
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        issue(issues, path, format!("Filter config {key} is required."));
    }
}

fn config_optional_uri(
    filter: &SceneSourceFilter,
    filter_path: &str,
    key: &str,
    issues: &mut Vec<SceneValidationIssue>,
) {
    let Some(value) = filter.config.get(key) else {
        return;
    };
    if value.is_null() {
        return;
    }
    if value
        .as_str()
        .map(str::trim)
        .filter(|uri| !uri.is_empty())
        .is_none()
    {
        issue(
            issues,
            format!("{filter_path}.config.{key}"),
            format!("Filter config {key} must be null or a non-empty string."),
        );
    }
}

fn config_string_enum(
    filter: &SceneSourceFilter,
    filter_path: &str,
    key: &str,
    allowed: &[&str],
    issues: &mut Vec<SceneValidationIssue>,
) {
    let path = format!("{filter_path}.config.{key}");
    let Some(value) = filter
        .config
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
    else {
        issue(issues, path, format!("Filter config {key} is required."));
        return;
    };
    if !allowed.contains(&value) {
        issue(
            issues,
            path,
            format!(
                "Filter config {key} must be one of: {}.",
                allowed.join(", ")
            ),
        );
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
                config: json!({
                    "brightness": 0.1,
                    "contrast": 1.0,
                    "saturation": 1.0,
                    "gamma": 1.0
                }),
            },
            SceneSourceFilter {
                id: "filter-duplicate".to_string(),
                name: "Chroma".to_string(),
                kind: SceneSourceFilterKind::ChromaKey,
                enabled: false,
                order: 10,
                config: json!({
                    "key_color": "#00ff00",
                    "similarity": 0.25,
                    "smoothness": 0.08
                }),
            },
            SceneSourceFilter {
                id: "filter-invalid-config".to_string(),
                name: "Hot Gain".to_string(),
                kind: SceneSourceFilterKind::AudioGain,
                enabled: true,
                order: 20,
                config: json!({ "gain_db": 99.0 }),
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
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.message.contains("gain_db")));
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

    #[test]
    fn scene_validation_rejects_invalid_group_children() {
        let mut collection = SceneCollection::default_collection(crate::now_utc());
        let scene = collection.scenes.first_mut().unwrap();
        scene.sources.push(SceneSource::new(
            "source-group-a",
            "Group A",
            SceneSourceKind::Group,
            ScenePoint { x: 100.0, y: 100.0 },
            SceneSize {
                width: 400.0,
                height: 300.0,
            },
            50,
            json!({
                "child_source_ids": [
                    "source-camera-placeholder",
                    "source-camera-placeholder",
                    "missing-source",
                    "source-group-b"
                ]
            }),
        ));
        scene.sources.push(SceneSource::new(
            "source-group-b",
            "Group B",
            SceneSourceKind::Group,
            ScenePoint { x: 24.0, y: 24.0 },
            SceneSize {
                width: 200.0,
                height: 120.0,
            },
            60,
            json!({
                "child_source_ids": ["source-group-a", "source-group-b"]
            }),
        ));

        let validation = collection.validation();
        let messages = validation
            .issues
            .iter()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(!validation.ok);
        assert!(messages.contains("Duplicate group child"));
        assert!(messages.contains("does not exist"));
        assert!(messages.contains("Group cannot contain itself"));
        assert!(messages.contains("creates a cycle"));
    }

    #[test]
    fn transition_preview_plan_samples_easing_curve() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let plan = build_scene_transition_preview_plan(&collection, None, None, 60);

        assert!(plan.validation.ready, "{:?}", plan.validation.errors);
        assert_eq!(plan.transition.id, "transition-fade");
        assert_eq!(plan.duration_ms, 300);
        assert_eq!(plan.frame_count, 18);
        assert_eq!(plan.sample_frames.len(), 3);
        assert_eq!(plan.sample_frames[0].linear_progress, 0.0);
        assert_eq!(plan.sample_frames[2].linear_progress, 1.0);
        assert!(plan.sample_frames[1].eased_progress > 0.0);
        assert!(plan
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("same from and to scene")));
    }
}
