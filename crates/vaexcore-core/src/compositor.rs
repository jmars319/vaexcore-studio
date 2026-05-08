use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{Scene, SceneCrop, ScenePoint, SceneSize, SceneSource, SceneSourceKind};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompositorNodeRole {
    Video,
    Audio,
    Overlay,
    Text,
    Group,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompositorNodeStatus {
    Ready,
    Placeholder,
    PermissionRequired,
    Unavailable,
    Hidden,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompositorBlendMode {
    Normal,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompositorScaleMode {
    Stretch,
    Fit,
    Fill,
    OriginalSize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorOutput {
    pub width: u32,
    pub height: u32,
    pub background_color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorTransform {
    pub position: ScenePoint,
    pub size: SceneSize,
    pub crop: SceneCrop,
    pub rotation_degrees: f64,
    pub opacity: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorNode {
    pub id: String,
    pub source_id: String,
    pub name: String,
    pub source_kind: SceneSourceKind,
    pub role: CompositorNodeRole,
    pub transform: CompositorTransform,
    pub visible: bool,
    pub locked: bool,
    pub z_index: i32,
    pub blend_mode: CompositorBlendMode,
    pub scale_mode: CompositorScaleMode,
    pub status: CompositorNodeStatus,
    pub status_detail: String,
    pub config: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorGraph {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub output: CompositorOutput,
    pub nodes: Vec<CompositorNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompositorRendererKind {
    Contract,
    Software,
    Gpu,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompositorRenderTargetKind {
    Preview,
    Program,
    Recording,
    Stream,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompositorFrameFormat {
    Rgba8,
    Bgra8,
    Nv12,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorRenderTarget {
    pub id: String,
    pub name: String,
    pub kind: CompositorRenderTargetKind,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub frame_format: CompositorFrameFormat,
    pub scale_mode: CompositorScaleMode,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorRenderPlan {
    pub version: u32,
    pub renderer: CompositorRendererKind,
    pub graph: CompositorGraph,
    pub targets: Vec<CompositorRenderTarget>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CompositorFrameClock {
    pub frame_index: u64,
    pub framerate: u32,
    pub pts_nanos: u64,
    pub duration_nanos: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorEvaluatedNode {
    pub node_id: String,
    pub source_id: String,
    pub name: String,
    pub role: CompositorNodeRole,
    pub status: CompositorNodeStatus,
    pub rect: CompositorRect,
    pub crop: SceneCrop,
    pub rotation_degrees: f64,
    pub opacity: f64,
    pub z_index: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorRenderedTarget {
    pub target_id: String,
    pub target_kind: CompositorRenderTargetKind,
    pub width: u32,
    pub height: u32,
    pub frame_format: CompositorFrameFormat,
    pub nodes: Vec<CompositorEvaluatedNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompositorRenderedFrame {
    pub renderer: CompositorRendererKind,
    pub scene_id: String,
    pub scene_name: String,
    pub clock: CompositorFrameClock,
    pub targets: Vec<CompositorRenderedTarget>,
    pub validation: CompositorValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CompositorValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn build_compositor_graph(scene: &Scene) -> CompositorGraph {
    let mut sources = scene.sources.iter().collect::<Vec<_>>();
    sources.sort_by(|left, right| {
        left.z_index
            .cmp(&right.z_index)
            .then_with(|| left.id.cmp(&right.id))
    });

    CompositorGraph {
        version: 1,
        scene_id: scene.id.clone(),
        scene_name: scene.name.clone(),
        output: CompositorOutput {
            width: scene.canvas.width,
            height: scene.canvas.height,
            background_color: scene.canvas.background_color.clone(),
        },
        nodes: sources.into_iter().map(build_compositor_node).collect(),
    }
}

pub fn validate_compositor_graph(graph: &CompositorGraph) -> CompositorValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut node_ids = HashSet::new();
    let visible_nodes = graph.nodes.iter().filter(|node| node.visible).count();

    if graph.version == 0 {
        errors.push("compositor graph version must be greater than zero".to_string());
    }
    if graph.scene_id.trim().is_empty() {
        errors.push("compositor graph scene id is required".to_string());
    }
    if graph.scene_name.trim().is_empty() {
        errors.push("compositor graph scene name is required".to_string());
    }
    if graph.output.width == 0 || graph.output.height == 0 {
        errors.push("compositor output dimensions must be greater than zero".to_string());
    }
    if graph.nodes.is_empty() {
        errors.push("compositor graph must contain at least one node".to_string());
    }
    if visible_nodes == 0 {
        errors.push("compositor graph must contain at least one visible node".to_string());
    }

    for node in &graph.nodes {
        if !node_ids.insert(node.id.as_str()) {
            errors.push(format!("duplicate compositor node id \"{}\"", node.id));
        }
        if node.source_id.trim().is_empty() {
            errors.push(format!("compositor node \"{}\" has no source id", node.id));
        }
        if node.name.trim().is_empty() {
            errors.push(format!("compositor node \"{}\" has no name", node.id));
        }
        validate_finite(
            node.transform.position.x,
            &format!("node {} position.x", node.id),
            &mut errors,
        );
        validate_finite(
            node.transform.position.y,
            &format!("node {} position.y", node.id),
            &mut errors,
        );
        validate_positive(
            node.transform.size.width,
            &format!("node {} size.width", node.id),
            &mut errors,
        );
        validate_positive(
            node.transform.size.height,
            &format!("node {} size.height", node.id),
            &mut errors,
        );
        validate_non_negative(
            node.transform.crop.top,
            &format!("node {} crop.top", node.id),
            &mut errors,
        );
        validate_non_negative(
            node.transform.crop.right,
            &format!("node {} crop.right", node.id),
            &mut errors,
        );
        validate_non_negative(
            node.transform.crop.bottom,
            &format!("node {} crop.bottom", node.id),
            &mut errors,
        );
        validate_non_negative(
            node.transform.crop.left,
            &format!("node {} crop.left", node.id),
            &mut errors,
        );
        validate_finite(
            node.transform.rotation_degrees,
            &format!("node {} rotation", node.id),
            &mut errors,
        );
        if !node.transform.opacity.is_finite()
            || node.transform.opacity < 0.0
            || node.transform.opacity > 1.0
        {
            errors.push(format!("node {} opacity must be between 0 and 1", node.id));
        }

        match node.status {
            CompositorNodeStatus::Hidden | CompositorNodeStatus::Ready => {}
            CompositorNodeStatus::Placeholder => warnings.push(format!(
                "{} is using a placeholder: {}",
                node.name, node.status_detail
            )),
            CompositorNodeStatus::PermissionRequired => warnings.push(format!(
                "{} requires permission: {}",
                node.name, node.status_detail
            )),
            CompositorNodeStatus::Unavailable => warnings.push(format!(
                "{} is unavailable: {}",
                node.name, node.status_detail
            )),
        }
    }

    CompositorValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

pub fn build_compositor_render_plan(
    graph: &CompositorGraph,
    targets: Vec<CompositorRenderTarget>,
) -> CompositorRenderPlan {
    CompositorRenderPlan {
        version: 1,
        renderer: CompositorRendererKind::Contract,
        graph: graph.clone(),
        targets,
    }
}

pub fn validate_compositor_render_plan(plan: &CompositorRenderPlan) -> CompositorValidation {
    let mut validation = validate_compositor_graph(&plan.graph);
    let mut target_ids = HashSet::new();
    let enabled_targets = plan.targets.iter().filter(|target| target.enabled).count();

    if plan.version == 0 {
        validation
            .errors
            .push("compositor render plan version must be greater than zero".to_string());
    }
    if plan.targets.is_empty() {
        validation
            .errors
            .push("compositor render plan must contain at least one target".to_string());
    }
    if enabled_targets == 0 {
        validation
            .errors
            .push("compositor render plan must contain at least one enabled target".to_string());
    }
    if !plan
        .targets
        .iter()
        .any(|target| target.enabled && target.kind == CompositorRenderTargetKind::Program)
    {
        validation
            .warnings
            .push("compositor render plan has no enabled program target".to_string());
    }

    for target in &plan.targets {
        if !target_ids.insert(target.id.as_str()) {
            validation.errors.push(format!(
                "duplicate compositor render target id \"{}\"",
                target.id
            ));
        }
        if target.id.trim().is_empty() {
            validation
                .errors
                .push("compositor render target id is required".to_string());
        }
        if target.name.trim().is_empty() {
            validation.errors.push(format!(
                "compositor render target \"{}\" name is required",
                target.id
            ));
        }
        if target.width == 0 || target.height == 0 {
            validation.errors.push(format!(
                "compositor render target \"{}\" dimensions must be greater than zero",
                target.id
            ));
        }
        if target.framerate == 0 {
            validation.errors.push(format!(
                "compositor render target \"{}\" framerate must be greater than zero",
                target.id
            ));
        }
    }

    validation.ready = validation.errors.is_empty();
    validation
}

pub fn compositor_render_target(
    id: impl Into<String>,
    name: impl Into<String>,
    kind: CompositorRenderTargetKind,
    width: u32,
    height: u32,
    framerate: u32,
) -> CompositorRenderTarget {
    CompositorRenderTarget {
        id: id.into(),
        name: name.into(),
        kind,
        width,
        height,
        framerate,
        frame_format: CompositorFrameFormat::Bgra8,
        scale_mode: CompositorScaleMode::Fit,
        enabled: true,
    }
}

pub fn evaluate_compositor_frame(
    plan: &CompositorRenderPlan,
    frame_index: u64,
) -> CompositorRenderedFrame {
    let validation = validate_compositor_render_plan(plan);
    let framerate = plan
        .targets
        .iter()
        .find(|target| target.enabled)
        .map(|target| target.framerate)
        .unwrap_or(60);
    let duration_nanos = 1_000_000_000_u64 / u64::from(framerate.max(1));
    let clock = CompositorFrameClock {
        frame_index,
        framerate,
        pts_nanos: frame_index.saturating_mul(duration_nanos),
        duration_nanos,
    };

    let targets = plan
        .targets
        .iter()
        .filter(|target| target.enabled)
        .map(|target| CompositorRenderedTarget {
            target_id: target.id.clone(),
            target_kind: target.kind.clone(),
            width: target.width,
            height: target.height,
            frame_format: target.frame_format.clone(),
            nodes: plan
                .graph
                .nodes
                .iter()
                .filter(|node| node.visible)
                .map(|node| evaluate_node_for_target(node, &plan.graph.output, target))
                .collect(),
        })
        .collect();

    CompositorRenderedFrame {
        renderer: plan.renderer.clone(),
        scene_id: plan.graph.scene_id.clone(),
        scene_name: plan.graph.scene_name.clone(),
        clock,
        targets,
        validation,
    }
}

fn evaluate_node_for_target(
    node: &CompositorNode,
    output: &CompositorOutput,
    target: &CompositorRenderTarget,
) -> CompositorEvaluatedNode {
    let (scale_x, scale_y, offset_x, offset_y) = target_mapping(output, target);
    CompositorEvaluatedNode {
        node_id: node.id.clone(),
        source_id: node.source_id.clone(),
        name: node.name.clone(),
        role: node.role.clone(),
        status: node.status.clone(),
        rect: CompositorRect {
            x: offset_x + node.transform.position.x * scale_x,
            y: offset_y + node.transform.position.y * scale_y,
            width: node.transform.size.width * scale_x,
            height: node.transform.size.height * scale_y,
        },
        crop: SceneCrop {
            top: node.transform.crop.top * scale_y,
            right: node.transform.crop.right * scale_x,
            bottom: node.transform.crop.bottom * scale_y,
            left: node.transform.crop.left * scale_x,
        },
        rotation_degrees: node.transform.rotation_degrees,
        opacity: node.transform.opacity,
        z_index: node.z_index,
    }
}

fn target_mapping(
    output: &CompositorOutput,
    target: &CompositorRenderTarget,
) -> (f64, f64, f64, f64) {
    let source_width = f64::from(output.width.max(1));
    let source_height = f64::from(output.height.max(1));
    let target_width = f64::from(target.width.max(1));
    let target_height = f64::from(target.height.max(1));
    match target.scale_mode {
        CompositorScaleMode::Stretch => (
            target_width / source_width,
            target_height / source_height,
            0.0,
            0.0,
        ),
        CompositorScaleMode::Fit => {
            let scale = (target_width / source_width).min(target_height / source_height);
            (
                scale,
                scale,
                (target_width - source_width * scale) / 2.0,
                (target_height - source_height * scale) / 2.0,
            )
        }
        CompositorScaleMode::Fill => {
            let scale = (target_width / source_width).max(target_height / source_height);
            (
                scale,
                scale,
                (target_width - source_width * scale) / 2.0,
                (target_height - source_height * scale) / 2.0,
            )
        }
        CompositorScaleMode::OriginalSize => (
            1.0,
            1.0,
            (target_width - source_width) / 2.0,
            (target_height - source_height) / 2.0,
        ),
    }
}

fn build_compositor_node(source: &SceneSource) -> CompositorNode {
    let (status, status_detail) = node_status(source);
    CompositorNode {
        id: format!("node-{}", source.id),
        source_id: source.id.clone(),
        name: source.name.clone(),
        source_kind: source.kind.clone(),
        role: node_role(&source.kind),
        transform: CompositorTransform {
            position: source.position.clone(),
            size: source.size.clone(),
            crop: source.crop.clone(),
            rotation_degrees: source.rotation_degrees,
            opacity: source.opacity,
        },
        visible: source.visible,
        locked: source.locked,
        z_index: source.z_index,
        blend_mode: CompositorBlendMode::Normal,
        scale_mode: CompositorScaleMode::Stretch,
        status,
        status_detail,
        config: source.config.clone(),
    }
}

fn node_role(kind: &SceneSourceKind) -> CompositorNodeRole {
    match kind {
        SceneSourceKind::Display | SceneSourceKind::Window | SceneSourceKind::Camera => {
            CompositorNodeRole::Video
        }
        SceneSourceKind::AudioMeter => CompositorNodeRole::Audio,
        SceneSourceKind::ImageMedia | SceneSourceKind::BrowserOverlay => {
            CompositorNodeRole::Overlay
        }
        SceneSourceKind::Text => CompositorNodeRole::Text,
        SceneSourceKind::Group => CompositorNodeRole::Group,
    }
}

fn node_status(source: &SceneSource) -> (CompositorNodeStatus, String) {
    if !source.visible {
        return (
            CompositorNodeStatus::Hidden,
            "Source is hidden in the active scene.".to_string(),
        );
    }

    if let Some((status, detail)) = explicit_availability_status(source) {
        return (status, detail);
    }

    match source.kind {
        SceneSourceKind::Display => capture_status(source, "display_id", "display"),
        SceneSourceKind::Window => capture_status(source, "window_id", "window"),
        SceneSourceKind::Camera => capture_status(source, "device_id", "camera"),
        SceneSourceKind::AudioMeter => capture_status(source, "device_id", "audio device"),
        SceneSourceKind::ImageMedia => config_string(source, "asset_uri")
            .map(|_| {
                (
                    CompositorNodeStatus::Ready,
                    "Media asset configured.".to_string(),
                )
            })
            .unwrap_or_else(|| {
                (
                    CompositorNodeStatus::Placeholder,
                    "No media asset has been selected.".to_string(),
                )
            }),
        SceneSourceKind::BrowserOverlay => config_string(source, "url")
            .map(|_| {
                (
                    CompositorNodeStatus::Ready,
                    "Browser overlay URL configured.".to_string(),
                )
            })
            .unwrap_or_else(|| {
                (
                    CompositorNodeStatus::Placeholder,
                    "No browser overlay URL has been configured.".to_string(),
                )
            }),
        SceneSourceKind::Text => config_string(source, "text")
            .map(|_| {
                (
                    CompositorNodeStatus::Ready,
                    "Text content configured.".to_string(),
                )
            })
            .unwrap_or_else(|| {
                (
                    CompositorNodeStatus::Placeholder,
                    "Text source is empty.".to_string(),
                )
            }),
        SceneSourceKind::Group => source
            .config
            .get("child_source_ids")
            .and_then(serde_json::Value::as_array)
            .filter(|children| !children.is_empty())
            .map(|children| {
                (
                    CompositorNodeStatus::Ready,
                    format!("{} child source(s) grouped.", children.len()),
                )
            })
            .unwrap_or_else(|| {
                (
                    CompositorNodeStatus::Placeholder,
                    "Group has no child sources.".to_string(),
                )
            }),
    }
}

fn explicit_availability_status(source: &SceneSource) -> Option<(CompositorNodeStatus, String)> {
    let availability = source.config.get("availability")?;
    let state = availability
        .get("state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    let detail = availability
        .get("detail")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Availability has not been checked.")
        .to_string();
    let status = match state {
        "available" => CompositorNodeStatus::Ready,
        "permission_required" => CompositorNodeStatus::PermissionRequired,
        "unavailable" => CompositorNodeStatus::Unavailable,
        _ => CompositorNodeStatus::Placeholder,
    };
    Some((status, detail))
}

fn capture_status(source: &SceneSource, key: &str, label: &str) -> (CompositorNodeStatus, String) {
    config_string(source, key)
        .map(|_| {
            (
                CompositorNodeStatus::Ready,
                format!("{label} capture target configured."),
            )
        })
        .unwrap_or_else(|| {
            (
                CompositorNodeStatus::Placeholder,
                format!("No {label} capture target has been assigned."),
            )
        })
}

fn config_string(source: &SceneSource, key: &str) -> Option<String> {
    source
        .config
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn validate_finite(value: f64, label: &str, errors: &mut Vec<String>) {
    if !value.is_finite() {
        errors.push(format!("{label} must be a finite number"));
    }
}

fn validate_positive(value: f64, label: &str, errors: &mut Vec<String>) {
    if !value.is_finite() || value <= 0.0 {
        errors.push(format!("{label} must be greater than zero"));
    }
}

fn validate_non_negative(value: f64, label: &str, errors: &mut Vec<String>) {
    if !value.is_finite() || value < 0.0 {
        errors.push(format!("{label} must be zero or greater"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compositor_graph_preserves_scene_order_and_status() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let graph = build_compositor_graph(scene);

        assert_eq!(graph.scene_id, "scene-main");
        assert_eq!(graph.output.width, 1920);
        assert_eq!(graph.nodes.len(), scene.sources.len());
        assert_eq!(graph.nodes[0].source_id, "source-main-display");
        assert_eq!(
            graph.nodes[0].status,
            CompositorNodeStatus::PermissionRequired
        );

        let validation = validate_compositor_graph(&graph);
        assert!(validation.ready, "{:?}", validation.errors);
        assert!(!validation.warnings.is_empty());
    }

    #[test]
    fn compositor_validation_rejects_invalid_geometry() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let mut graph = build_compositor_graph(scene);
        graph.nodes[0].transform.size.width = 0.0;
        graph.nodes[0].transform.opacity = 3.0;

        let validation = validate_compositor_graph(&graph);

        assert!(!validation.ready);
        assert!(validation
            .errors
            .iter()
            .any(|error| error.contains("size.width")));
        assert!(validation
            .errors
            .iter()
            .any(|error| error.contains("opacity")));
    }

    #[test]
    fn compositor_render_plan_validates_program_and_output_targets() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let graph = build_compositor_graph(scene);
        let plan = build_compositor_render_plan(
            &graph,
            vec![
                compositor_render_target(
                    "preview",
                    "Preview",
                    CompositorRenderTargetKind::Preview,
                    graph.output.width,
                    graph.output.height,
                    60,
                ),
                compositor_render_target(
                    "program",
                    "Program",
                    CompositorRenderTargetKind::Program,
                    graph.output.width,
                    graph.output.height,
                    60,
                ),
            ],
        );

        let validation = validate_compositor_render_plan(&plan);

        assert!(validation.ready, "{:?}", validation.errors);
        assert_eq!(plan.targets.len(), 2);
        assert_eq!(plan.targets[1].kind, CompositorRenderTargetKind::Program);
    }

    #[test]
    fn compositor_frame_evaluation_maps_nodes_to_targets() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let graph = build_compositor_graph(scene);
        let plan = build_compositor_render_plan(
            &graph,
            vec![compositor_render_target(
                "program-720",
                "Program 720p",
                CompositorRenderTargetKind::Program,
                1280,
                720,
                30,
            )],
        );

        let frame = evaluate_compositor_frame(&plan, 2);

        assert!(frame.validation.ready, "{:?}", frame.validation.errors);
        assert_eq!(frame.clock.framerate, 30);
        assert_eq!(frame.clock.pts_nanos, 66_666_666);
        assert_eq!(frame.targets.len(), 1);
        assert_eq!(frame.targets[0].nodes.len(), graph.nodes.len());
        assert_eq!(frame.targets[0].nodes[0].rect.width, 1280.0);
        assert_eq!(frame.targets[0].nodes[0].rect.height, 720.0);
    }
}
