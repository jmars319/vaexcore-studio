use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use ab_glyph::{point, Font, FontArc, Glyph, PxScale, ScaleFont};
use image::ImageReader;
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
    pub status_detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<SoftwareCompositorAssetMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<SoftwareCompositorTextMetadata>,
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
pub struct SoftwareCompositorInputFrame {
    pub source_id: String,
    pub source_kind: SceneSourceKind,
    pub width: u32,
    pub height: u32,
    pub frame_format: CompositorFrameFormat,
    pub status: CompositorNodeStatus,
    pub status_detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<SoftwareCompositorAssetMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<SoftwareCompositorTextMetadata>,
    pub checksum: u64,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareCompositorAssetStatus {
    Decoded,
    MissingFile,
    UnsupportedExtension,
    DecodeFailed,
    VideoPlaceholder,
    NoAsset,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoftwareCompositorAssetMetadata {
    pub uri: String,
    pub status: SoftwareCompositorAssetStatus,
    pub status_detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_unix_ms: Option<u64>,
    #[serde(default)]
    pub cache_hit: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareCompositorTextStatus {
    Rendered,
    FontFallback,
    Empty,
    InvalidColor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoftwareCompositorTextMetadata {
    pub status: SoftwareCompositorTextStatus,
    pub status_detail: String,
    pub requested_font_family: String,
    pub used_font_family: String,
    pub font_size: f64,
    pub color: String,
    pub align: String,
    pub text_length: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered_bounds: Option<CompositorRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoftwareCompositorRenderResult {
    pub frame: CompositorRenderedFrame,
    pub input_frames: Vec<SoftwareCompositorInputFrame>,
    pub pixel_frames: Vec<SoftwareCompositorFrame>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ImageAssetCacheKey {
    path: String,
    modified_unix_ms: u64,
}

#[derive(Clone, Debug)]
struct CachedDecodedImage {
    width: u32,
    height: u32,
    format: String,
    checksum: u64,
    pixels: Vec<u8>,
}

#[derive(Clone, Debug)]
struct DecodedImageAsset {
    modified_unix_ms: u64,
    cache_hit: bool,
    image: CachedDecodedImage,
}

#[derive(Clone, Debug)]
struct TextRenderRequest {
    text: String,
    requested_font_family: String,
    font_size: f64,
    color: [u8; 4],
    color_string: String,
    invalid_color: bool,
    align: TextAlign,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TextAlign {
    Left,
    Center,
    Right,
}

static IMAGE_ASSET_CACHE: OnceLock<Mutex<HashMap<ImageAssetCacheKey, CachedDecodedImage>>> =
    OnceLock::new();
static INTER_FONT: OnceLock<Result<FontArc, String>> = OnceLock::new();
const INTER_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/Inter.ttf");
const SOFTWARE_TEXT_FONT_FAMILY: &str = "Inter";

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
    let background = parse_background_color(&plan.graph.output.background_color);
    let input_frames = build_software_compositor_input_frames(&plan.graph);
    let input_frame_by_source = input_frames
        .iter()
        .map(|input| (input.source_id.as_str(), input))
        .collect::<HashMap<_, _>>();
    let mut frame = evaluate_software_compositor_frame(plan, frame_index, &input_frame_by_source);
    let pixel_frames = frame
        .targets
        .iter()
        .map(|target| render_software_target(target, background, &input_frame_by_source))
        .collect();
    apply_software_input_validation(&mut frame.validation, &input_frames);

    SoftwareCompositorRenderResult {
        frame,
        input_frames,
        pixel_frames,
    }
}

pub fn build_software_compositor_input_frames(
    graph: &CompositorGraph,
) -> Vec<SoftwareCompositorInputFrame> {
    graph
        .nodes
        .iter()
        .filter(|node| node.visible)
        .map(software_input_frame_for_node)
        .collect()
}

fn evaluate_software_compositor_frame(
    plan: &CompositorRenderPlan,
    frame_index: u64,
    input_frames: &HashMap<&str, &SoftwareCompositorInputFrame>,
) -> CompositorRenderedFrame {
    let mut frame = evaluate_compositor_frame(plan, frame_index);
    frame.renderer = CompositorRendererKind::Software;

    for rendered_target in &mut frame.targets {
        let Some(plan_target) = plan
            .targets
            .iter()
            .find(|target| target.id == rendered_target.target_id)
        else {
            continue;
        };
        for node in &mut rendered_target.nodes {
            let Some(input_frame) = input_frames.get(node.source_id.as_str()) else {
                continue;
            };
            if let Some(graph_node) = plan
                .graph
                .nodes
                .iter()
                .find(|candidate| candidate.source_id == node.source_id)
            {
                *node = evaluate_node_for_target_with_input(
                    graph_node,
                    &plan.graph,
                    plan_target,
                    Some(input_frame),
                );
            }
        }
    }

    frame
}

fn evaluate_node_for_target(
    node: &CompositorNode,
    graph: &CompositorGraph,
    target: &CompositorRenderTarget,
) -> CompositorEvaluatedNode {
    evaluate_node_for_target_with_input(node, graph, target, None)
}

fn evaluate_node_for_target_with_input(
    node: &CompositorNode,
    graph: &CompositorGraph,
    target: &CompositorRenderTarget,
    input_frame: Option<&SoftwareCompositorInputFrame>,
) -> CompositorEvaluatedNode {
    let transform = effective_node_transform(node, graph);
    let source_rect = node_bounds_rect(&transform, node, input_frame);
    let (scale_x, scale_y, offset_x, offset_y) = target_mapping(&graph.output, target);
    let status = input_frame
        .map(|frame| frame.status.clone())
        .unwrap_or_else(|| node.status.clone());
    let status_detail = input_frame
        .map(|frame| frame.status_detail.clone())
        .unwrap_or_else(|| node.status_detail.clone());
    let asset = input_frame.and_then(|frame| frame.asset.clone());
    let text = input_frame.and_then(|frame| frame.text.clone());
    CompositorEvaluatedNode {
        node_id: node.id.clone(),
        source_id: node.source_id.clone(),
        name: node.name.clone(),
        role: node.role.clone(),
        status,
        status_detail,
        asset,
        text,
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

fn node_bounds_rect(
    transform: &CompositorTransform,
    node: &CompositorNode,
    input_frame: Option<&SoftwareCompositorInputFrame>,
) -> CompositorRect {
    let bounds = CompositorRect {
        x: transform.position.x,
        y: transform.position.y,
        width: transform.size.width,
        height: transform.size.height,
    };
    let native_size = node_native_size(node, transform, input_frame);

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

fn node_native_size(
    node: &CompositorNode,
    transform: &CompositorTransform,
    input_frame: Option<&SoftwareCompositorInputFrame>,
) -> SceneSize {
    let size = match node.source_kind {
        SceneSourceKind::Display | SceneSourceKind::Window | SceneSourceKind::Camera => {
            config_size(&node.config, "resolution")
        }
        SceneSourceKind::ImageMedia => input_frame.and_then(|frame| {
            if frame.status == CompositorNodeStatus::Ready {
                Some(SceneSize {
                    width: f64::from(frame.width.max(1)),
                    height: f64::from(frame.height.max(1)),
                })
            } else {
                None
            }
        }),
        SceneSourceKind::BrowserOverlay => config_size(&node.config, "viewport"),
        SceneSourceKind::AudioMeter | SceneSourceKind::Text | SceneSourceKind::Group => None,
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

fn config_value_string(config: &serde_json::Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn config_value_number(config: &serde_json::Value, key: &str) -> Option<f64> {
    config
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .filter(|value| value.is_finite())
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
    input_frames: &HashMap<&str, &SoftwareCompositorInputFrame>,
) -> SoftwareCompositorFrame {
    let width = target.width.max(1) as usize;
    let height = target.height.max(1) as usize;
    let bytes_per_row = width * 4;
    let mut pixels = vec![0; bytes_per_row * height];
    fill_background(&mut pixels, background);

    for node in &target.nodes {
        if let Some(input_frame) = input_frames.get(node.source_id.as_str()) {
            draw_node(&mut pixels, width, height, node, input_frame);
        }
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
    input_frame: &SoftwareCompositorInputFrame,
) {
    let Some(rect) = drawable_rect(node) else {
        return;
    };

    if node.opacity <= 0.0 {
        return;
    }
    let bounds = rotated_bounds(&rect, node.rotation_degrees);
    let left = bounds.x.floor().max(0.0) as usize;
    let top = bounds.y.floor().max(0.0) as usize;
    let right = (bounds.x + bounds.width).ceil().min(target_width as f64) as usize;
    let bottom = (bounds.y + bounds.height).ceil().min(target_height as f64) as usize;

    for y in top..bottom {
        let row_offset = y * target_width * 4;
        for x in left..right {
            let Some((source_x, source_y)) =
                source_sample_for_target_pixel(x, y, &rect, node.rotation_degrees, input_frame)
            else {
                continue;
            };
            let mut color = input_frame_pixel(input_frame, source_x, source_y);
            color[3] = ((f64::from(color[3]) * node.opacity.clamp(0.0, 1.0)).round())
                .clamp(0.0, 255.0) as u8;
            if color[3] == 0 {
                continue;
            }
            blend_pixel(
                &mut pixels[row_offset + x * 4..row_offset + x * 4 + 4],
                color,
            );
        }
    }
}

fn software_input_frame_for_node(node: &CompositorNode) -> SoftwareCompositorInputFrame {
    if node.source_kind == SceneSourceKind::ImageMedia {
        return image_media_input_frame_for_node(node);
    }
    if node.source_kind == SceneSourceKind::Text {
        return text_input_frame_for_node(node);
    }

    placeholder_input_frame_for_node(
        node,
        node.status.clone(),
        node.status_detail.clone(),
        None,
        None,
    )
}

fn image_media_input_frame_for_node(node: &CompositorNode) -> SoftwareCompositorInputFrame {
    let asset_uri = config_value_string(&node.config, "asset_uri").unwrap_or_default();
    if asset_uri.trim().is_empty() {
        let metadata = asset_metadata(
            asset_uri,
            SoftwareCompositorAssetStatus::NoAsset,
            "No local image asset has been selected.".to_string(),
            None,
        );
        return placeholder_input_frame_for_node(
            node,
            CompositorNodeStatus::Placeholder,
            "No local image asset has been selected.".to_string(),
            Some(metadata),
            None,
        );
    }

    let media_type =
        config_value_string(&node.config, "media_type").unwrap_or_else(|| "image".into());
    if media_type != "image" {
        let metadata = asset_metadata(
            asset_uri.clone(),
            SoftwareCompositorAssetStatus::VideoPlaceholder,
            "Video media decode/playback is deferred for this source.".to_string(),
            None,
        );
        return placeholder_input_frame_for_node(
            node,
            CompositorNodeStatus::Placeholder,
            "Video media decode/playback is deferred for this source.".to_string(),
            Some(metadata),
            None,
        );
    }

    match decode_image_asset(&asset_uri) {
        Ok(decoded) => decoded_image_input_frame(node, asset_uri, decoded),
        Err(metadata) => {
            let metadata = *metadata;
            placeholder_input_frame_for_node(
                node,
                CompositorNodeStatus::Placeholder,
                metadata.status_detail.clone(),
                Some(metadata),
                None,
            )
        }
    }
}

fn decoded_image_input_frame(
    node: &CompositorNode,
    asset_uri: String,
    decoded: DecodedImageAsset,
) -> SoftwareCompositorInputFrame {
    let metadata = SoftwareCompositorAssetMetadata {
        uri: asset_uri,
        status: SoftwareCompositorAssetStatus::Decoded,
        status_detail: format!(
            "Decoded {} image {}x{}.",
            decoded.image.format, decoded.image.width, decoded.image.height
        ),
        format: Some(decoded.image.format.clone()),
        width: Some(decoded.image.width),
        height: Some(decoded.image.height),
        checksum: Some(decoded.image.checksum),
        modified_unix_ms: Some(decoded.modified_unix_ms),
        cache_hit: decoded.cache_hit,
    };

    SoftwareCompositorInputFrame {
        source_id: node.source_id.clone(),
        source_kind: node.source_kind.clone(),
        width: decoded.image.width,
        height: decoded.image.height,
        frame_format: CompositorFrameFormat::Rgba8,
        status: CompositorNodeStatus::Ready,
        status_detail: metadata.status_detail.clone(),
        asset: Some(metadata),
        text: None,
        checksum: decoded.image.checksum,
        pixels: decoded.image.pixels,
    }
}

fn text_input_frame_for_node(node: &CompositorNode) -> SoftwareCompositorInputFrame {
    let request = text_render_request(node);
    if request.text.is_empty() {
        let metadata = text_metadata(
            SoftwareCompositorTextStatus::Empty,
            "Text source is empty.".to_string(),
            &request,
            None,
            None,
        );
        return placeholder_input_frame_for_node(
            node,
            CompositorNodeStatus::Placeholder,
            metadata.status_detail.clone(),
            None,
            Some(metadata),
        );
    }

    match render_text_source(node, &request) {
        Ok(frame) => frame,
        Err(detail) => {
            let metadata = text_metadata(
                SoftwareCompositorTextStatus::Empty,
                detail,
                &request,
                None,
                None,
            );
            placeholder_input_frame_for_node(
                node,
                CompositorNodeStatus::Placeholder,
                metadata.status_detail.clone(),
                None,
                Some(metadata),
            )
        }
    }
}

fn render_text_source(
    node: &CompositorNode,
    request: &TextRenderRequest,
) -> Result<SoftwareCompositorInputFrame, String> {
    let size = input_frame_size(node);
    let width = size.width.max(1.0).round().min(3840.0) as u32;
    let height = size.height.max(1.0).round().min(2160.0) as u32;
    let mut pixels = vec![0; width as usize * height as usize * 4];
    let font = inter_font()?;
    let glyphs = layout_text_glyphs(&font, request, width, height);

    for glyph in glyphs {
        let Some(outlined) = font.outline_glyph(glyph) else {
            continue;
        };
        let bounds = outlined.px_bounds();
        outlined.draw(|glyph_x, glyph_y, coverage| {
            if coverage <= 0.0 {
                return;
            }
            let x = glyph_x as i32 + bounds.min.x.floor() as i32;
            let y = glyph_y as i32 + bounds.min.y.floor() as i32;
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                return;
            }
            let offset = (y as usize * width as usize + x as usize) * 4;
            let mut color = request.color;
            color[3] = ((coverage * f32::from(color[3])).round()).clamp(0.0, 255.0) as u8;
            blend_pixel(&mut pixels[offset..offset + 4], color);
        });
    }

    let checksum = checksum_pixels(&pixels);
    let rendered_bounds = alpha_bounds(&pixels, width, height);
    let status = if request.invalid_color {
        SoftwareCompositorTextStatus::InvalidColor
    } else if uses_font_fallback(&request.requested_font_family) {
        SoftwareCompositorTextStatus::FontFallback
    } else {
        SoftwareCompositorTextStatus::Rendered
    };
    let metadata = text_metadata(
        status,
        text_status_detail(request),
        request,
        rendered_bounds,
        Some(checksum),
    );
    let status_detail = metadata.status_detail.clone();

    Ok(SoftwareCompositorInputFrame {
        source_id: node.source_id.clone(),
        source_kind: node.source_kind.clone(),
        width,
        height,
        frame_format: CompositorFrameFormat::Rgba8,
        status: CompositorNodeStatus::Ready,
        status_detail,
        asset: None,
        text: Some(metadata),
        checksum,
        pixels,
    })
}

fn layout_text_glyphs(
    font: &FontArc,
    request: &TextRenderRequest,
    width: u32,
    height: u32,
) -> Vec<Glyph> {
    let scale = PxScale::from(request.font_size as f32);
    let scaled_font = font.as_scaled(scale);
    let mut cursor_x = 0.0_f32;
    let mut previous = None;
    let mut glyphs = Vec::new();

    for character in request.text.chars() {
        let glyph_id = font.glyph_id(character);
        if let Some(previous_id) = previous {
            cursor_x += scaled_font.kern(previous_id, glyph_id);
        }
        glyphs.push(Glyph {
            id: glyph_id,
            scale,
            position: point(cursor_x, 0.0),
        });
        cursor_x += scaled_font.h_advance(glyph_id);
        previous = Some(glyph_id);
    }

    let text_width = cursor_x.max(1.0);
    let inset = (width as f32 * 0.08).clamp(2.0, 24.0);
    let start_x = match request.align {
        TextAlign::Left => inset,
        TextAlign::Center => (width as f32 - text_width) / 2.0,
        TextAlign::Right => width as f32 - inset - text_width,
    };
    let ascent = scaled_font.ascent();
    let descent = scaled_font.descent();
    let text_height = (ascent - descent).max(1.0);
    let baseline = ((height as f32 - text_height) / 2.0) + ascent;

    for glyph in &mut glyphs {
        glyph.position.x += start_x;
        glyph.position.y += baseline;
    }

    glyphs
}

fn text_render_request(node: &CompositorNode) -> TextRenderRequest {
    let text = config_value_string(&node.config, "text").unwrap_or_default();
    let requested_font_family = config_value_string(&node.config, "font_family")
        .unwrap_or_else(|| SOFTWARE_TEXT_FONT_FAMILY.to_string());
    let font_size = config_value_number(&node.config, "font_size")
        .filter(|value| *value > 0.0)
        .unwrap_or_else(|| node.transform.size.height.max(1.0) * 0.58)
        .clamp(1.0, 512.0);
    let color_value =
        config_value_string(&node.config, "color").unwrap_or_else(|| "#f4f8ff".to_string());
    let (color, color_string, invalid_color) = parse_text_color(&color_value)
        .map(|color| (color, normalized_hex_color(color), false))
        .unwrap_or_else(|| ([244, 248, 255, 255], "#f4f8ff".to_string(), true));
    let align = match config_value_string(&node.config, "align")
        .unwrap_or_else(|| "center".to_string())
        .as_str()
    {
        "left" => TextAlign::Left,
        "right" => TextAlign::Right,
        _ => TextAlign::Center,
    };

    TextRenderRequest {
        text,
        requested_font_family,
        font_size,
        color,
        color_string,
        invalid_color,
        align,
    }
}

fn text_metadata(
    status: SoftwareCompositorTextStatus,
    status_detail: String,
    request: &TextRenderRequest,
    rendered_bounds: Option<CompositorRect>,
    checksum: Option<u64>,
) -> SoftwareCompositorTextMetadata {
    SoftwareCompositorTextMetadata {
        status,
        status_detail,
        requested_font_family: request.requested_font_family.clone(),
        used_font_family: SOFTWARE_TEXT_FONT_FAMILY.to_string(),
        font_size: request.font_size,
        color: request.color_string.clone(),
        align: request.align.as_str().to_string(),
        text_length: request.text.chars().count(),
        rendered_bounds,
        checksum,
    }
}

fn text_status_detail(request: &TextRenderRequest) -> String {
    if request.invalid_color {
        return format!(
            "Text rendered with fallback color {} because the configured color is invalid.",
            request.color_string
        );
    }
    if uses_font_fallback(&request.requested_font_family) {
        return format!(
            "Text rendered with bundled Inter because requested font \"{}\" is not bundled.",
            request.requested_font_family
        );
    }
    "Text rendered with bundled Inter.".to_string()
}

fn uses_font_fallback(requested_font_family: &str) -> bool {
    !requested_font_family
        .trim()
        .eq_ignore_ascii_case(SOFTWARE_TEXT_FONT_FAMILY)
}

impl TextAlign {
    fn as_str(&self) -> &'static str {
        match self {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
        }
    }
}

fn placeholder_input_frame_for_node(
    node: &CompositorNode,
    status: CompositorNodeStatus,
    status_detail: String,
    asset: Option<SoftwareCompositorAssetMetadata>,
    text: Option<SoftwareCompositorTextMetadata>,
) -> SoftwareCompositorInputFrame {
    let size = input_frame_size(node);
    let width = size.width.max(1.0).round().min(3840.0) as u32;
    let height = size.height.max(1.0).round().min(2160.0) as u32;
    let mut pixels = vec![0; width as usize * height as usize * 4];
    let mut style_node = node.clone();
    style_node.status = status.clone();
    let base = node_base_color(&style_node);
    let accent = node_accent_color(&style_node);
    let id_tint = stable_source_tint(&node.source_id);

    for y in 0..height as usize {
        for x in 0..width as usize {
            let checker = ((x / 24 + y / 24) % 2) as u8;
            let diagonal = ((x + y + id_tint as usize) / 18) % 7 == 0;
            let mut color = if checker == 0 {
                base
            } else {
                mix_color(base, accent, 0.22)
            };
            if diagonal {
                color = mix_color(color, [244, 248, 255, 255], 0.16);
            }
            if status != CompositorNodeStatus::Ready {
                color = mix_color(color, [140, 148, 166, 255], 0.34);
            }
            let offset = (y * width as usize + x) * 4;
            pixels[offset] = color[0].saturating_add(id_tint / 9);
            pixels[offset + 1] = color[1].saturating_add(id_tint / 13);
            pixels[offset + 2] = color[2].saturating_add(id_tint / 17);
            pixels[offset + 3] = color[3];
        }
    }

    let checksum = checksum_pixels(&pixels);
    SoftwareCompositorInputFrame {
        source_id: node.source_id.clone(),
        source_kind: node.source_kind.clone(),
        width,
        height,
        frame_format: CompositorFrameFormat::Rgba8,
        status,
        status_detail,
        asset,
        text,
        checksum,
        pixels,
    }
}

fn apply_software_input_validation(
    validation: &mut CompositorValidation,
    input_frames: &[SoftwareCompositorInputFrame],
) {
    for input in input_frames {
        if let Some(asset) = &input.asset {
            match asset.status {
                SoftwareCompositorAssetStatus::Decoded => {}
                SoftwareCompositorAssetStatus::MissingFile
                | SoftwareCompositorAssetStatus::UnsupportedExtension
                | SoftwareCompositorAssetStatus::DecodeFailed
                | SoftwareCompositorAssetStatus::VideoPlaceholder
                | SoftwareCompositorAssetStatus::NoAsset => {
                    validation.warnings.push(format!(
                        "{} image/media asset is using a placeholder: {}",
                        input.source_id, asset.status_detail
                    ));
                }
            }
        }
        if let Some(text) = &input.text {
            match text.status {
                SoftwareCompositorTextStatus::Rendered => {}
                SoftwareCompositorTextStatus::FontFallback
                | SoftwareCompositorTextStatus::InvalidColor => {
                    validation.warnings.push(format!(
                        "{} text rendered with fallback behavior: {}",
                        input.source_id, text.status_detail
                    ));
                }
                SoftwareCompositorTextStatus::Empty => {
                    validation.warnings.push(format!(
                        "{} text source is using a placeholder: {}",
                        input.source_id, text.status_detail
                    ));
                }
            }
        }
    }
    validation.ready = validation.errors.is_empty();
}

fn input_frame_size(node: &CompositorNode) -> SceneSize {
    match node.role {
        CompositorNodeRole::Audio => SceneSize {
            width: 512.0,
            height: 128.0,
        },
        CompositorNodeRole::Group => SceneSize {
            width: node.transform.size.width.max(64.0),
            height: node.transform.size.height.max(64.0),
        },
        _ => node_native_size(node, &node.transform, None),
    }
}

fn decode_image_asset(
    asset_uri: &str,
) -> Result<DecodedImageAsset, Box<SoftwareCompositorAssetMetadata>> {
    let Some(path) = asset_uri_path(asset_uri) else {
        return Err(Box::new(asset_metadata(
            asset_uri.to_string(),
            SoftwareCompositorAssetStatus::NoAsset,
            "No local image asset has been selected.".to_string(),
            None,
        )));
    };
    let normalized_path = normalized_asset_path(&path);
    let Some(format) = supported_image_extension(&path) else {
        return Err(Box::new(asset_metadata(
            asset_uri.to_string(),
            SoftwareCompositorAssetStatus::UnsupportedExtension,
            "Unsupported image extension. Supported image assets are png, jpg, jpeg, webp, and gif."
                .to_string(),
            None,
        )));
    };
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return Err(Box::new(asset_metadata(
                asset_uri.to_string(),
                SoftwareCompositorAssetStatus::MissingFile,
                format!("Image asset file does not exist: {normalized_path}"),
                Some(format),
            )));
        }
    };
    if !metadata.is_file() {
        return Err(Box::new(asset_metadata(
            asset_uri.to_string(),
            SoftwareCompositorAssetStatus::MissingFile,
            format!("Image asset path is not a file: {normalized_path}"),
            Some(format),
        )));
    }
    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(system_time_unix_ms)
        .unwrap_or(0);
    let key = ImageAssetCacheKey {
        path: normalized_path.clone(),
        modified_unix_ms,
    };
    if let Some(image) = cached_image_asset(&key) {
        return Ok(DecodedImageAsset {
            modified_unix_ms,
            cache_hit: true,
            image,
        });
    }

    let image = match ImageReader::open(&path)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(|error| error.to_string())
        .and_then(|reader| reader.decode().map_err(|error| error.to_string()))
    {
        Ok(image) => image,
        Err(error) => {
            return Err(Box::new(asset_metadata(
                asset_uri.to_string(),
                SoftwareCompositorAssetStatus::DecodeFailed,
                format!("Image asset could not be decoded: {error}"),
                Some(format),
            )));
        }
    };
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = rgba.into_raw();
    let decoded = CachedDecodedImage {
        width,
        height,
        format,
        checksum: checksum_pixels(&pixels),
        pixels,
    };
    store_cached_image_asset(key, decoded.clone());

    Ok(DecodedImageAsset {
        modified_unix_ms,
        cache_hit: false,
        image: decoded,
    })
}

fn asset_uri_path(asset_uri: &str) -> Option<PathBuf> {
    let trimmed = asset_uri.trim();
    if trimmed.is_empty() {
        return None;
    }
    let path = trimmed.strip_prefix("file://").unwrap_or(trimmed);
    Some(PathBuf::from(path))
}

fn normalized_asset_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn supported_image_extension(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" => Some(extension),
        _ => None,
    }
}

fn cached_image_asset(key: &ImageAssetCacheKey) -> Option<CachedDecodedImage> {
    IMAGE_ASSET_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|cache| cache.get(key).cloned())
}

fn store_cached_image_asset(key: ImageAssetCacheKey, image: CachedDecodedImage) {
    let Ok(mut cache) = IMAGE_ASSET_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    else {
        return;
    };
    cache.retain(|existing_key, _| {
        existing_key.path != key.path || existing_key.modified_unix_ms == key.modified_unix_ms
    });
    cache.insert(key, image);
    if cache.len() > 64 {
        let Some(first_key) = cache.keys().next().cloned() else {
            return;
        };
        cache.remove(&first_key);
    }
}

fn asset_metadata(
    uri: String,
    status: SoftwareCompositorAssetStatus,
    status_detail: String,
    format: Option<String>,
) -> SoftwareCompositorAssetMetadata {
    SoftwareCompositorAssetMetadata {
        uri,
        status,
        status_detail,
        format,
        width: None,
        height: None,
        checksum: None,
        modified_unix_ms: None,
        cache_hit: false,
    }
}

fn inter_font() -> Result<FontArc, String> {
    INTER_FONT
        .get_or_init(|| {
            FontArc::try_from_slice(INTER_FONT_BYTES)
                .map_err(|error| format!("Bundled Inter font could not be loaded: {error:?}"))
        })
        .clone()
}

fn system_time_unix_ms(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

fn input_frame_pixel(
    input_frame: &SoftwareCompositorInputFrame,
    source_x: usize,
    source_y: usize,
) -> [u8; 4] {
    let width = input_frame.width.max(1) as usize;
    let height = input_frame.height.max(1) as usize;
    let x = source_x.min(width - 1);
    let y = source_y.min(height - 1);
    let offset = (y * width + x) * 4;
    [
        input_frame.pixels[offset],
        input_frame.pixels[offset + 1],
        input_frame.pixels[offset + 2],
        input_frame.pixels[offset + 3],
    ]
}

fn source_sample_for_target_pixel(
    x: usize,
    y: usize,
    rect: &CompositorRect,
    rotation_degrees: f64,
    input_frame: &SoftwareCompositorInputFrame,
) -> Option<(usize, usize)> {
    let point_x = x as f64 + 0.5;
    let point_y = y as f64 + 0.5;
    let center_x = rect.x + rect.width / 2.0;
    let center_y = rect.y + rect.height / 2.0;
    let (local_x, local_y) = if rotation_degrees.abs() > f64::EPSILON {
        rotate_point(point_x, point_y, center_x, center_y, -rotation_degrees)
    } else {
        (point_x, point_y)
    };

    if local_x < rect.x
        || local_x >= rect.x + rect.width
        || local_y < rect.y
        || local_y >= rect.y + rect.height
    {
        return None;
    }

    let u = ((local_x - rect.x) / rect.width).clamp(0.0, 0.999_999);
    let v = ((local_y - rect.y) / rect.height).clamp(0.0, 0.999_999);
    let source_x = (u * f64::from(input_frame.width.max(1))).floor() as usize;
    let source_y = (v * f64::from(input_frame.height.max(1))).floor() as usize;
    Some((
        source_x.min(input_frame.width.saturating_sub(1) as usize),
        source_y.min(input_frame.height.saturating_sub(1) as usize),
    ))
}

fn rotated_bounds(rect: &CompositorRect, rotation_degrees: f64) -> CompositorRect {
    if rotation_degrees.abs() <= f64::EPSILON {
        return rect.clone();
    }

    let center_x = rect.x + rect.width / 2.0;
    let center_y = rect.y + rect.height / 2.0;
    let corners = [
        rotate_point(rect.x, rect.y, center_x, center_y, rotation_degrees),
        rotate_point(
            rect.x + rect.width,
            rect.y,
            center_x,
            center_y,
            rotation_degrees,
        ),
        rotate_point(
            rect.x + rect.width,
            rect.y + rect.height,
            center_x,
            center_y,
            rotation_degrees,
        ),
        rotate_point(
            rect.x,
            rect.y + rect.height,
            center_x,
            center_y,
            rotation_degrees,
        ),
    ];
    let min_x = corners
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::INFINITY, f64::min);
    let max_x = corners
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = corners
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::INFINITY, f64::min);
    let max_y = corners
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);

    CompositorRect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

fn rotate_point(x: f64, y: f64, center_x: f64, center_y: f64, rotation_degrees: f64) -> (f64, f64) {
    let radians = rotation_degrees.to_radians();
    let sin = radians.sin();
    let cos = radians.cos();
    let translated_x = x - center_x;
    let translated_y = y - center_y;
    (
        center_x + translated_x * cos - translated_y * sin,
        center_y + translated_x * sin + translated_y * cos,
    )
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

fn node_base_color(node: &CompositorNode) -> [u8; 4] {
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
    [red, green, blue, 255]
}

fn node_accent_color(node: &CompositorNode) -> [u8; 4] {
    match node.source_kind {
        SceneSourceKind::Display => [45, 151, 230, 255],
        SceneSourceKind::Window => [82, 118, 232, 255],
        SceneSourceKind::Camera => [77, 205, 189, 255],
        SceneSourceKind::AudioMeter => [66, 214, 139, 255],
        SceneSourceKind::ImageMedia => [224, 146, 58, 255],
        SceneSourceKind::BrowserOverlay => [209, 99, 191, 255],
        SceneSourceKind::Text => [196, 114, 230, 255],
        SceneSourceKind::Group => [148, 163, 184, 255],
    }
}

fn mix_color(left: [u8; 4], right: [u8; 4], amount: f64) -> [u8; 4] {
    let amount = amount.clamp(0.0, 1.0);
    [
        mix_channel(left[0], right[0], amount),
        mix_channel(left[1], right[1], amount),
        mix_channel(left[2], right[2], amount),
        mix_channel(left[3], right[3], amount),
    ]
}

fn mix_channel(left: u8, right: u8, amount: f64) -> u8 {
    (f64::from(left) + (f64::from(right) - f64::from(left)) * amount)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn stable_source_tint(value: &str) -> u8 {
    value
        .bytes()
        .fold(0_u8, |hash, byte| hash.wrapping_mul(31).wrapping_add(byte))
        % 48
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

fn parse_text_color(value: &str) -> Option<[u8; 4]> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }

    Some([
        parse_hex_channel(&hex[0..2])?,
        parse_hex_channel(&hex[2..4])?,
        parse_hex_channel(&hex[4..6])?,
        255,
    ])
}

fn normalized_hex_color(color: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
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

fn alpha_bounds(pixels: &[u8], width: u32, height: u32) -> Option<CompositorRect> {
    let width = width.max(1) as usize;
    let height = height.max(1) as usize;
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0_usize;
    let mut max_y = 0_usize;
    let mut found = false;

    for y in 0..height {
        for x in 0..width {
            let alpha = pixels[(y * width + x) * 4 + 3];
            if alpha == 0 {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    found.then(|| CompositorRect {
        x: min_x as f64,
        y: min_y as f64,
        width: (max_x - min_x + 1) as f64,
        height: (max_y - min_y + 1) as f64,
    })
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
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::{thread, time::Duration};
    use tempfile::tempdir;

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
        assert_eq!(result.input_frames.len(), graph.nodes.len());
        assert!(result
            .input_frames
            .iter()
            .all(|input| input.frame_format == CompositorFrameFormat::Rgba8));
        assert_eq!(result.pixel_frames.len(), 1);
        let target = &result.pixel_frames[0];
        assert_eq!(target.frame_format, CompositorFrameFormat::Rgba8);
        assert_eq!(target.bytes_per_row, 1280 * 4);
        assert_eq!(target.pixels.len(), 1280 * 720 * 4);
        assert_ne!(target.checksum, 0);
        assert_ne!(&target.pixels[0..4], &[5, 7, 17, 255]);
    }

    #[test]
    fn software_compositor_decodes_supported_image_assets() {
        for (extension, format) in [
            ("png", ImageFormat::Png),
            ("jpg", ImageFormat::Jpeg),
            ("webp", ImageFormat::WebP),
            ("gif", ImageFormat::Gif),
        ] {
            let dir = tempdir().unwrap();
            let path = dir.path().join(format!("asset.{extension}"));
            write_test_image(&path, format, [220, 20, 40, 255]);

            let result = render_test_image_source(&path.display().to_string(), None);
            let input = result
                .input_frames
                .iter()
                .find(|frame| frame.source_id == "source-image")
                .unwrap();
            let asset = input.asset.as_ref().unwrap();

            assert_eq!(input.status, CompositorNodeStatus::Ready);
            assert_eq!(asset.status, SoftwareCompositorAssetStatus::Decoded);
            assert_eq!(asset.width, Some(4));
            assert_eq!(asset.height, Some(4));
            assert_eq!(asset.format.as_deref(), Some(extension));
            assert!(asset.checksum.is_some_and(|checksum| checksum > 0));
            assert!(result.frame.validation.ready);
            assert_ne!(
                software_test_pixel(&result.pixel_frames[0], 4, 4),
                [5, 7, 17, 255]
            );
        }
    }

    #[test]
    fn software_compositor_reports_image_asset_placeholder_states() {
        let dir = tempdir().unwrap();
        let missing_path = dir.path().join("missing.png");
        let missing = render_test_image_source(&missing_path.display().to_string(), None);
        let missing_asset = missing.input_frames[0].asset.as_ref().unwrap();
        assert_eq!(
            missing_asset.status,
            SoftwareCompositorAssetStatus::MissingFile
        );
        assert_eq!(
            missing.input_frames[0].status,
            CompositorNodeStatus::Placeholder
        );
        assert!(!missing.frame.validation.warnings.is_empty());

        let unsupported_path = dir.path().join("asset.txt");
        fs::write(&unsupported_path, b"not an image").unwrap();
        let unsupported = render_test_image_source(&unsupported_path.display().to_string(), None);
        assert_eq!(
            unsupported.input_frames[0].asset.as_ref().unwrap().status,
            SoftwareCompositorAssetStatus::UnsupportedExtension
        );

        let broken_path = dir.path().join("broken.png");
        fs::write(&broken_path, b"not a png").unwrap();
        let broken = render_test_image_source(&broken_path.display().to_string(), None);
        assert_eq!(
            broken.input_frames[0].asset.as_ref().unwrap().status,
            SoftwareCompositorAssetStatus::DecodeFailed
        );

        let video = render_test_image_source("clip.webm", Some("video"));
        assert_eq!(
            video.input_frames[0].asset.as_ref().unwrap().status,
            SoftwareCompositorAssetStatus::VideoPlaceholder
        );
    }

    #[test]
    fn software_compositor_renders_text_source_pixels() {
        let result = render_test_text_source("VAEX", "Inter", "#f4f8ff", "center", 42.0);
        let input = result
            .input_frames
            .iter()
            .find(|frame| frame.source_id == "source-text")
            .unwrap();
        let text = input.text.as_ref().unwrap();

        assert_eq!(input.status, CompositorNodeStatus::Ready);
        assert_eq!(text.status, SoftwareCompositorTextStatus::Rendered);
        assert_eq!(text.used_font_family, "Inter");
        assert_eq!(text.text_length, 4);
        assert!(text.rendered_bounds.is_some());
        assert!(text.checksum.is_some_and(|checksum| checksum > 0));
        assert!(alpha_pixel_count(input) > 0);
        assert_ne!(result.pixel_frames[0].checksum, 0);
    }

    #[test]
    fn software_compositor_reports_empty_text_placeholder_state() {
        let result = render_test_text_source(" ", "Inter", "#f4f8ff", "center", 42.0);
        let input = result
            .input_frames
            .iter()
            .find(|frame| frame.source_id == "source-text")
            .unwrap();
        let text = input.text.as_ref().unwrap();

        assert_eq!(input.status, CompositorNodeStatus::Placeholder);
        assert_eq!(text.status, SoftwareCompositorTextStatus::Empty);
        assert!(result
            .frame
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("Text source is empty")));
    }

    #[test]
    fn software_compositor_reports_invalid_text_color_fallback() {
        let result = render_test_text_source("VAEX", "Inter", "not-a-color", "center", 42.0);
        let input = result
            .input_frames
            .iter()
            .find(|frame| frame.source_id == "source-text")
            .unwrap();
        let text = input.text.as_ref().unwrap();

        assert_eq!(input.status, CompositorNodeStatus::Ready);
        assert_eq!(text.status, SoftwareCompositorTextStatus::InvalidColor);
        assert_eq!(text.color, "#f4f8ff");
        assert!(result
            .frame
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("fallback color")));
    }

    #[test]
    fn software_compositor_applies_text_alignment() {
        let left = render_test_text_source("VAEX", "Inter", "#f4f8ff", "left", 42.0);
        let center = render_test_text_source("VAEX", "Inter", "#f4f8ff", "center", 42.0);
        let right = render_test_text_source("VAEX", "Inter", "#f4f8ff", "right", 42.0);

        let left_bounds = non_background_bounds(&left.pixel_frames[0], [5, 7, 17, 255]).unwrap();
        let center_bounds =
            non_background_bounds(&center.pixel_frames[0], [5, 7, 17, 255]).unwrap();
        let right_bounds = non_background_bounds(&right.pixel_frames[0], [5, 7, 17, 255]).unwrap();

        assert!(left_bounds.x < center_bounds.x);
        assert!(center_bounds.x < right_bounds.x);
    }

    #[test]
    fn software_compositor_text_font_size_changes_bounds_and_checksum() {
        let small = render_test_text_source("VAEX", "Inter", "#f4f8ff", "center", 20.0);
        let large = render_test_text_source("VAEX", "Inter", "#f4f8ff", "center", 48.0);
        let small_text = small.input_frames[0].text.as_ref().unwrap();
        let large_text = large.input_frames[0].text.as_ref().unwrap();

        assert!(
            small_text.rendered_bounds.as_ref().unwrap().height
                < large_text.rendered_bounds.as_ref().unwrap().height
        );
        assert_ne!(
            small.input_frames[0].checksum,
            large.input_frames[0].checksum
        );
    }

    #[test]
    fn software_compositor_applies_crop_opacity_and_z_order_to_text() {
        let dir = tempdir().unwrap();
        let bottom_path = dir.path().join("bottom.png");
        write_test_image(&bottom_path, ImageFormat::Png, [0, 0, 255, 255]);

        let mut scene = test_image_scene(&bottom_path.display().to_string(), None);
        scene.canvas.width = 96;
        scene.canvas.height = 64;
        scene.sources[0].size = SceneSize {
            width: 96.0,
            height: 64.0,
        };
        scene.sources[0].z_index = 0;
        scene.sources.push(SceneSource {
            id: "source-text".to_string(),
            name: "Top Text".to_string(),
            kind: SceneSourceKind::Text,
            position: ScenePoint { x: 0.0, y: 0.0 },
            size: SceneSize {
                width: 96.0,
                height: 64.0,
            },
            crop: SceneCrop {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 48.0,
            },
            rotation_degrees: 0.0,
            opacity: 0.5,
            visible: true,
            locked: false,
            z_index: 20,
            bounds_mode: SceneSourceBoundsMode::Stretch,
            filters: Vec::new(),
            config: serde_json::json!({
                "text": "MMMM",
                "font_family": "Inter",
                "font_size": 60,
                "color": "#ff0000",
                "align": "center"
            }),
        });

        let result = render_test_scene_with_target(scene, 96, 64);
        let left_blended = count_pixels_in_region(&result.pixel_frames[0], 0, 48, blended_red_blue);
        let right_blended =
            count_pixels_in_region(&result.pixel_frames[0], 48, 96, blended_red_blue);

        assert_eq!(left_blended, 0);
        assert!(right_blended > 0);
    }

    #[test]
    fn software_compositor_reports_text_font_fallback() {
        let result = render_test_text_source("VAEX", "Papyrus", "#f4f8ff", "center", 42.0);
        let text = result.input_frames[0].text.as_ref().unwrap();

        assert_eq!(text.status, SoftwareCompositorTextStatus::FontFallback);
        assert_eq!(text.requested_font_family, "Papyrus");
        assert_eq!(text.used_font_family, "Inter");
        assert!(result
            .frame
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("requested font")));
    }

    #[test]
    fn software_compositor_uses_decoded_image_dimensions_for_bounds_modes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tall.png");
        write_test_image_size(&path, ImageFormat::Png, [40, 220, 80, 255], 2, 4);
        let mut result = render_test_image_source(&path.display().to_string(), None);
        let node = result.frame.targets[0]
            .nodes
            .iter()
            .find(|node| node.source_id == "source-image")
            .unwrap();
        assert_eq!(node.rect.width, 8.0);
        assert_eq!(node.rect.height, 8.0);

        result = render_test_image_source_with_bounds(
            &path.display().to_string(),
            SceneSourceBoundsMode::Fit,
            SceneSize {
                width: 8.0,
                height: 8.0,
            },
        );
        let node = result.frame.targets[0]
            .nodes
            .iter()
            .find(|node| node.source_id == "source-image")
            .unwrap();
        assert_eq!(node.rect.x, 2.0);
        assert_eq!(node.rect.width, 4.0);
        assert_eq!(node.rect.height, 8.0);
        assert_eq!(node.asset.as_ref().unwrap().width, Some(2));
        assert_ne!(result.pixel_frames[0].checksum, 0);
    }

    #[test]
    fn software_compositor_invalidates_image_cache_when_file_changes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cached.png");
        write_test_image(&path, ImageFormat::Png, [255, 0, 0, 255]);

        let first = render_test_image_source(&path.display().to_string(), None);
        assert!(!first.input_frames[0].asset.as_ref().unwrap().cache_hit);
        let second = render_test_image_source(&path.display().to_string(), None);
        assert!(second.input_frames[0].asset.as_ref().unwrap().cache_hit);

        let first_checksum = first.input_frames[0].checksum;
        wait_for_distinct_mtime(&path, || {
            write_test_image(&path, ImageFormat::Png, [0, 0, 255, 255]);
        });
        let third = render_test_image_source(&path.display().to_string(), None);
        assert!(!third.input_frames[0].asset.as_ref().unwrap().cache_hit);
        assert_ne!(third.input_frames[0].checksum, first_checksum);
    }

    #[test]
    fn software_compositor_applies_crop_opacity_and_z_order_to_decoded_images() {
        let dir = tempdir().unwrap();
        let bottom_path = dir.path().join("bottom.png");
        let top_path = dir.path().join("top.png");
        write_test_image(&bottom_path, ImageFormat::Png, [0, 0, 255, 255]);
        write_test_image(&top_path, ImageFormat::Png, [255, 0, 0, 255]);

        let mut scene = test_image_scene(&bottom_path.display().to_string(), None);
        scene.sources.push(SceneSource {
            id: "source-top-image".to_string(),
            name: "Top Image".to_string(),
            kind: SceneSourceKind::ImageMedia,
            position: ScenePoint { x: 0.0, y: 0.0 },
            size: SceneSize {
                width: 8.0,
                height: 8.0,
            },
            crop: SceneCrop {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 4.0,
            },
            rotation_degrees: 0.0,
            opacity: 0.5,
            visible: true,
            locked: false,
            z_index: 20,
            bounds_mode: SceneSourceBoundsMode::Stretch,
            filters: Vec::new(),
            config: serde_json::json!({
                "asset_uri": top_path.display().to_string(),
                "media_type": "image"
            }),
        });
        let result = render_test_scene(scene);
        let left_pixel = software_test_pixel(&result.pixel_frames[0], 2, 4);
        let right_pixel = software_test_pixel(&result.pixel_frames[0], 6, 4);

        assert!(left_pixel[2] > left_pixel[0]);
        assert!(right_pixel[0] > 100);
        assert!(right_pixel[2] > 100);
        assert!(result
            .input_frames
            .iter()
            .all(|frame| frame.status == CompositorNodeStatus::Ready));
    }

    #[test]
    fn software_compositor_applies_crop_opacity_rotation_and_z_order() {
        let mut collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.scenes.first_mut().unwrap();
        scene.canvas.width = 64;
        scene.canvas.height = 64;
        scene.sources.retain(|source| {
            source.id == "source-main-display" || source.id == "source-camera-placeholder"
        });

        let display = scene
            .sources
            .iter_mut()
            .find(|source| source.id == "source-main-display")
            .unwrap();
        display.position = ScenePoint { x: 0.0, y: 0.0 };
        display.size = SceneSize {
            width: 64.0,
            height: 64.0,
        };
        display.z_index = 0;
        display.config["availability"] =
            serde_json::json!({ "state": "available", "detail": "Display ready." });

        let camera = scene
            .sources
            .iter_mut()
            .find(|source| source.id == "source-camera-placeholder")
            .unwrap();
        camera.position = ScenePoint { x: 0.0, y: 0.0 };
        camera.size = SceneSize {
            width: 64.0,
            height: 64.0,
        };
        camera.crop.left = 32.0;
        camera.rotation_degrees = 12.0;
        camera.opacity = 0.7;
        camera.z_index = 20;
        camera.config["availability"] =
            serde_json::json!({ "state": "available", "detail": "Camera ready." });

        let graph = build_compositor_graph(scene);
        let plan = build_compositor_render_plan(
            &graph,
            vec![compositor_render_target(
                "preview",
                "Preview",
                CompositorRenderTargetKind::Preview,
                64,
                64,
                30,
            )],
        );

        let result = render_software_compositor_frame(&plan, 3);
        let target = &result.pixel_frames[0];
        let left_pixel = software_test_pixel(target, 12, 32);
        let right_pixel = software_test_pixel(target, 52, 32);

        assert_eq!(result.input_frames.len(), 2);
        assert_ne!(left_pixel, right_pixel);
        assert_ne!(target.checksum, 0);
        assert!(result.frame.targets[0]
            .nodes
            .iter()
            .any(|node| node.source_id == "source-camera-placeholder"
                && node.rotation_degrees == 12.0
                && (node.opacity - 0.7).abs() < f64::EPSILON));
    }

    fn write_test_image(path: &Path, format: ImageFormat, color: [u8; 4]) {
        write_test_image_size(path, format, color, 4, 4);
    }

    fn write_test_image_size(
        path: &Path,
        format: ImageFormat,
        color: [u8; 4],
        width: u32,
        height: u32,
    ) {
        let image = RgbaImage::from_pixel(width, height, Rgba(color));
        image::DynamicImage::ImageRgba8(image)
            .save_with_format(path, format)
            .unwrap();
    }

    fn render_test_image_source(
        asset_uri: &str,
        media_type: Option<&str>,
    ) -> SoftwareCompositorRenderResult {
        render_test_scene(test_image_scene(asset_uri, media_type))
    }

    fn render_test_image_source_with_bounds(
        asset_uri: &str,
        bounds_mode: SceneSourceBoundsMode,
        size: SceneSize,
    ) -> SoftwareCompositorRenderResult {
        let mut scene = test_image_scene(asset_uri, None);
        scene.sources[0].bounds_mode = bounds_mode;
        scene.sources[0].size = size;
        render_test_scene(scene)
    }

    fn test_image_scene(asset_uri: &str, media_type: Option<&str>) -> Scene {
        Scene {
            id: "scene-image".to_string(),
            name: "Image Scene".to_string(),
            canvas: crate::SceneCanvas {
                width: 8,
                height: 8,
                background_color: "#050711".to_string(),
            },
            sources: vec![SceneSource {
                id: "source-image".to_string(),
                name: "Image".to_string(),
                kind: SceneSourceKind::ImageMedia,
                position: ScenePoint { x: 0.0, y: 0.0 },
                size: SceneSize {
                    width: 8.0,
                    height: 8.0,
                },
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
                z_index: 10,
                bounds_mode: SceneSourceBoundsMode::Stretch,
                filters: Vec::new(),
                config: serde_json::json!({
                    "asset_uri": asset_uri,
                    "media_type": media_type.unwrap_or("image")
                }),
            }],
        }
    }

    fn render_test_text_source(
        text: &str,
        font_family: &str,
        color: &str,
        align: &str,
        font_size: f64,
    ) -> SoftwareCompositorRenderResult {
        render_test_scene_with_target(
            Scene {
                id: "scene-text".to_string(),
                name: "Text Scene".to_string(),
                canvas: crate::SceneCanvas {
                    width: 160,
                    height: 80,
                    background_color: "#050711".to_string(),
                },
                sources: vec![SceneSource {
                    id: "source-text".to_string(),
                    name: "Text".to_string(),
                    kind: SceneSourceKind::Text,
                    position: ScenePoint { x: 0.0, y: 0.0 },
                    size: SceneSize {
                        width: 160.0,
                        height: 80.0,
                    },
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
                    z_index: 10,
                    bounds_mode: SceneSourceBoundsMode::Stretch,
                    filters: Vec::new(),
                    config: serde_json::json!({
                        "text": text,
                        "font_family": font_family,
                        "font_size": font_size,
                        "color": color,
                        "align": align
                    }),
                }],
            },
            160,
            80,
        )
    }

    fn render_test_scene(scene: Scene) -> SoftwareCompositorRenderResult {
        render_test_scene_with_target(scene, 8, 8)
    }

    fn render_test_scene_with_target(
        scene: Scene,
        width: u32,
        height: u32,
    ) -> SoftwareCompositorRenderResult {
        let graph = build_compositor_graph(&scene);
        let plan = build_compositor_render_plan(
            &graph,
            vec![compositor_render_target(
                "preview",
                "Preview",
                CompositorRenderTargetKind::Preview,
                width,
                height,
                30,
            )],
        );
        render_software_compositor_frame(&plan, 0)
    }

    fn alpha_pixel_count(input: &SoftwareCompositorInputFrame) -> usize {
        input
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 0)
            .count()
    }

    fn non_background_bounds(
        frame: &SoftwareCompositorFrame,
        background: [u8; 4],
    ) -> Option<CompositorRect> {
        let width = frame.width as usize;
        let height = frame.height as usize;
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0_usize;
        let mut max_y = 0_usize;
        let mut found = false;

        for y in 0..height {
            for x in 0..width {
                let pixel = software_test_pixel(frame, x, y);
                if pixel == background {
                    continue;
                }
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }

        found.then(|| CompositorRect {
            x: min_x as f64,
            y: min_y as f64,
            width: (max_x - min_x + 1) as f64,
            height: (max_y - min_y + 1) as f64,
        })
    }

    fn count_pixels_in_region(
        frame: &SoftwareCompositorFrame,
        start_x: usize,
        end_x: usize,
        predicate: impl Fn([u8; 4]) -> bool,
    ) -> usize {
        let mut count = 0;
        for y in 0..frame.height as usize {
            for x in start_x..end_x.min(frame.width as usize) {
                if predicate(software_test_pixel(frame, x, y)) {
                    count += 1;
                }
            }
        }
        count
    }

    fn blended_red_blue(pixel: [u8; 4]) -> bool {
        pixel[0] > 40 && pixel[2] > 80 && pixel[3] == 255
    }

    fn wait_for_distinct_mtime(path: &Path, write: impl Fn()) {
        let before = fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok();
        for _ in 0..8 {
            thread::sleep(Duration::from_millis(25));
            write();
            let after = fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok();
            if before != after {
                return;
            }
        }
    }

    fn software_test_pixel(frame: &SoftwareCompositorFrame, x: usize, y: usize) -> [u8; 4] {
        let offset = (y * frame.width as usize + x) * 4;
        [
            frame.pixels[offset],
            frame.pixels[offset + 1],
            frame.pixels[offset + 2],
            frame.pixels[offset + 3],
        ]
    }
}
