use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    Scene, SceneCrop, ScenePoint, SceneSize, SceneSource, SceneSourceBoundsMode, SceneSourceFilter,
    SceneSourceKind,
};

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
    Center,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_source_id: Option<String>,
    #[serde(default)]
    pub group_depth: u32,
    pub transform: CompositorTransform,
    pub visible: bool,
    pub locked: bool,
    pub z_index: i32,
    pub blend_mode: CompositorBlendMode,
    pub scale_mode: CompositorScaleMode,
    pub status: CompositorNodeStatus,
    pub status_detail: String,
    pub filters: Vec<SceneSourceFilter>,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoftwareCompositorFrame {
    pub target_id: String,
    pub target_kind: CompositorRenderTargetKind,
    pub width: u32,
    pub height: u32,
    pub frame_format: CompositorFrameFormat,
    pub bytes_per_row: usize,
    pub checksum: u64,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoftwareCompositorRenderResult {
    pub frame: CompositorRenderedFrame,
    pub pixel_frames: Vec<SoftwareCompositorFrame>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CompositorValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn build_compositor_graph(scene: &Scene) -> CompositorGraph {
    let mut sources = scene.sources.iter().collect::<Vec<_>>();
    let parent_by_source_id = group_parent_map(&scene.sources);
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
        nodes: sources
            .into_iter()
            .map(|source| {
                let parent_source_id = parent_by_source_id.get(&source.id).cloned();
                let group_depth = group_depth(&source.id, &parent_by_source_id);
                build_compositor_node(source, parent_source_id, group_depth)
            })
            .collect(),
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
        if let Some(parent_source_id) = &node.parent_source_id {
            if parent_source_id == &node.source_id {
                errors.push(format!(
                    "compositor node \"{}\" cannot parent itself",
                    node.id
                ));
            }
            if !graph
                .nodes
                .iter()
                .any(|candidate| candidate.source_id == *parent_source_id)
            {
                errors.push(format!(
                    "compositor node \"{}\" references missing parent source \"{}\"",
                    node.id, parent_source_id
                ));
            }
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
                .map(|node| evaluate_node_for_target(node, &plan.graph, target))
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

pub fn render_software_compositor_frame(
    plan: &CompositorRenderPlan,
    frame_index: u64,
) -> SoftwareCompositorRenderResult {
    let mut frame = evaluate_compositor_frame(plan, frame_index);
    frame.renderer = CompositorRendererKind::Software;
    let background = parse_background_color(&plan.graph.output.background_color);
    let pixel_frames = frame
        .targets
        .iter()
        .map(|target| render_software_target(target, background))
        .collect();

    SoftwareCompositorRenderResult {
        frame,
        pixel_frames,
    }
}

fn evaluate_node_for_target(
    node: &CompositorNode,
    graph: &CompositorGraph,
    target: &CompositorRenderTarget,
) -> CompositorEvaluatedNode {
    let transform = effective_node_transform(node, graph);
    let source_rect = node_bounds_rect(&transform, node);
    let (scale_x, scale_y, offset_x, offset_y) = target_mapping(&graph.output, target);
    CompositorEvaluatedNode {
        node_id: node.id.clone(),
        source_id: node.source_id.clone(),
        name: node.name.clone(),
        role: node.role.clone(),
        status: node.status.clone(),
        rect: CompositorRect {
            x: offset_x + source_rect.x * scale_x,
            y: offset_y + source_rect.y * scale_y,
            width: source_rect.width * scale_x,
            height: source_rect.height * scale_y,
        },
        crop: SceneCrop {
            top: transform.crop.top * scale_y,
            right: transform.crop.right * scale_x,
            bottom: transform.crop.bottom * scale_y,
            left: transform.crop.left * scale_x,
        },
        rotation_degrees: transform.rotation_degrees,
        opacity: transform.opacity,
        z_index: node.z_index,
    }
}

fn node_bounds_rect(transform: &CompositorTransform, node: &CompositorNode) -> CompositorRect {
    let bounds = CompositorRect {
        x: transform.position.x,
        y: transform.position.y,
        width: transform.size.width,
        height: transform.size.height,
    };
    let native_size = node_native_size(node, transform);

    match node.scale_mode {
        CompositorScaleMode::Stretch => bounds,
        CompositorScaleMode::Fit => {
            let scale = (bounds.width / native_size.width).min(bounds.height / native_size.height);
            centered_rect(
                &bounds,
                native_size.width * scale,
                native_size.height * scale,
            )
        }
        CompositorScaleMode::Fill => {
            let scale = (bounds.width / native_size.width).max(bounds.height / native_size.height);
            centered_rect(
                &bounds,
                native_size.width * scale,
                native_size.height * scale,
            )
        }
        CompositorScaleMode::Center => {
            centered_rect(&bounds, native_size.width, native_size.height)
        }
        CompositorScaleMode::OriginalSize => CompositorRect {
            x: bounds.x,
            y: bounds.y,
            width: native_size.width,
            height: native_size.height,
        },
    }
}

fn centered_rect(bounds: &CompositorRect, width: f64, height: f64) -> CompositorRect {
    CompositorRect {
        x: bounds.x + (bounds.width - width) / 2.0,
        y: bounds.y + (bounds.height - height) / 2.0,
        width,
        height,
    }
}

fn node_native_size(node: &CompositorNode, transform: &CompositorTransform) -> SceneSize {
    let size = match node.source_kind {
        SceneSourceKind::Display | SceneSourceKind::Window | SceneSourceKind::Camera => {
            config_size(&node.config, "resolution")
        }
        SceneSourceKind::BrowserOverlay => config_size(&node.config, "viewport"),
        SceneSourceKind::ImageMedia
        | SceneSourceKind::AudioMeter
        | SceneSourceKind::Text
        | SceneSourceKind::Group => None,
    }
    .unwrap_or_else(|| transform.size.clone());

    SceneSize {
        width: size.width.max(1.0),
        height: size.height.max(1.0),
    }
}

fn config_size(config: &serde_json::Value, key: &str) -> Option<SceneSize> {
    let value = config.get(key)?;
    let width = value.get("width")?.as_f64()?;
    let height = value.get("height")?.as_f64()?;
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        Some(SceneSize { width, height })
    } else {
        None
    }
}

fn effective_node_transform(node: &CompositorNode, graph: &CompositorGraph) -> CompositorTransform {
    let mut transform = node.transform.clone();
    let mut parent_source_id = node.parent_source_id.as_deref();
    let mut visited = HashSet::new();

    while let Some(source_id) = parent_source_id {
        if !visited.insert(source_id.to_string()) {
            break;
        }
        let Some(parent) = graph
            .nodes
            .iter()
            .find(|candidate| candidate.source_id == source_id)
        else {
            break;
        };
        transform.position.x += parent.transform.position.x;
        transform.position.y += parent.transform.position.y;
        transform.rotation_degrees += parent.transform.rotation_degrees;
        transform.opacity = (transform.opacity * parent.transform.opacity).clamp(0.0, 1.0);
        parent_source_id = parent.parent_source_id.as_deref();
    }

    transform
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
        CompositorScaleMode::Center => (
            1.0,
            1.0,
            (target_width - source_width) / 2.0,
            (target_height - source_height) / 2.0,
        ),
        CompositorScaleMode::OriginalSize => (
            1.0,
            1.0,
            (target_width - source_width) / 2.0,
            (target_height - source_height) / 2.0,
        ),
    }
}

fn render_software_target(
    target: &CompositorRenderedTarget,
    background: [u8; 4],
) -> SoftwareCompositorFrame {
    let width = target.width.max(1) as usize;
    let height = target.height.max(1) as usize;
    let bytes_per_row = width * 4;
    let mut pixels = vec![0; bytes_per_row * height];
    fill_background(&mut pixels, background);

    for node in &target.nodes {
        draw_node(&mut pixels, width, height, node);
    }

    let checksum = checksum_pixels(&pixels);
    SoftwareCompositorFrame {
        target_id: target.target_id.clone(),
        target_kind: target.target_kind.clone(),
        width: width as u32,
        height: height as u32,
        frame_format: CompositorFrameFormat::Rgba8,
        bytes_per_row,
        checksum,
        pixels,
    }
}

fn fill_background(pixels: &mut [u8], background: [u8; 4]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&background);
    }
}

fn draw_node(
    pixels: &mut [u8],
    target_width: usize,
    target_height: usize,
    node: &CompositorEvaluatedNode,
) {
    let Some(rect) = drawable_rect(node) else {
        return;
    };
    let left = rect.x.floor().max(0.0) as usize;
    let top = rect.y.floor().max(0.0) as usize;
    let right = (rect.x + rect.width).ceil().min(target_width as f64) as usize;
    let bottom = (rect.y + rect.height).ceil().min(target_height as f64) as usize;

    if left >= right || top >= bottom {
        return;
    }

    let alpha = (node.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    if alpha == 0 {
        return;
    }
    let color = node_color(node, alpha);

    for y in top..bottom {
        let row_offset = y * target_width * 4;
        for x in left..right {
            blend_pixel(
                &mut pixels[row_offset + x * 4..row_offset + x * 4 + 4],
                color,
            );
        }
    }
}

fn drawable_rect(node: &CompositorEvaluatedNode) -> Option<CompositorRect> {
    if !node.opacity.is_finite()
        || !node.rect.x.is_finite()
        || !node.rect.y.is_finite()
        || !node.rect.width.is_finite()
        || !node.rect.height.is_finite()
        || node.rect.width <= 0.0
        || node.rect.height <= 0.0
    {
        return None;
    }

    let crop_left = node.crop.left.max(0.0).min(node.rect.width);
    let crop_right = node.crop.right.max(0.0).min(node.rect.width - crop_left);
    let crop_top = node.crop.top.max(0.0).min(node.rect.height);
    let crop_bottom = node.crop.bottom.max(0.0).min(node.rect.height - crop_top);
    let width = node.rect.width - crop_left - crop_right;
    let height = node.rect.height - crop_top - crop_bottom;

    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    Some(CompositorRect {
        x: node.rect.x + crop_left,
        y: node.rect.y + crop_top,
        width,
        height,
    })
}

fn blend_pixel(pixel: &mut [u8], color: [u8; 4]) {
    let alpha = u16::from(color[3]);
    let inverse_alpha = 255 - alpha;
    pixel[0] = blend_channel(color[0], pixel[0], alpha, inverse_alpha);
    pixel[1] = blend_channel(color[1], pixel[1], alpha, inverse_alpha);
    pixel[2] = blend_channel(color[2], pixel[2], alpha, inverse_alpha);
    pixel[3] = (alpha + u16::from(pixel[3]) * inverse_alpha / 255).min(255) as u8;
}

fn blend_channel(source: u8, destination: u8, alpha: u16, inverse_alpha: u16) -> u8 {
    ((u16::from(source) * alpha + u16::from(destination) * inverse_alpha + 127) / 255) as u8
}

fn node_color(node: &CompositorEvaluatedNode, alpha: u8) -> [u8; 4] {
    let [red, green, blue] = match node.status {
        CompositorNodeStatus::Unavailable => [133, 55, 71],
        CompositorNodeStatus::PermissionRequired => [202, 126, 46],
        CompositorNodeStatus::Placeholder => [94, 112, 139],
        CompositorNodeStatus::Hidden | CompositorNodeStatus::Ready => match node.role {
            CompositorNodeRole::Video => [43, 104, 217],
            CompositorNodeRole::Audio => [36, 170, 142],
            CompositorNodeRole::Overlay => [211, 137, 55],
            CompositorNodeRole::Text => [178, 81, 209],
            CompositorNodeRole::Group => [123, 139, 163],
        },
    };
    [red, green, blue, alpha]
}

fn parse_background_color(value: &str) -> [u8; 4] {
    let trimmed = value.trim();
    let Some(hex) = trimmed.strip_prefix('#') else {
        return [5, 7, 17, 255];
    };
    if hex.len() != 6 {
        return [5, 7, 17, 255];
    }

    let Some(red) = parse_hex_channel(&hex[0..2]) else {
        return [5, 7, 17, 255];
    };
    let Some(green) = parse_hex_channel(&hex[2..4]) else {
        return [5, 7, 17, 255];
    };
    let Some(blue) = parse_hex_channel(&hex[4..6]) else {
        return [5, 7, 17, 255];
    };

    [red, green, blue, 255]
}

fn parse_hex_channel(value: &str) -> Option<u8> {
    u8::from_str_radix(value, 16).ok()
}

fn checksum_pixels(pixels: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for byte in pixels {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn build_compositor_node(
    source: &SceneSource,
    parent_source_id: Option<String>,
    group_depth: u32,
) -> CompositorNode {
    let (status, status_detail) = node_status(source);
    CompositorNode {
        id: format!("node-{}", source.id),
        source_id: source.id.clone(),
        name: source.name.clone(),
        source_kind: source.kind.clone(),
        role: node_role(&source.kind),
        parent_source_id,
        group_depth,
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
        scale_mode: node_scale_mode(&source.bounds_mode),
        status,
        status_detail,
        filters: source.filters.clone(),
        config: source.config.clone(),
    }
}

fn group_parent_map(sources: &[SceneSource]) -> HashMap<String, String> {
    let mut parent_by_source_id = HashMap::new();
    for source in sources {
        if source.kind != SceneSourceKind::Group {
            continue;
        }
        for child_source_id in group_child_ids(source) {
            parent_by_source_id
                .entry(child_source_id)
                .or_insert_with(|| source.id.clone());
        }
    }
    parent_by_source_id
}

fn group_depth(source_id: &str, parent_by_source_id: &HashMap<String, String>) -> u32 {
    let mut depth = 0_u32;
    let mut visited = HashSet::new();
    let mut cursor = source_id;

    while let Some(parent_source_id) = parent_by_source_id.get(cursor) {
        if !visited.insert(parent_source_id.as_str()) {
            break;
        }
        depth = depth.saturating_add(1);
        cursor = parent_source_id;
    }

    depth
}

fn group_child_ids(source: &SceneSource) -> Vec<String> {
    source
        .config
        .get("child_source_ids")
        .and_then(serde_json::Value::as_array)
        .map(|children| {
            children
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|child_source_id| !child_source_id.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
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

fn node_scale_mode(bounds_mode: &SceneSourceBoundsMode) -> CompositorScaleMode {
    match bounds_mode {
        SceneSourceBoundsMode::Stretch => CompositorScaleMode::Stretch,
        SceneSourceBoundsMode::Fit => CompositorScaleMode::Fit,
        SceneSourceBoundsMode::Fill => CompositorScaleMode::Fill,
        SceneSourceBoundsMode::Center => CompositorScaleMode::Center,
        SceneSourceBoundsMode::OriginalSize => CompositorScaleMode::OriginalSize,
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

    #[test]
    fn compositor_frame_evaluation_applies_group_parent_transforms() {
        let mut collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.scenes.first_mut().unwrap();
        let camera = scene
            .sources
            .iter_mut()
            .find(|source| source.id == "source-camera-placeholder")
            .unwrap();
        camera.position = ScenePoint { x: 20.0, y: 30.0 };
        camera.opacity = 0.5;
        camera.rotation_degrees = 5.0;
        scene.sources.push(SceneSource {
            id: "source-group".to_string(),
            name: "Camera Group".to_string(),
            kind: SceneSourceKind::Group,
            position: ScenePoint { x: 100.0, y: 50.0 },
            size: SceneSize {
                width: 640.0,
                height: 360.0,
            },
            crop: SceneCrop {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
            rotation_degrees: 10.0,
            opacity: 0.8,
            visible: true,
            locked: false,
            z_index: 5,
            bounds_mode: SceneSourceBoundsMode::Stretch,
            filters: Vec::new(),
            config: serde_json::json!({
                "child_source_ids": ["source-camera-placeholder"]
            }),
        });

        let graph = build_compositor_graph(scene);
        let camera_node = graph
            .nodes
            .iter()
            .find(|node| node.source_id == "source-camera-placeholder")
            .unwrap();
        assert_eq!(
            camera_node.parent_source_id.as_deref(),
            Some("source-group")
        );
        assert_eq!(camera_node.group_depth, 1);

        let plan = build_compositor_render_plan(
            &graph,
            vec![compositor_render_target(
                "program",
                "Program",
                CompositorRenderTargetKind::Program,
                1920,
                1080,
                60,
            )],
        );
        let frame = evaluate_compositor_frame(&plan, 0);
        let camera_frame_node = frame.targets[0]
            .nodes
            .iter()
            .find(|node| node.source_id == "source-camera-placeholder")
            .unwrap();

        assert_eq!(camera_frame_node.rect.x, 120.0);
        assert_eq!(camera_frame_node.rect.y, 80.0);
        assert_eq!(camera_frame_node.rotation_degrees, 15.0);
        assert!((camera_frame_node.opacity - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn compositor_frame_evaluation_applies_source_bounds_modes() {
        let mut collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.scenes.first_mut().unwrap();
        let camera = scene
            .sources
            .iter_mut()
            .find(|source| source.id == "source-camera-placeholder")
            .unwrap();
        camera.position = ScenePoint { x: 0.0, y: 0.0 };
        camera.size = SceneSize {
            width: 300.0,
            height: 300.0,
        };
        camera.bounds_mode = SceneSourceBoundsMode::Fit;

        let graph = build_compositor_graph(scene);
        let camera_node = graph
            .nodes
            .iter()
            .find(|node| node.source_id == "source-camera-placeholder")
            .unwrap();
        assert_eq!(camera_node.scale_mode, CompositorScaleMode::Fit);

        let plan = build_compositor_render_plan(
            &graph,
            vec![compositor_render_target(
                "program",
                "Program",
                CompositorRenderTargetKind::Program,
                1920,
                1080,
                60,
            )],
        );
        let frame = evaluate_compositor_frame(&plan, 0);
        let camera_frame_node = frame.targets[0]
            .nodes
            .iter()
            .find(|node| node.source_id == "source-camera-placeholder")
            .unwrap();

        assert_eq!(camera_frame_node.rect.x, 0.0);
        assert!((camera_frame_node.rect.y - 65.625).abs() < f64::EPSILON);
        assert_eq!(camera_frame_node.rect.width, 300.0);
        assert!((camera_frame_node.rect.height - 168.75).abs() < f64::EPSILON);
    }

    #[test]
    fn software_compositor_renders_rgba_target_buffers() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let graph = build_compositor_graph(scene);
        let mut disabled_target = compositor_render_target(
            "disabled-preview",
            "Disabled Preview",
            CompositorRenderTargetKind::Preview,
            320,
            180,
            30,
        );
        disabled_target.enabled = false;
        let plan = build_compositor_render_plan(
            &graph,
            vec![
                compositor_render_target(
                    "program-720",
                    "Program 720p",
                    CompositorRenderTargetKind::Program,
                    1280,
                    720,
                    30,
                ),
                disabled_target,
            ],
        );

        let result = render_software_compositor_frame(&plan, 0);

        assert!(
            result.frame.validation.ready,
            "{:?}",
            result.frame.validation.errors
        );
        assert_eq!(result.frame.renderer, CompositorRendererKind::Software);
        assert_eq!(result.pixel_frames.len(), 1);
        let target = &result.pixel_frames[0];
        assert_eq!(target.frame_format, CompositorFrameFormat::Rgba8);
        assert_eq!(target.bytes_per_row, 1280 * 4);
        assert_eq!(target.pixels.len(), 1280 * 720 * 4);
        assert_ne!(target.checksum, 0);
        assert_ne!(&target.pixels[0..4], &[5, 7, 17, 255]);
    }
}
