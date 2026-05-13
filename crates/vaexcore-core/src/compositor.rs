use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use ab_glyph::{point, Font, FontArc, Glyph, PxScale, ScaleFont};
use base64::{engine::general_purpose, Engine as _};
use image::ImageReader;
use serde::{Deserialize, Serialize};
use tungstenite::{stream::MaybeTlsStream, Message, WebSocket};
use uuid::Uuid;

use crate::{
    CaptureSourceKind, Scene, SceneCrop, ScenePoint, SceneSize, SceneSource, SceneSourceBoundsMode,
    SceneSourceFilter, SceneSourceFilterKind, SceneSourceKind,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<SoftwareCompositorBrowserMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<SoftwareCompositorCaptureMetadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<SoftwareCompositorFilterMetadata>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<SoftwareCompositorBrowserMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<SoftwareCompositorCaptureMetadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<SoftwareCompositorFilterMetadata>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampled_frame_time_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoder_name: Option<String>,
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

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareCompositorBrowserStatus {
    Rendered,
    NoUrl,
    BrowserUnavailable,
    UnsupportedUrl,
    NavigationFailed,
    CaptureFailed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoftwareCompositorBrowserMetadata {
    pub status: SoftwareCompositorBrowserStatus,
    pub status_detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub viewport_width: u32,
    pub viewport_height: u32,
    #[serde(default)]
    pub custom_css_present: bool,
    #[serde(default)]
    pub custom_css_applied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_css_detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampled_frame_time_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_duration_ms: Option<u64>,
    #[serde(default)]
    pub cache_hit: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareCompositorCaptureStatus {
    Rendered,
    NoSource,
    PermissionRequired,
    DecoderUnavailable,
    UnsupportedPlatform,
    UnsupportedSource,
    CaptureFailed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoftwareCompositorCaptureMetadata {
    pub status: SoftwareCompositorCaptureStatus,
    pub status_detail: String,
    pub capture_source_id: Option<String>,
    pub capture_kind: CaptureSourceKind,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_index: u64,
    pub checksum: Option<u64>,
    pub capture_duration_ms: Option<u64>,
    pub latency_ms: Option<f64>,
    pub dropped_frames: u64,
    pub provider_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareCompositorFilterStatus {
    Applied,
    Skipped,
    Deferred,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SoftwareCompositorFilterMetadata {
    pub id: String,
    pub name: String,
    pub kind: SceneSourceFilterKind,
    pub status: SoftwareCompositorFilterStatus,
    pub status_detail: String,
    pub order: i32,
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct BrowserSnapshotCacheKey {
    url: String,
    viewport_width: u32,
    viewport_height: u32,
    custom_css_hash: u64,
    sample_time_ms: u64,
}

#[derive(Clone, Debug)]
struct CachedBrowserSnapshot {
    width: u32,
    height: u32,
    checksum: u64,
    pixels: Vec<u8>,
}

#[derive(Clone, Debug)]
struct BrowserSnapshot {
    width: u32,
    height: u32,
    checksum: u64,
    pixels: Vec<u8>,
    browser_name: String,
    browser_path: String,
    sample_time_ms: u64,
    sample_index: u64,
    capture_duration_ms: u64,
    custom_css_applied: bool,
    custom_css_detail: Option<String>,
    cache_hit: bool,
}

#[derive(Clone, Debug)]
struct BrowserBinary {
    name: String,
    path: PathBuf,
}

#[derive(Clone, Debug)]
struct BrowserSnapshotRequest {
    url: String,
    viewport_width: u32,
    viewport_height: u32,
    custom_css: String,
    sample_time_ms: u64,
    sample_index: u64,
}

#[derive(Clone, Debug)]
struct BrowserCaptureError {
    status: SoftwareCompositorBrowserStatus,
    detail: String,
    custom_css_applied: bool,
    custom_css_detail: Option<String>,
}

#[derive(Clone, Debug)]
struct DecodedImageAsset {
    modified_unix_ms: u64,
    cache_hit: bool,
    image: CachedDecodedImage,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct VideoAssetCacheKey {
    path: String,
    modified_unix_ms: u64,
    sample_time_ms: u64,
}

#[derive(Clone, Debug)]
struct DecodedVideoAsset {
    modified_unix_ms: u64,
    sample_time_ms: u64,
    sample_index: u64,
    decoder_name: String,
    cache_hit: bool,
    image: CachedDecodedImage,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct LutAssetCacheKey {
    path: String,
    modified_unix_ms: u64,
}

#[derive(Clone, Debug)]
struct CachedCubeLut {
    size: usize,
    domain_min: [f64; 3],
    domain_max: [f64; 3],
    values: Vec<[f64; 3]>,
    checksum: u64,
}

#[derive(Clone, Debug)]
struct DecodedCubeLut {
    modified_unix_ms: u64,
    cache_hit: bool,
    lut: CachedCubeLut,
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
static BROWSER_SNAPSHOT_CACHE: OnceLock<
    Mutex<HashMap<BrowserSnapshotCacheKey, CachedBrowserSnapshot>>,
> = OnceLock::new();
static VIDEO_ASSET_CACHE: OnceLock<Mutex<HashMap<VideoAssetCacheKey, CachedDecodedImage>>> =
    OnceLock::new();
static LUT_ASSET_CACHE: OnceLock<Mutex<HashMap<LutAssetCacheKey, CachedCubeLut>>> = OnceLock::new();
static INTER_FONT: OnceLock<Result<FontArc, String>> = OnceLock::new();
const INTER_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/Inter.ttf");
const SOFTWARE_TEXT_FONT_FAMILY: &str = "Inter";
const BROWSER_SAMPLE_INTERVAL_MS: u64 = 1_000;
const VIDEO_SAMPLE_INTERVAL_MS: u64 = 500;

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
    let input_clock = software_frame_clock_for_plan(plan, frame_index);
    let input_frames = build_software_compositor_input_frames_at_clock(&plan.graph, &input_clock);
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
    let clock = default_software_input_clock();
    build_software_compositor_input_frames_at_clock(graph, &clock)
}

pub fn build_software_compositor_input_frames_at_clock(
    graph: &CompositorGraph,
    clock: &CompositorFrameClock,
) -> Vec<SoftwareCompositorInputFrame> {
    graph
        .nodes
        .iter()
        .filter(|node| node.visible)
        .map(|node| software_input_frame_for_node(node, clock))
        .collect()
}

pub fn stinger_video_input_frame(
    asset_uri: &str,
    clock: &CompositorFrameClock,
    width: u32,
    height: u32,
) -> SoftwareCompositorInputFrame {
    let node = CompositorNode {
        id: "node-transition-stinger".to_string(),
        source_id: "transition-stinger".to_string(),
        name: "Stinger Transition".to_string(),
        source_kind: SceneSourceKind::ImageMedia,
        role: CompositorNodeRole::Overlay,
        parent_source_id: None,
        group_depth: 0,
        transform: CompositorTransform {
            position: ScenePoint { x: 0.0, y: 0.0 },
            size: SceneSize {
                width: f64::from(width.max(1)),
                height: f64::from(height.max(1)),
            },
            crop: SceneCrop {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
            rotation_degrees: 0.0,
            opacity: 1.0,
        },
        visible: true,
        locked: true,
        z_index: i32::MAX,
        blend_mode: CompositorBlendMode::Normal,
        scale_mode: CompositorScaleMode::Stretch,
        status: CompositorNodeStatus::Placeholder,
        status_detail: "Stinger transition asset preview.".to_string(),
        filters: Vec::new(),
        config: serde_json::json!({
            "asset_uri": asset_uri,
            "media_type": "video"
        }),
    };

    image_media_input_frame_for_node(&node, clock)
}

fn default_software_input_clock() -> CompositorFrameClock {
    CompositorFrameClock {
        frame_index: 0,
        framerate: 30,
        pts_nanos: 0,
        duration_nanos: 1_000_000_000_u64 / 30,
    }
}

fn software_frame_clock_for_plan(
    plan: &CompositorRenderPlan,
    frame_index: u64,
) -> CompositorFrameClock {
    let framerate = plan
        .targets
        .iter()
        .find(|target| target.enabled)
        .map(|target| target.framerate)
        .unwrap_or(30)
        .max(1);
    let duration_nanos = 1_000_000_000_u64 / u64::from(framerate);
    CompositorFrameClock {
        frame_index,
        framerate,
        pts_nanos: frame_index.saturating_mul(duration_nanos),
        duration_nanos,
    }
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
    let browser = input_frame.and_then(|frame| frame.browser.clone());
    let capture = input_frame.and_then(|frame| frame.capture.clone());
    let filters = input_frame
        .map(|frame| frame.filters.clone())
        .unwrap_or_default();
    CompositorEvaluatedNode {
        node_id: node.id.clone(),
        source_id: node.source_id.clone(),
        name: node.name.clone(),
        role: node.role.clone(),
        status,
        status_detail,
        asset,
        text,
        browser,
        capture,
        filters,
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

fn config_bool(config: &serde_json::Value, key: &str) -> Option<bool> {
    config.get(key).and_then(serde_json::Value::as_bool)
}

fn config_value_number(config: &serde_json::Value, key: &str) -> Option<f64> {
    config
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .filter(|value| value.is_finite())
}

fn filter_config_number(filter: &SceneSourceFilter, key: &str, fallback: f64) -> f64 {
    filter
        .config
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}

fn filter_config_string(filter: &SceneSourceFilter, key: &str, fallback: &str) -> String {
    filter
        .config
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
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

fn software_input_frame_for_node(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
) -> SoftwareCompositorInputFrame {
    let input_frame = if node.source_kind == SceneSourceKind::ImageMedia {
        image_media_input_frame_for_node(node, clock)
    } else if node.source_kind == SceneSourceKind::BrowserOverlay {
        browser_overlay_input_frame_for_node(node, clock)
    } else if node.source_kind == SceneSourceKind::Text {
        text_input_frame_for_node(node)
    } else if matches!(
        node.source_kind,
        SceneSourceKind::Display | SceneSourceKind::Window | SceneSourceKind::Camera
    ) {
        capture_input_frame_for_node(node, clock)
    } else {
        placeholder_input_frame_for_node(
            node,
            node.status.clone(),
            node.status_detail.clone(),
            None,
            None,
            None,
            None,
        )
    };

    apply_software_filters(node, input_frame)
}

fn capture_input_frame_for_node(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
) -> SoftwareCompositorInputFrame {
    let Some(capture_source_id) = capture_source_id_for_node(node) else {
        return capture_placeholder_input_frame_for_node(
            node,
            SoftwareCompositorCaptureStatus::NoSource,
            "No capture source has been assigned.".to_string(),
            None,
            clock,
            CompositorNodeStatus::Placeholder,
        );
    };

    if node.status != CompositorNodeStatus::Ready {
        let status = if node.status == CompositorNodeStatus::PermissionRequired {
            SoftwareCompositorCaptureStatus::PermissionRequired
        } else {
            SoftwareCompositorCaptureStatus::CaptureFailed
        };
        return capture_placeholder_input_frame_for_node(
            node,
            status,
            node.status_detail.clone(),
            Some(capture_source_id),
            clock,
            node.status.clone(),
        );
    }

    capture_input_frame_for_node_with_provider(node, clock, platform_capture_frame_for_node)
}

fn capture_input_frame_for_node_with_provider(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
    provider: fn(
        &CompositorNode,
        &CompositorFrameClock,
    ) -> Result<DecodedCaptureFrame, CaptureFrameError>,
) -> SoftwareCompositorInputFrame {
    let capture_source_id = capture_source_id_for_node(node);
    match provider(node, clock) {
        Ok(decoded) => decoded_capture_input_frame(node, decoded),
        Err(error) => capture_placeholder_input_frame_for_node(
            node,
            error.status,
            error.detail,
            capture_source_id,
            clock,
            error.node_status,
        ),
    }
}

fn decoded_capture_input_frame(
    node: &CompositorNode,
    decoded: DecodedCaptureFrame,
) -> SoftwareCompositorInputFrame {
    let checksum = checksum_pixels(&decoded.pixels);
    let metadata = capture_metadata(
        node,
        CaptureMetadataInput {
            status: SoftwareCompositorCaptureStatus::Rendered,
            status_detail: decoded.status_detail,
            capture_source_id: decoded.capture_source_id,
            width: Some(decoded.width),
            height: Some(decoded.height),
            frame_index: decoded.frame_index,
            checksum: Some(checksum),
            capture_duration_ms: Some(decoded.capture_duration_ms),
            latency_ms: Some(decoded.capture_duration_ms as f64),
            dropped_frames: 0,
            provider_name: decoded.provider_name,
        },
    );

    SoftwareCompositorInputFrame {
        source_id: node.source_id.clone(),
        source_kind: node.source_kind.clone(),
        width: decoded.width,
        height: decoded.height,
        frame_format: CompositorFrameFormat::Rgba8,
        status: CompositorNodeStatus::Ready,
        status_detail: metadata.status_detail.clone(),
        asset: None,
        text: None,
        browser: None,
        capture: Some(metadata),
        filters: Vec::new(),
        checksum,
        pixels: decoded.pixels,
    }
}

fn capture_placeholder_input_frame_for_node(
    node: &CompositorNode,
    status: SoftwareCompositorCaptureStatus,
    detail: String,
    capture_source_id: Option<String>,
    clock: &CompositorFrameClock,
    node_status: CompositorNodeStatus,
) -> SoftwareCompositorInputFrame {
    let node_status = if status == SoftwareCompositorCaptureStatus::PermissionRequired {
        CompositorNodeStatus::PermissionRequired
    } else {
        node_status
    };
    let metadata = capture_metadata(
        node,
        CaptureMetadataInput {
            status,
            status_detail: detail.clone(),
            capture_source_id,
            width: None,
            height: None,
            frame_index: clock.frame_index,
            checksum: None,
            capture_duration_ms: None,
            latency_ms: None,
            dropped_frames: 0,
            provider_name: platform_capture_provider_name().to_string(),
        },
    );
    placeholder_input_frame_for_node(node, node_status, detail, None, None, None, Some(metadata))
}

struct CaptureMetadataInput {
    status: SoftwareCompositorCaptureStatus,
    status_detail: String,
    capture_source_id: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    frame_index: u64,
    checksum: Option<u64>,
    capture_duration_ms: Option<u64>,
    latency_ms: Option<f64>,
    dropped_frames: u64,
    provider_name: String,
}

fn capture_metadata(
    node: &CompositorNode,
    input: CaptureMetadataInput,
) -> SoftwareCompositorCaptureMetadata {
    SoftwareCompositorCaptureMetadata {
        status: input.status,
        status_detail: input.status_detail,
        capture_source_id: input.capture_source_id,
        capture_kind: capture_kind_for_node(node),
        width: input.width,
        height: input.height,
        frame_index: input.frame_index,
        checksum: input.checksum,
        capture_duration_ms: input.capture_duration_ms,
        latency_ms: input.latency_ms,
        dropped_frames: input.dropped_frames,
        provider_name: input.provider_name,
    }
}

#[derive(Clone, Debug)]
struct DecodedCaptureFrame {
    capture_source_id: Option<String>,
    width: u32,
    height: u32,
    frame_index: u64,
    capture_duration_ms: u64,
    provider_name: String,
    status_detail: String,
    pixels: Vec<u8>,
}

#[derive(Clone, Debug)]
struct CaptureFrameError {
    status: SoftwareCompositorCaptureStatus,
    node_status: CompositorNodeStatus,
    detail: String,
}

fn capture_source_id_for_node(node: &CompositorNode) -> Option<String> {
    match node.source_kind {
        SceneSourceKind::Camera => config_value_string(&node.config, "device_id"),
        SceneSourceKind::Display => config_value_string(&node.config, "display_id"),
        SceneSourceKind::Window => config_value_string(&node.config, "window_id"),
        _ => None,
    }
}

fn capture_kind_for_node(node: &CompositorNode) -> CaptureSourceKind {
    match node.source_kind {
        SceneSourceKind::Camera => CaptureSourceKind::Camera,
        SceneSourceKind::Window => CaptureSourceKind::Window,
        _ => CaptureSourceKind::Display,
    }
}

fn platform_capture_provider_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos-capture"
    } else {
        "unsupported-platform"
    }
}

#[cfg(target_os = "macos")]
fn platform_capture_frame_for_node(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
) -> Result<DecodedCaptureFrame, CaptureFrameError> {
    if node.source_kind == SceneSourceKind::Camera {
        macos_avfoundation_camera_frame_for_node(node, clock)
    } else {
        macos_screencapture_frame_for_node(node, clock)
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_capture_frame_for_node(
    node: &CompositorNode,
    _clock: &CompositorFrameClock,
) -> Result<DecodedCaptureFrame, CaptureFrameError> {
    Err(CaptureFrameError {
        status: SoftwareCompositorCaptureStatus::UnsupportedPlatform,
        node_status: CompositorNodeStatus::Placeholder,
        detail: format!(
            "{} capture preview is not implemented on this platform yet.",
            match capture_kind_for_node(node) {
                CaptureSourceKind::Display => "Display",
                CaptureSourceKind::Window => "Window",
                CaptureSourceKind::Camera => "Camera",
                CaptureSourceKind::Microphone => "Microphone",
                CaptureSourceKind::SystemAudio => "System audio",
            }
        ),
    })
}

#[cfg(target_os = "macos")]
fn macos_screencapture_frame_for_node(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
) -> Result<DecodedCaptureFrame, CaptureFrameError> {
    let capture_source_id = capture_source_id_for_node(node);
    let Some(capture_id) = capture_source_id.clone() else {
        return Err(CaptureFrameError {
            status: SoftwareCompositorCaptureStatus::NoSource,
            node_status: CompositorNodeStatus::Placeholder,
            detail: "No capture source has been assigned.".to_string(),
        });
    };

    let output_path = env::temp_dir().join(format!(
        "vaexcore-capture-{}-{}.png",
        std::process::id(),
        Uuid::new_v4()
    ));
    let mut args = vec!["-x".to_string(), "-t".to_string(), "png".to_string()];
    if config_bool(&node.config, "capture_cursor").unwrap_or(false) {
        args.push("-C".to_string());
    }
    match node.source_kind {
        SceneSourceKind::Display => {
            if let Some(display_id) = capture_id.strip_prefix("display:") {
                if display_id != "main" {
                    args.push("-D".to_string());
                    args.push(display_id.to_string());
                }
            }
        }
        SceneSourceKind::Window => {
            let Some(window_id) = capture_id.strip_prefix("window:") else {
                let _ = fs::remove_file(&output_path);
                return Err(CaptureFrameError {
                    status: SoftwareCompositorCaptureStatus::UnsupportedSource,
                    node_status: CompositorNodeStatus::Placeholder,
                    detail: format!("Unsupported macOS window capture source id \"{capture_id}\"."),
                });
            };
            let Ok(window_number) = window_id.parse::<u32>() else {
                let _ = fs::remove_file(&output_path);
                return Err(CaptureFrameError {
                    status: SoftwareCompositorCaptureStatus::UnsupportedSource,
                    node_status: CompositorNodeStatus::Placeholder,
                    detail: format!("Unsupported macOS window capture source id \"{capture_id}\"."),
                });
            };
            args.push("-l".to_string());
            args.push(window_number.to_string());
        }
        _ => {
            let _ = fs::remove_file(&output_path);
            return Err(CaptureFrameError {
                status: SoftwareCompositorCaptureStatus::UnsupportedSource,
                node_status: CompositorNodeStatus::Placeholder,
                detail: "Only display and window sources can use macOS capture preview."
                    .to_string(),
            });
        }
    }
    args.push(output_path.display().to_string());

    let started_at = Instant::now();
    let output = Command::new("screencapture").args(&args).output();
    let capture_duration_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let output = output.map_err(|error| CaptureFrameError {
        status: SoftwareCompositorCaptureStatus::CaptureFailed,
        node_status: CompositorNodeStatus::Placeholder,
        detail: format!("macOS screencapture could not be started: {error}"),
    })?;

    if !output.status.success() {
        let detail = macos_screencapture_error_detail(&output.stderr);
        let _ = fs::remove_file(&output_path);
        return Err(CaptureFrameError {
            status: macos_screencapture_error_status(&detail),
            node_status: if detail.to_ascii_lowercase().contains("permission") {
                CompositorNodeStatus::PermissionRequired
            } else {
                CompositorNodeStatus::Placeholder
            },
            detail,
        });
    }

    let bytes = fs::read(&output_path).map_err(|error| CaptureFrameError {
        status: SoftwareCompositorCaptureStatus::CaptureFailed,
        node_status: CompositorNodeStatus::Placeholder,
        detail: format!("macOS capture image could not be read: {error}"),
    });
    let _ = fs::remove_file(&output_path);
    let bytes = bytes?;
    let image = image::load_from_memory(&bytes).map_err(|error| CaptureFrameError {
        status: SoftwareCompositorCaptureStatus::CaptureFailed,
        node_status: CompositorNodeStatus::Placeholder,
        detail: format!("macOS capture image could not be decoded: {error}"),
    })?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(DecodedCaptureFrame {
        capture_source_id,
        width,
        height,
        frame_index: clock.frame_index,
        capture_duration_ms,
        provider_name: platform_capture_provider_name().to_string(),
        status_detail: format!(
            "Captured macOS {} frame from {}.",
            match node.source_kind {
                SceneSourceKind::Window => "window",
                _ => "display",
            },
            capture_id
        ),
        pixels: rgba.into_raw(),
    })
}

#[cfg(target_os = "macos")]
fn macos_avfoundation_camera_frame_for_node(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
) -> Result<DecodedCaptureFrame, CaptureFrameError> {
    let capture_source_id = capture_source_id_for_node(node);
    let Some(capture_id) = capture_source_id.clone() else {
        return Err(CaptureFrameError {
            status: SoftwareCompositorCaptureStatus::NoSource,
            node_status: CompositorNodeStatus::Placeholder,
            detail: "No camera source has been assigned.".to_string(),
        });
    };
    let Some(ffmpeg_path) = find_ffmpeg_binary() else {
        return Err(CaptureFrameError {
            status: SoftwareCompositorCaptureStatus::DecoderUnavailable,
            node_status: CompositorNodeStatus::Placeholder,
            detail: "FFmpeg is required for camera preview V1 and was not found.".to_string(),
        });
    };
    let Some(camera_index) = macos_camera_index_from_source_id(&capture_id) else {
        return Err(CaptureFrameError {
            status: SoftwareCompositorCaptureStatus::UnsupportedSource,
            node_status: CompositorNodeStatus::Placeholder,
            detail: format!("Unsupported macOS camera source id \"{capture_id}\"."),
        });
    };
    let (width, height) = camera_capture_resolution(node);
    let framerate = config_value_number(&node.config, "framerate")
        .unwrap_or(30.0)
        .round()
        .clamp(1.0, 120.0) as u32;
    let video_size = format!("{width}x{height}");
    let input_name = format!("{camera_index}:none");
    let framerate_arg = framerate.to_string();
    let started_at = Instant::now();
    let output = Command::new(&ffmpeg_path)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-f",
            "avfoundation",
            "-framerate",
            &framerate_arg,
            "-video_size",
            &video_size,
            "-i",
            &input_name,
            "-frames:v",
            "1",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "pipe:1",
        ])
        .output()
        .map_err(|error| CaptureFrameError {
            status: SoftwareCompositorCaptureStatus::CaptureFailed,
            node_status: CompositorNodeStatus::Placeholder,
            detail: format!("FFmpeg camera preview could not be started: {error}"),
        })?;
    let capture_duration_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    if !output.status.success() {
        let detail = ffmpeg_capture_error_detail(&output.stderr);
        return Err(CaptureFrameError {
            status: ffmpeg_capture_error_status(&detail),
            node_status: if detail.to_ascii_lowercase().contains("permission") {
                CompositorNodeStatus::PermissionRequired
            } else {
                CompositorNodeStatus::Placeholder
            },
            detail,
        });
    }
    if output.stdout.is_empty() {
        return Err(CaptureFrameError {
            status: SoftwareCompositorCaptureStatus::CaptureFailed,
            node_status: CompositorNodeStatus::Placeholder,
            detail: "FFmpeg returned an empty camera preview frame.".to_string(),
        });
    }
    let image = image::load_from_memory(&output.stdout).map_err(|error| CaptureFrameError {
        status: SoftwareCompositorCaptureStatus::CaptureFailed,
        node_status: CompositorNodeStatus::Placeholder,
        detail: format!("FFmpeg camera preview image could not be decoded: {error}"),
    })?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(DecodedCaptureFrame {
        capture_source_id,
        width,
        height,
        frame_index: clock.frame_index,
        capture_duration_ms,
        provider_name: "macos-avfoundation-ffmpeg".to_string(),
        status_detail: format!("Captured macOS camera frame from {capture_id}."),
        pixels: rgba.into_raw(),
    })
}

#[cfg(target_os = "macos")]
fn macos_camera_index_from_source_id(capture_id: &str) -> Option<u32> {
    if capture_id == "camera:default" {
        return Some(0);
    }
    capture_id
        .strip_prefix("camera:")
        .and_then(|value| value.parse::<u32>().ok())
}

#[cfg(target_os = "macos")]
fn camera_capture_resolution(node: &CompositorNode) -> (u32, u32) {
    let resolution = node.config.get("resolution");
    let width = resolution
        .and_then(|value| value.get("width"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or_else(|| node.transform.size.width.round().max(1.0) as u32)
        .clamp(1, 3840);
    let height = resolution
        .and_then(|value| value.get("height"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or_else(|| node.transform.size.height.round().max(1.0) as u32)
        .clamp(1, 2160);

    (width, height)
}

#[cfg(target_os = "macos")]
fn ffmpeg_capture_error_detail(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if stderr.is_empty() {
        "FFmpeg camera preview failed. Check Camera permission and source availability.".to_string()
    } else {
        format!("FFmpeg camera preview failed: {stderr}")
    }
}

#[cfg(target_os = "macos")]
fn ffmpeg_capture_error_status(detail: &str) -> SoftwareCompositorCaptureStatus {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("not authorized")
        || lower.contains("permission")
        || lower.contains("privacy")
        || lower.contains("denied")
    {
        SoftwareCompositorCaptureStatus::PermissionRequired
    } else {
        SoftwareCompositorCaptureStatus::CaptureFailed
    }
}

#[cfg(target_os = "macos")]
fn macos_screencapture_error_detail(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if stderr.is_empty() {
        "macOS screencapture failed. Check Screen Recording permission and source availability."
            .to_string()
    } else {
        format!("macOS screencapture failed: {stderr}")
    }
}

#[cfg(target_os = "macos")]
fn macos_screencapture_error_status(detail: &str) -> SoftwareCompositorCaptureStatus {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("not authorized")
        || lower.contains("permission")
        || lower.contains("privacy")
        || lower.contains("denied")
    {
        SoftwareCompositorCaptureStatus::PermissionRequired
    } else {
        SoftwareCompositorCaptureStatus::CaptureFailed
    }
}

fn image_media_input_frame_for_node(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
) -> SoftwareCompositorInputFrame {
    let asset_uri = config_value_string(&node.config, "asset_uri").unwrap_or_default();
    if asset_uri.trim().is_empty() {
        let metadata = asset_metadata(
            asset_uri,
            SoftwareCompositorAssetStatus::NoAsset,
            "No local media asset has been selected.".to_string(),
            None,
        );
        return placeholder_input_frame_for_node(
            node,
            CompositorNodeStatus::Placeholder,
            "No local media asset has been selected.".to_string(),
            Some(metadata),
            None,
            None,
            None,
        );
    }

    let media_type =
        config_value_string(&node.config, "media_type").unwrap_or_else(|| "image".into());
    if media_type != "image" {
        return match decode_video_asset(&asset_uri, clock) {
            Ok(decoded) => decoded_video_input_frame(node, asset_uri, decoded),
            Err(metadata) => {
                let metadata = *metadata;
                placeholder_input_frame_for_node(
                    node,
                    CompositorNodeStatus::Placeholder,
                    metadata.status_detail.clone(),
                    Some(metadata),
                    None,
                    None,
                    None,
                )
            }
        };
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
                None,
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
        sampled_frame_time_ms: None,
        sample_index: None,
        decoder_name: None,
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
        browser: None,
        capture: None,
        filters: Vec::new(),
        checksum: decoded.image.checksum,
        pixels: decoded.image.pixels,
    }
}

fn decoded_video_input_frame(
    node: &CompositorNode,
    asset_uri: String,
    decoded: DecodedVideoAsset,
) -> SoftwareCompositorInputFrame {
    let metadata = SoftwareCompositorAssetMetadata {
        uri: asset_uri,
        status: SoftwareCompositorAssetStatus::Decoded,
        status_detail: format!(
            "Decoded {} video preview frame {}x{} at {:.2}s using {}.",
            decoded.image.format,
            decoded.image.width,
            decoded.image.height,
            decoded.sample_time_ms as f64 / 1000.0,
            decoded.decoder_name
        ),
        format: Some(format!("video:{}", decoded.image.format)),
        width: Some(decoded.image.width),
        height: Some(decoded.image.height),
        checksum: Some(decoded.image.checksum),
        modified_unix_ms: Some(decoded.modified_unix_ms),
        cache_hit: decoded.cache_hit,
        sampled_frame_time_ms: Some(decoded.sample_time_ms),
        sample_index: Some(decoded.sample_index),
        decoder_name: Some(decoded.decoder_name),
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
        browser: None,
        capture: None,
        filters: Vec::new(),
        checksum: decoded.image.checksum,
        pixels: decoded.image.pixels,
    }
}

fn browser_overlay_input_frame_for_node(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
) -> SoftwareCompositorInputFrame {
    browser_overlay_input_frame_for_node_with_browser(node, clock, find_browser_binary())
}

fn browser_overlay_input_frame_for_node_with_browser(
    node: &CompositorNode,
    clock: &CompositorFrameClock,
    browser: Option<BrowserBinary>,
) -> SoftwareCompositorInputFrame {
    let viewport = browser_viewport_for_node(node);
    let url = config_value_string(&node.config, "url").unwrap_or_default();
    if url.trim().is_empty() {
        let metadata = browser_metadata(
            SoftwareCompositorBrowserStatus::NoUrl,
            "No browser overlay URL has been configured.".to_string(),
            None,
            viewport,
            None,
        );
        return placeholder_input_frame_for_node(
            node,
            CompositorNodeStatus::Placeholder,
            metadata.status_detail.clone(),
            None,
            None,
            Some(metadata),
            None,
        );
    }

    if !is_supported_browser_overlay_url(&url) {
        let metadata = browser_metadata(
            SoftwareCompositorBrowserStatus::UnsupportedUrl,
            "Unsupported browser overlay URL. Supported URLs are http, https, and file."
                .to_string(),
            Some(url),
            viewport,
            None,
        );
        return placeholder_input_frame_for_node(
            node,
            CompositorNodeStatus::Placeholder,
            metadata.status_detail.clone(),
            None,
            None,
            Some(metadata),
            None,
        );
    }

    let Some(browser) = browser else {
        let metadata = browser_metadata(
            SoftwareCompositorBrowserStatus::BrowserUnavailable,
            "Chrome, Chromium, or Edge is not available; browser overlay preview is using a placeholder."
                .to_string(),
            Some(url),
            viewport,
            None,
        );
        return placeholder_input_frame_for_node(
            node,
            CompositorNodeStatus::Placeholder,
            metadata.status_detail.clone(),
            None,
            None,
            Some(metadata),
            None,
        );
    };

    let custom_css = config_value_string(&node.config, "custom_css").unwrap_or_default();
    let sample_time_ms = quantized_browser_sample_time_ms(clock);
    let request = BrowserSnapshotRequest {
        url: url.clone(),
        viewport_width: viewport.0,
        viewport_height: viewport.1,
        custom_css,
        sample_time_ms,
        sample_index: sample_time_ms / BROWSER_SAMPLE_INTERVAL_MS,
    };

    match browser_overlay_snapshot(&browser, &request) {
        Ok(snapshot) => browser_snapshot_input_frame(node, &request, snapshot),
        Err(error) => {
            let metadata = browser_metadata(
                error.status,
                error.detail,
                Some(url),
                viewport,
                Some((
                    browser,
                    request,
                    error.custom_css_applied,
                    error.custom_css_detail,
                )),
            );
            placeholder_input_frame_for_node(
                node,
                CompositorNodeStatus::Placeholder,
                metadata.status_detail.clone(),
                None,
                None,
                Some(metadata),
                None,
            )
        }
    }
}

fn browser_snapshot_input_frame(
    node: &CompositorNode,
    request: &BrowserSnapshotRequest,
    snapshot: BrowserSnapshot,
) -> SoftwareCompositorInputFrame {
    let metadata = SoftwareCompositorBrowserMetadata {
        status: SoftwareCompositorBrowserStatus::Rendered,
        status_detail: format!(
            "Rendered browser overlay {}x{} at {:.2}s using {}.",
            snapshot.width,
            snapshot.height,
            snapshot.sample_time_ms as f64 / 1000.0,
            snapshot.browser_name
        ),
        url: Some(request.url.clone()),
        viewport_width: request.viewport_width,
        viewport_height: request.viewport_height,
        custom_css_present: !request.custom_css.trim().is_empty(),
        custom_css_applied: snapshot.custom_css_applied,
        custom_css_detail: snapshot.custom_css_detail,
        browser_name: Some(snapshot.browser_name),
        browser_path: Some(snapshot.browser_path),
        sampled_frame_time_ms: Some(snapshot.sample_time_ms),
        sample_index: Some(snapshot.sample_index),
        checksum: Some(snapshot.checksum),
        capture_duration_ms: Some(snapshot.capture_duration_ms),
        cache_hit: snapshot.cache_hit,
    };

    SoftwareCompositorInputFrame {
        source_id: node.source_id.clone(),
        source_kind: node.source_kind.clone(),
        width: snapshot.width,
        height: snapshot.height,
        frame_format: CompositorFrameFormat::Rgba8,
        status: CompositorNodeStatus::Ready,
        status_detail: metadata.status_detail.clone(),
        asset: None,
        text: None,
        browser: Some(metadata),
        capture: None,
        filters: Vec::new(),
        checksum: snapshot.checksum,
        pixels: snapshot.pixels,
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
            None,
            None,
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
                None,
                None,
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
        browser: None,
        capture: None,
        filters: Vec::new(),
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
    browser: Option<SoftwareCompositorBrowserMetadata>,
    capture: Option<SoftwareCompositorCaptureMetadata>,
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
        browser,
        capture,
        filters: Vec::new(),
        checksum,
        pixels,
    }
}

fn apply_software_filters(
    node: &CompositorNode,
    mut frame: SoftwareCompositorInputFrame,
) -> SoftwareCompositorInputFrame {
    let filters = sorted_source_filters(&node.filters);
    if filters.is_empty() {
        return frame;
    }

    let mut metadata = Vec::with_capacity(filters.len());
    for filter in filters {
        let result = apply_software_filter(&mut frame, filter);
        metadata.push(result);
    }
    frame.checksum = checksum_pixels(&frame.pixels);
    frame.filters = metadata;
    frame
}

fn sorted_source_filters(filters: &[SceneSourceFilter]) -> Vec<&SceneSourceFilter> {
    let mut sorted = filters.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.id.cmp(&right.id))
    });
    sorted
}

fn apply_software_filter(
    frame: &mut SoftwareCompositorInputFrame,
    filter: &SceneSourceFilter,
) -> SoftwareCompositorFilterMetadata {
    if !filter.enabled {
        return filter_metadata(
            filter,
            SoftwareCompositorFilterStatus::Skipped,
            "Filter is disabled.".to_string(),
            Some(frame.checksum),
        );
    }

    let result = match filter.kind {
        SceneSourceFilterKind::ColorCorrection => {
            apply_color_correction_filter(frame, filter);
            Ok("Applied color correction.".to_string())
        }
        SceneSourceFilterKind::ChromaKey => apply_chroma_key_filter(frame, filter),
        SceneSourceFilterKind::CropPad => {
            apply_crop_pad_filter(frame, filter);
            Ok("Applied crop/pad as source-frame alpha crop.".to_string())
        }
        SceneSourceFilterKind::Blur => apply_blur_filter(frame, filter),
        SceneSourceFilterKind::Sharpen => apply_sharpen_filter(frame, filter),
        SceneSourceFilterKind::MaskBlend => apply_mask_blend_filter(frame, filter),
        SceneSourceFilterKind::Lut => apply_lut_filter(frame, filter),
        SceneSourceFilterKind::AudioGain
        | SceneSourceFilterKind::NoiseGate
        | SceneSourceFilterKind::Compressor => Err((
            SoftwareCompositorFilterStatus::Deferred,
            deferred_filter_detail(&filter.kind).to_string(),
        )),
    };

    match result {
        Ok(detail) => {
            frame.checksum = checksum_pixels(&frame.pixels);
            filter_metadata(
                filter,
                SoftwareCompositorFilterStatus::Applied,
                detail,
                Some(frame.checksum),
            )
        }
        Err((status, detail)) => filter_metadata(filter, status, detail, Some(frame.checksum)),
    }
}

fn filter_metadata(
    filter: &SceneSourceFilter,
    status: SoftwareCompositorFilterStatus,
    status_detail: String,
    checksum: Option<u64>,
) -> SoftwareCompositorFilterMetadata {
    SoftwareCompositorFilterMetadata {
        id: filter.id.clone(),
        name: filter.name.clone(),
        kind: filter.kind.clone(),
        status,
        status_detail,
        order: filter.order,
        checksum,
    }
}

fn deferred_filter_detail(kind: &SceneSourceFilterKind) -> &'static str {
    match kind {
        SceneSourceFilterKind::MaskBlend => {
            "Mask/blend preview rendering is deferred until mask asset decoding is implemented."
        }
        SceneSourceFilterKind::Lut => {
            "LUT preview rendering is deferred until LUT file parsing is implemented."
        }
        SceneSourceFilterKind::AudioGain
        | SceneSourceFilterKind::NoiseGate
        | SceneSourceFilterKind::Compressor => {
            "Audio filter execution is deferred until the real audio mixer path is implemented."
        }
        _ => "Filter execution is deferred.",
    }
}

fn apply_color_correction_filter(
    frame: &mut SoftwareCompositorInputFrame,
    filter: &SceneSourceFilter,
) {
    let brightness = filter_config_number(filter, "brightness", 0.0).clamp(-1.0, 1.0);
    let contrast = filter_config_number(filter, "contrast", 1.0).clamp(0.0, 4.0);
    let saturation = filter_config_number(filter, "saturation", 1.0).clamp(0.0, 4.0);
    let gamma = filter_config_number(filter, "gamma", 1.0).clamp(0.01, 4.0);

    for pixel in frame.pixels.chunks_exact_mut(4) {
        if pixel[3] == 0 {
            continue;
        }
        let mut red = color_channel_to_unit(pixel[0]);
        let mut green = color_channel_to_unit(pixel[1]);
        let mut blue = color_channel_to_unit(pixel[2]);

        red = ((red - 0.5) * contrast + 0.5 + brightness).clamp(0.0, 1.0);
        green = ((green - 0.5) * contrast + 0.5 + brightness).clamp(0.0, 1.0);
        blue = ((blue - 0.5) * contrast + 0.5 + brightness).clamp(0.0, 1.0);

        let luma = red * 0.2126 + green * 0.7152 + blue * 0.0722;
        red = (luma + (red - luma) * saturation).clamp(0.0, 1.0);
        green = (luma + (green - luma) * saturation).clamp(0.0, 1.0);
        blue = (luma + (blue - luma) * saturation).clamp(0.0, 1.0);

        let inverse_gamma = 1.0 / gamma;
        pixel[0] = unit_to_color_channel(red.powf(inverse_gamma));
        pixel[1] = unit_to_color_channel(green.powf(inverse_gamma));
        pixel[2] = unit_to_color_channel(blue.powf(inverse_gamma));
    }
}

fn apply_chroma_key_filter(
    frame: &mut SoftwareCompositorInputFrame,
    filter: &SceneSourceFilter,
) -> Result<String, (SoftwareCompositorFilterStatus, String)> {
    let key_color_value = filter_config_string(filter, "key_color", "#00ff00");
    let Some(key_color) = parse_text_color(&key_color_value) else {
        return Err((
            SoftwareCompositorFilterStatus::Error,
            format!(
                "Chroma key was not applied because key color \"{key_color_value}\" is invalid."
            ),
        ));
    };
    let similarity = filter_config_number(filter, "similarity", 0.25).clamp(0.0, 1.0);
    let smoothness = filter_config_number(filter, "smoothness", 0.08).clamp(0.0, 1.0);
    let fade_end = (similarity + smoothness).clamp(similarity, 1.0);

    for pixel in frame.pixels.chunks_exact_mut(4) {
        if pixel[3] == 0 {
            continue;
        }
        let distance = color_distance_unit(
            [pixel[0], pixel[1], pixel[2]],
            [key_color[0], key_color[1], key_color[2]],
        );
        let alpha_factor = if distance <= similarity {
            0.0
        } else if smoothness <= f64::EPSILON || distance >= fade_end {
            1.0
        } else {
            ((distance - similarity) / smoothness).clamp(0.0, 1.0)
        };
        pixel[3] = (f64::from(pixel[3]) * alpha_factor).round() as u8;
        if pixel[3] == 0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
        }
    }

    Ok(format!(
        "Applied chroma key for {} with similarity {:.2} and smoothness {:.2}.",
        normalized_hex_color(key_color),
        similarity,
        smoothness
    ))
}

fn apply_crop_pad_filter(frame: &mut SoftwareCompositorInputFrame, filter: &SceneSourceFilter) {
    let top = filter_config_number(filter, "top", 0.0).max(0.0).round() as usize;
    let right = filter_config_number(filter, "right", 0.0).max(0.0).round() as usize;
    let bottom = filter_config_number(filter, "bottom", 0.0).max(0.0).round() as usize;
    let left = filter_config_number(filter, "left", 0.0).max(0.0).round() as usize;
    let width = frame.width as usize;
    let height = frame.height as usize;

    for y in 0..height {
        for x in 0..width {
            if y < top
                || y >= height.saturating_sub(bottom)
                || x < left
                || x >= width.saturating_sub(right)
            {
                let offset = (y * width + x) * 4;
                frame.pixels[offset] = 0;
                frame.pixels[offset + 1] = 0;
                frame.pixels[offset + 2] = 0;
                frame.pixels[offset + 3] = 0;
            }
        }
    }
}

fn apply_blur_filter(
    frame: &mut SoftwareCompositorInputFrame,
    filter: &SceneSourceFilter,
) -> Result<String, (SoftwareCompositorFilterStatus, String)> {
    let radius = filter_config_number(filter, "radius", 4.0)
        .max(0.0)
        .round()
        .min(32.0) as usize;
    if radius == 0 {
        return Ok("Blur radius is zero; pixels were unchanged.".to_string());
    }
    frame.pixels = box_blur_rgba(
        &frame.pixels,
        frame.width as usize,
        frame.height as usize,
        radius,
    );
    Ok(format!("Applied CPU box blur with radius {radius}."))
}

fn apply_sharpen_filter(
    frame: &mut SoftwareCompositorInputFrame,
    filter: &SceneSourceFilter,
) -> Result<String, (SoftwareCompositorFilterStatus, String)> {
    let amount = filter_config_number(filter, "amount", 0.35).clamp(0.0, 5.0);
    if amount <= f64::EPSILON {
        return Ok("Sharpen amount is zero; pixels were unchanged.".to_string());
    }
    let blurred = box_blur_rgba(
        &frame.pixels,
        frame.width as usize,
        frame.height as usize,
        1,
    );
    for (pixel, blurred_pixel) in frame
        .pixels
        .chunks_exact_mut(4)
        .zip(blurred.chunks_exact(4))
    {
        for channel in 0..3 {
            let original = f64::from(pixel[channel]);
            let low_pass = f64::from(blurred_pixel[channel]);
            pixel[channel] = (original + (original - low_pass) * amount)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
    Ok(format!("Applied CPU sharpen with amount {:.2}.", amount))
}

fn apply_mask_blend_filter(
    frame: &mut SoftwareCompositorInputFrame,
    filter: &SceneSourceFilter,
) -> Result<String, (SoftwareCompositorFilterStatus, String)> {
    let mask_uri = filter_config_string(filter, "mask_uri", "");
    if mask_uri.trim().is_empty() {
        return filter_error("No mask image has been selected.");
    }
    let blend_mode = filter_config_string(filter, "blend_mode", "normal");
    let decoded = decode_image_asset(&mask_uri).map_err(|metadata| {
        (
            SoftwareCompositorFilterStatus::Error,
            format!("Mask image could not be loaded: {}", metadata.status_detail),
        )
    })?;
    apply_mask_pixels(frame, &decoded.image, &blend_mode)?;
    Ok(format!(
        "Applied {} mask/blend image {}x{} ({}).",
        blend_mode,
        decoded.image.width,
        decoded.image.height,
        if decoded.cache_hit {
            "cache hit"
        } else {
            "fresh decode"
        }
    ))
}

fn apply_mask_pixels(
    frame: &mut SoftwareCompositorInputFrame,
    mask: &CachedDecodedImage,
    blend_mode: &str,
) -> Result<(), (SoftwareCompositorFilterStatus, String)> {
    let width = frame.width.max(1) as usize;
    let height = frame.height.max(1) as usize;
    let mask_width = mask.width.max(1) as usize;
    let mask_height = mask.height.max(1) as usize;

    for y in 0..height {
        let mask_y = ((y * mask_height) / height).min(mask_height - 1);
        for x in 0..width {
            let mask_x = ((x * mask_width) / width).min(mask_width - 1);
            let mask_offset = (mask_y * mask_width + mask_x) * 4;
            let mask_color = [
                mask.pixels[mask_offset],
                mask.pixels[mask_offset + 1],
                mask.pixels[mask_offset + 2],
                mask.pixels[mask_offset + 3],
            ];
            let mask_alpha = color_channel_to_unit(mask_color[3]);
            let mask_luma = color_luma_unit([mask_color[0], mask_color[1], mask_color[2]]);
            let alpha_factor = (mask_luma * mask_alpha).clamp(0.0, 1.0);

            let offset = (y * width + x) * 4;
            let source = [
                frame.pixels[offset],
                frame.pixels[offset + 1],
                frame.pixels[offset + 2],
            ];
            let blended = mask_blend_rgb(
                source,
                [mask_color[0], mask_color[1], mask_color[2]],
                blend_mode,
            )?;
            let blend_amount = if blend_mode == "alpha" {
                0.0
            } else {
                mask_alpha
            };

            for channel in 0..3 {
                frame.pixels[offset + channel] =
                    mix_channel(source[channel], blended[channel], blend_amount);
            }
            frame.pixels[offset + 3] =
                (f64::from(frame.pixels[offset + 3]) * alpha_factor).round() as u8;
        }
    }
    Ok(())
}

fn mask_blend_rgb(
    source: [u8; 3],
    mask: [u8; 3],
    blend_mode: &str,
) -> Result<[u8; 3], (SoftwareCompositorFilterStatus, String)> {
    let Some(red) = mask_blend_channel(source[0], mask[0], blend_mode) else {
        return filter_error(format!("Unsupported mask blend mode \"{blend_mode}\"."));
    };
    let Some(green) = mask_blend_channel(source[1], mask[1], blend_mode) else {
        return filter_error(format!("Unsupported mask blend mode \"{blend_mode}\"."));
    };
    let Some(blue) = mask_blend_channel(source[2], mask[2], blend_mode) else {
        return filter_error(format!("Unsupported mask blend mode \"{blend_mode}\"."));
    };

    Ok([
        unit_to_color_channel(red),
        unit_to_color_channel(green),
        unit_to_color_channel(blue),
    ])
}

fn mask_blend_channel(source: u8, mask: u8, blend_mode: &str) -> Option<f64> {
    let source = color_channel_to_unit(source);
    let mask = color_channel_to_unit(mask);
    Some(match blend_mode {
        "normal" | "alpha" => mask,
        "multiply" => source * mask,
        "screen" => 1.0 - (1.0 - source) * (1.0 - mask),
        "overlay" if source < 0.5 => 2.0 * source * mask,
        "overlay" => 1.0 - 2.0 * (1.0 - source) * (1.0 - mask),
        _ => return None,
    })
}

fn apply_lut_filter(
    frame: &mut SoftwareCompositorInputFrame,
    filter: &SceneSourceFilter,
) -> Result<String, (SoftwareCompositorFilterStatus, String)> {
    let lut_uri = filter_config_string(filter, "lut_uri", "");
    if lut_uri.trim().is_empty() {
        return filter_error("No LUT file has been selected.");
    }
    let strength = filter_config_number(filter, "strength", 1.0).clamp(0.0, 1.0);
    let decoded = decode_cube_lut(&lut_uri)?;

    if strength > f64::EPSILON {
        for pixel in frame.pixels.chunks_exact_mut(4) {
            if pixel[3] == 0 {
                continue;
            }
            let original = [
                color_channel_to_unit(pixel[0]),
                color_channel_to_unit(pixel[1]),
                color_channel_to_unit(pixel[2]),
            ];
            let mapped = sample_cube_lut(&decoded.lut, original);
            for channel in 0..3 {
                pixel[channel] = unit_to_color_channel(
                    original[channel] + (mapped[channel] - original[channel]) * strength,
                );
            }
        }
    }

    Ok(format!(
        "Applied {}x{}x{} .cube LUT at {:.2} strength ({}; checksum {:x}; modified {}).",
        decoded.lut.size,
        decoded.lut.size,
        decoded.lut.size,
        strength,
        if decoded.cache_hit {
            "cache hit"
        } else {
            "fresh parse"
        },
        decoded.lut.checksum,
        decoded.modified_unix_ms
    ))
}

fn filter_error<T>(
    message: impl Into<String>,
) -> Result<T, (SoftwareCompositorFilterStatus, String)> {
    Err((SoftwareCompositorFilterStatus::Error, message.into()))
}

fn decode_cube_lut(
    lut_uri: &str,
) -> Result<DecodedCubeLut, (SoftwareCompositorFilterStatus, String)> {
    let Some(path) = asset_uri_path(lut_uri) else {
        return filter_error("No LUT file has been selected.");
    };
    let normalized_path = normalized_asset_path(&path);
    if path
        .extension()
        .map(|extension| extension.to_string_lossy().eq_ignore_ascii_case("cube"))
        != Some(true)
    {
        return filter_error(format!(
            "Unsupported LUT extension for {normalized_path}. Supported LUT files use .cube."
        ));
    }
    let metadata = fs::metadata(&path).map_err(|_| {
        (
            SoftwareCompositorFilterStatus::Error,
            format!("LUT file does not exist: {normalized_path}"),
        )
    })?;
    if !metadata.is_file() {
        return filter_error(format!("LUT path is not a file: {normalized_path}"));
    }
    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(system_time_unix_ms)
        .unwrap_or(0);
    let key = LutAssetCacheKey {
        path: normalized_path.clone(),
        modified_unix_ms,
    };
    if let Some(lut) = cached_lut_asset(&key) {
        return Ok(DecodedCubeLut {
            modified_unix_ms,
            cache_hit: true,
            lut,
        });
    }

    let contents = fs::read_to_string(&path).map_err(|error| {
        (
            SoftwareCompositorFilterStatus::Error,
            format!("LUT file could not be read: {error}"),
        )
    })?;
    let lut = parse_cube_lut(&contents).map_err(|error| {
        (
            SoftwareCompositorFilterStatus::Error,
            format!("LUT file could not be parsed: {error}"),
        )
    })?;
    store_cached_lut_asset(key, lut.clone());
    Ok(DecodedCubeLut {
        modified_unix_ms,
        cache_hit: false,
        lut,
    })
}

fn parse_cube_lut(contents: &str) -> Result<CachedCubeLut, String> {
    let mut size = None;
    let mut domain_min = [0.0, 0.0, 0.0];
    let mut domain_max = [1.0, 1.0, 1.0];
    let mut values = Vec::new();

    for raw_line in contents.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let parts = line.split_whitespace().collect::<Vec<_>>();
        let keyword = parts[0].to_ascii_uppercase();
        match keyword.as_str() {
            "TITLE" => continue,
            "LUT_1D_SIZE" => return Err("1D LUT files are not supported in this pass.".to_string()),
            "LUT_3D_SIZE" => {
                let parsed_size = parts
                    .get(1)
                    .ok_or_else(|| "LUT_3D_SIZE requires a size.".to_string())?
                    .parse::<usize>()
                    .map_err(|_| "LUT_3D_SIZE must be an integer.".to_string())?;
                if !(2..=128).contains(&parsed_size) {
                    return Err("LUT_3D_SIZE must be between 2 and 128.".to_string());
                }
                size = Some(parsed_size);
            }
            "DOMAIN_MIN" => {
                domain_min = parse_cube_triplet(&parts, "DOMAIN_MIN")?;
            }
            "DOMAIN_MAX" => {
                domain_max = parse_cube_triplet(&parts, "DOMAIN_MAX")?;
            }
            "LUT_3D_INPUT_RANGE" => continue,
            _ => {
                if parts.len() < 3 || parts[0].parse::<f64>().is_err() {
                    continue;
                }
                values.push([
                    parts[0]
                        .parse::<f64>()
                        .map_err(|_| "LUT red channel must be numeric.".to_string())?
                        .clamp(0.0, 1.0),
                    parts[1]
                        .parse::<f64>()
                        .map_err(|_| "LUT green channel must be numeric.".to_string())?
                        .clamp(0.0, 1.0),
                    parts[2]
                        .parse::<f64>()
                        .map_err(|_| "LUT blue channel must be numeric.".to_string())?
                        .clamp(0.0, 1.0),
                ]);
            }
        }
    }

    let size = size.ok_or_else(|| "LUT_3D_SIZE is required.".to_string())?;
    let expected = size
        .checked_mul(size)
        .and_then(|value| value.checked_mul(size))
        .ok_or_else(|| "LUT size is too large.".to_string())?;
    if values.len() != expected {
        return Err(format!(
            "LUT_3D_SIZE {size} requires {expected} RGB rows; found {}.",
            values.len()
        ));
    }
    for index in 0..3 {
        if domain_max[index] <= domain_min[index] {
            return Err("DOMAIN_MAX values must be greater than DOMAIN_MIN values.".to_string());
        }
    }
    let checksum = checksum_lut_values(size, domain_min, domain_max, &values);
    Ok(CachedCubeLut {
        size,
        domain_min,
        domain_max,
        values,
        checksum,
    })
}

fn parse_cube_triplet(parts: &[&str], label: &str) -> Result<[f64; 3], String> {
    if parts.len() < 4 {
        return Err(format!("{label} requires three numeric values."));
    }
    Ok([
        parts[1]
            .parse::<f64>()
            .map_err(|_| format!("{label} red value must be numeric."))?,
        parts[2]
            .parse::<f64>()
            .map_err(|_| format!("{label} green value must be numeric."))?,
        parts[3]
            .parse::<f64>()
            .map_err(|_| format!("{label} blue value must be numeric."))?,
    ])
}

fn sample_cube_lut(lut: &CachedCubeLut, rgb: [f64; 3]) -> [f64; 3] {
    let scaled = [0, 1, 2].map(|index| {
        let range = lut.domain_max[index] - lut.domain_min[index];
        ((rgb[index] - lut.domain_min[index]) / range).clamp(0.0, 1.0) * (lut.size - 1) as f64
    });
    let low = scaled.map(|value| value.floor() as usize);
    let high = low.map(|value| (value + 1).min(lut.size - 1));
    let t = [
        scaled[0] - low[0] as f64,
        scaled[1] - low[1] as f64,
        scaled[2] - low[2] as f64,
    ];

    let mut output = [0.0, 0.0, 0.0];
    for blue_index in [low[2], high[2]] {
        let blue_weight = if blue_index == low[2] {
            1.0 - t[2]
        } else {
            t[2]
        };
        for green_index in [low[1], high[1]] {
            let green_weight = if green_index == low[1] {
                1.0 - t[1]
            } else {
                t[1]
            };
            for red_index in [low[0], high[0]] {
                let red_weight = if red_index == low[0] {
                    1.0 - t[0]
                } else {
                    t[0]
                };
                let weight = red_weight * green_weight * blue_weight;
                let value = cube_lut_value(lut, red_index, green_index, blue_index);
                for channel in 0..3 {
                    output[channel] += value[channel] * weight;
                }
            }
        }
    }
    output.map(|value| value.clamp(0.0, 1.0))
}

fn cube_lut_value(lut: &CachedCubeLut, red: usize, green: usize, blue: usize) -> [f64; 3] {
    lut.values[(blue * lut.size * lut.size) + (green * lut.size) + red]
}

fn checksum_lut_values(
    size: usize,
    domain_min: [f64; 3],
    domain_max: [f64; 3],
    values: &[[f64; 3]],
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in (size as u64).to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for value in domain_min
        .into_iter()
        .chain(domain_max)
        .chain(values.iter().flat_map(|value| value.iter().copied()))
    {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn cached_lut_asset(key: &LutAssetCacheKey) -> Option<CachedCubeLut> {
    LUT_ASSET_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|cache| cache.get(key).cloned())
}

fn store_cached_lut_asset(key: LutAssetCacheKey, lut: CachedCubeLut) {
    let Ok(mut cache) = LUT_ASSET_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    else {
        return;
    };
    cache.retain(|existing_key, _| {
        existing_key.path != key.path || existing_key.modified_unix_ms == key.modified_unix_ms
    });
    cache.insert(key, lut);
    if cache.len() > 32 {
        let Some(first_key) = cache.keys().next().cloned() else {
            return;
        };
        cache.remove(&first_key);
    }
}

fn box_blur_rgba(pixels: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    if radius == 0 || width == 0 || height == 0 {
        return pixels.to_vec();
    }

    let mut horizontal = vec![0_u8; pixels.len()];
    let mut output = vec![0_u8; pixels.len()];

    for y in 0..height {
        for x in 0..width {
            let start = x.saturating_sub(radius);
            let end = (x + radius).min(width - 1);
            let count = (end - start + 1) as u32;
            let mut totals = [0_u32; 4];
            for sample_x in start..=end {
                let offset = (y * width + sample_x) * 4;
                for channel in 0..4 {
                    totals[channel] += u32::from(pixels[offset + channel]);
                }
            }
            let offset = (y * width + x) * 4;
            for channel in 0..4 {
                horizontal[offset + channel] = (totals[channel] / count) as u8;
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            let start = y.saturating_sub(radius);
            let end = (y + radius).min(height - 1);
            let count = (end - start + 1) as u32;
            let mut totals = [0_u32; 4];
            for sample_y in start..=end {
                let offset = (sample_y * width + x) * 4;
                for channel in 0..4 {
                    totals[channel] += u32::from(horizontal[offset + channel]);
                }
            }
            let offset = (y * width + x) * 4;
            for channel in 0..4 {
                output[offset + channel] = (totals[channel] / count) as u8;
            }
        }
    }

    output
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
        if let Some(browser) = &input.browser {
            match browser.status {
                SoftwareCompositorBrowserStatus::Rendered => {}
                SoftwareCompositorBrowserStatus::NoUrl
                | SoftwareCompositorBrowserStatus::BrowserUnavailable
                | SoftwareCompositorBrowserStatus::UnsupportedUrl
                | SoftwareCompositorBrowserStatus::NavigationFailed
                | SoftwareCompositorBrowserStatus::CaptureFailed => {
                    validation.warnings.push(format!(
                        "{} browser overlay is using a placeholder: {}",
                        input.source_id, browser.status_detail
                    ));
                }
            }
        }
        if let Some(capture) = &input.capture {
            match capture.status {
                SoftwareCompositorCaptureStatus::Rendered => {}
                SoftwareCompositorCaptureStatus::NoSource
                | SoftwareCompositorCaptureStatus::PermissionRequired
                | SoftwareCompositorCaptureStatus::DecoderUnavailable
                | SoftwareCompositorCaptureStatus::UnsupportedPlatform
                | SoftwareCompositorCaptureStatus::UnsupportedSource
                | SoftwareCompositorCaptureStatus::CaptureFailed => {
                    validation.warnings.push(format!(
                        "{} capture source is using a placeholder: {}",
                        input.source_id, capture.status_detail
                    ));
                }
            }
        }
        for filter in &input.filters {
            match filter.status {
                SoftwareCompositorFilterStatus::Applied
                | SoftwareCompositorFilterStatus::Skipped => {}
                SoftwareCompositorFilterStatus::Deferred => {
                    validation.warnings.push(format!(
                        "{} filter \"{}\" is deferred in software preview: {}",
                        input.source_id, filter.name, filter.status_detail
                    ));
                }
                SoftwareCompositorFilterStatus::Error => {
                    validation.warnings.push(format!(
                        "{} filter \"{}\" could not run in software preview: {}",
                        input.source_id, filter.name, filter.status_detail
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

fn decode_video_asset(
    asset_uri: &str,
    clock: &CompositorFrameClock,
) -> Result<DecodedVideoAsset, Box<SoftwareCompositorAssetMetadata>> {
    decode_video_asset_with_decoder(asset_uri, clock, find_ffmpeg_binary())
}

fn decode_video_asset_with_decoder(
    asset_uri: &str,
    clock: &CompositorFrameClock,
    ffmpeg_path: Option<PathBuf>,
) -> Result<DecodedVideoAsset, Box<SoftwareCompositorAssetMetadata>> {
    let Some(path) = asset_uri_path(asset_uri) else {
        return Err(Box::new(asset_metadata(
            asset_uri.to_string(),
            SoftwareCompositorAssetStatus::NoAsset,
            "No local video asset has been selected.".to_string(),
            None,
        )));
    };
    let normalized_path = normalized_asset_path(&path);
    let Some(format) = supported_video_extension(&path) else {
        return Err(Box::new(asset_metadata(
            asset_uri.to_string(),
            SoftwareCompositorAssetStatus::UnsupportedExtension,
            "Unsupported video extension. Supported video assets are mp4, mov, webm, and mkv."
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
                format!("Video asset file does not exist: {normalized_path}"),
                Some(format),
            )));
        }
    };
    if !metadata.is_file() {
        return Err(Box::new(asset_metadata(
            asset_uri.to_string(),
            SoftwareCompositorAssetStatus::MissingFile,
            format!("Video asset path is not a file: {normalized_path}"),
            Some(format),
        )));
    }
    let Some(ffmpeg_path) = ffmpeg_path else {
        return Err(Box::new(asset_metadata(
            asset_uri.to_string(),
            SoftwareCompositorAssetStatus::VideoPlaceholder,
            "FFmpeg is not available; video preview frame extraction is disabled.".to_string(),
            Some(format),
        )));
    };

    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(system_time_unix_ms)
        .unwrap_or(0);
    let sample_time_ms = quantized_video_sample_time_ms(clock);
    let sample_index = sample_time_ms / VIDEO_SAMPLE_INTERVAL_MS;
    let key = VideoAssetCacheKey {
        path: normalized_path.clone(),
        modified_unix_ms,
        sample_time_ms,
    };
    if let Some(image) = cached_video_asset(&key) {
        return Ok(DecodedVideoAsset {
            modified_unix_ms,
            sample_time_ms,
            sample_index,
            decoder_name: "ffmpeg".to_string(),
            cache_hit: true,
            image,
        });
    }

    let mut decoded = extract_video_frame_with_ffmpeg(&ffmpeg_path, &path, sample_time_ms)
        .map_err(|error| {
            Box::new(asset_metadata(
                asset_uri.to_string(),
                SoftwareCompositorAssetStatus::DecodeFailed,
                format!("Video preview frame could not be decoded: {error}"),
                Some(format.clone()),
            ))
        })?;
    decoded.format = format;
    store_cached_video_asset(key, decoded.clone());

    Ok(DecodedVideoAsset {
        modified_unix_ms,
        sample_time_ms,
        sample_index,
        decoder_name: "ffmpeg".to_string(),
        cache_hit: false,
        image: decoded,
    })
}

fn extract_video_frame_with_ffmpeg(
    ffmpeg_path: &Path,
    path: &Path,
    sample_time_ms: u64,
) -> Result<CachedDecodedImage, String> {
    let sample_seconds = format!("{:.3}", sample_time_ms as f64 / 1000.0);
    let output = Command::new(ffmpeg_path)
        .arg("-v")
        .arg("error")
        .arg("-nostdin")
        .arg("-ss")
        .arg(sample_seconds)
        .arg("-i")
        .arg(path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-frames:v")
        .arg("1")
        .arg("-f")
        .arg("image2pipe")
        .arg("-vcodec")
        .arg("png")
        .arg("pipe:1")
        .output()
        .map_err(|error| format!("could not start ffmpeg: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("ffmpeg exited with status {}", output.status)
        } else {
            stderr
        });
    }
    if output.stdout.is_empty() {
        return Err("ffmpeg returned an empty preview frame".to_string());
    }

    let image = image::load_from_memory(&output.stdout).map_err(|error| error.to_string())?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = rgba.into_raw();
    Ok(CachedDecodedImage {
        width,
        height,
        format: "video".to_string(),
        checksum: checksum_pixels(&pixels),
        pixels,
    })
}

fn browser_overlay_snapshot(
    browser: &BrowserBinary,
    request: &BrowserSnapshotRequest,
) -> Result<BrowserSnapshot, BrowserCaptureError> {
    let key = BrowserSnapshotCacheKey {
        url: request.url.clone(),
        viewport_width: request.viewport_width,
        viewport_height: request.viewport_height,
        custom_css_hash: checksum_pixels(request.custom_css.as_bytes()),
        sample_time_ms: request.sample_time_ms,
    };
    if let Some(cached) = cached_browser_snapshot(&key) {
        return Ok(BrowserSnapshot {
            width: cached.width,
            height: cached.height,
            checksum: cached.checksum,
            pixels: cached.pixels,
            browser_name: browser.name.clone(),
            browser_path: browser.path.display().to_string(),
            sample_time_ms: request.sample_time_ms,
            sample_index: request.sample_index,
            capture_duration_ms: 0,
            custom_css_applied: !request.custom_css.trim().is_empty(),
            custom_css_detail: custom_css_success_detail(&request.custom_css),
            cache_hit: true,
        });
    }

    let started_at = Instant::now();
    let (snapshot, custom_css_applied, custom_css_detail) =
        capture_browser_snapshot_with_cdp(browser, request)?;
    let capture_duration_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    store_cached_browser_snapshot(key, snapshot.clone());

    Ok(BrowserSnapshot {
        width: snapshot.width,
        height: snapshot.height,
        checksum: snapshot.checksum,
        pixels: snapshot.pixels,
        browser_name: browser.name.clone(),
        browser_path: browser.path.display().to_string(),
        sample_time_ms: request.sample_time_ms,
        sample_index: request.sample_index,
        capture_duration_ms,
        custom_css_applied,
        custom_css_detail,
        cache_hit: false,
    })
}

fn capture_browser_snapshot_with_cdp(
    browser: &BrowserBinary,
    request: &BrowserSnapshotRequest,
) -> Result<(CachedBrowserSnapshot, bool, Option<String>), BrowserCaptureError> {
    let port = allocate_local_port()?;
    let user_data_dir = env::temp_dir().join(format!("vaexcore-browser-{}", Uuid::new_v4()));
    fs::create_dir_all(&user_data_dir).map_err(|error| BrowserCaptureError {
        status: SoftwareCompositorBrowserStatus::CaptureFailed,
        detail: format!("Could not create temporary browser profile: {error}"),
        custom_css_applied: false,
        custom_css_detail: None,
    })?;

    let mut child = Command::new(&browser.path)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-background-networking")
        .arg("--disable-extensions")
        .arg("--disable-sync")
        .arg("--hide-scrollbars")
        .arg("--run-all-compositor-stages-before-draw")
        .arg(format!(
            "--window-size={},{}",
            request.viewport_width, request.viewport_height
        ))
        .arg(format!("--remote-debugging-port={port}"))
        .arg(format!("--user-data-dir={}", user_data_dir.display()))
        .arg("about:blank")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| BrowserCaptureError {
            status: SoftwareCompositorBrowserStatus::CaptureFailed,
            detail: format!("Could not start {}: {error}", browser.name),
            custom_css_applied: false,
            custom_css_detail: None,
        })?;

    let result = run_browser_capture_session(port, request);
    stop_browser_child(&mut child);
    let _ = fs::remove_dir_all(&user_data_dir);
    result
}

fn run_browser_capture_session(
    port: u16,
    request: &BrowserSnapshotRequest,
) -> Result<(CachedBrowserSnapshot, bool, Option<String>), BrowserCaptureError> {
    let web_socket_url = wait_for_cdp_target(port)?;
    let mut cdp = CdpClient::connect(&web_socket_url).map_err(|error| BrowserCaptureError {
        status: SoftwareCompositorBrowserStatus::CaptureFailed,
        detail: format!("Could not connect to browser DevTools: {error}"),
        custom_css_applied: false,
        custom_css_detail: None,
    })?;

    cdp.send("Page.enable", serde_json::json!({}))
        .map_err(browser_capture_failed)?;
    cdp.send("Runtime.enable", serde_json::json!({}))
        .map_err(browser_capture_failed)?;
    cdp.send(
        "Emulation.setDeviceMetricsOverride",
        serde_json::json!({
            "width": request.viewport_width,
            "height": request.viewport_height,
            "deviceScaleFactor": 1,
            "mobile": false
        }),
    )
    .map_err(browser_capture_failed)?;

    let navigation = cdp
        .send("Page.navigate", serde_json::json!({ "url": request.url }))
        .map_err(browser_navigation_failed)?;
    if let Some(error_text) = navigation
        .get("errorText")
        .and_then(serde_json::Value::as_str)
    {
        return Err(BrowserCaptureError {
            status: SoftwareCompositorBrowserStatus::NavigationFailed,
            detail: format!("Browser overlay navigation failed: {error_text}"),
            custom_css_applied: false,
            custom_css_detail: None,
        });
    }
    if !wait_for_document_ready(&mut cdp) {
        return Err(BrowserCaptureError {
            status: SoftwareCompositorBrowserStatus::NavigationFailed,
            detail: "Browser overlay navigation did not finish before the preview timeout."
                .to_string(),
            custom_css_applied: false,
            custom_css_detail: None,
        });
    }

    let (custom_css_applied, custom_css_detail) = inject_browser_custom_css(&mut cdp, request);
    std::thread::sleep(Duration::from_millis(120));
    let result = cdp
        .send(
            "Page.captureScreenshot",
            serde_json::json!({
                "format": "png",
                "fromSurface": true,
                "captureBeyondViewport": false
            }),
        )
        .map_err(browser_capture_failed)?;
    let encoded = result
        .get("data")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| BrowserCaptureError {
            status: SoftwareCompositorBrowserStatus::CaptureFailed,
            detail: "Browser screenshot response did not include image data.".to_string(),
            custom_css_applied,
            custom_css_detail: custom_css_detail.clone(),
        })?;
    let png = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| BrowserCaptureError {
            status: SoftwareCompositorBrowserStatus::CaptureFailed,
            detail: format!("Browser screenshot data could not be decoded: {error}"),
            custom_css_applied,
            custom_css_detail: custom_css_detail.clone(),
        })?;
    let image = image::load_from_memory(&png).map_err(|error| BrowserCaptureError {
        status: SoftwareCompositorBrowserStatus::CaptureFailed,
        detail: format!("Browser screenshot image could not be decoded: {error}"),
        custom_css_applied,
        custom_css_detail: custom_css_detail.clone(),
    })?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = rgba.into_raw();
    Ok((
        CachedBrowserSnapshot {
            width,
            height,
            checksum: checksum_pixels(&pixels),
            pixels,
        },
        custom_css_applied,
        custom_css_detail,
    ))
}

fn inject_browser_custom_css(
    cdp: &mut CdpClient,
    request: &BrowserSnapshotRequest,
) -> (bool, Option<String>) {
    if request.custom_css.trim().is_empty() {
        return (false, None);
    }
    let css = serde_json::to_string(&request.custom_css).unwrap_or_else(|_| "\"\"".to_string());
    let expression = format!(
        "(() => {{
            const style = document.createElement('style');
            style.setAttribute('data-vaexcore-preview-css', 'true');
            style.textContent = {css};
            (document.head || document.documentElement).appendChild(style);
            return true;
        }})()"
    );
    match cdp.evaluate_bool(&expression) {
        Ok(true) => (true, custom_css_success_detail(&request.custom_css)),
        Ok(false) => (
            false,
            Some("Custom CSS injection returned false.".to_string()),
        ),
        Err(error) => (false, Some(format!("Custom CSS injection failed: {error}"))),
    }
}

fn wait_for_document_ready(cdp: &mut CdpClient) -> bool {
    let started_at = Instant::now();
    while started_at.elapsed() < Duration::from_secs(10) {
        if cdp
            .evaluate_bool(
                "document.readyState === 'complete' || document.readyState === 'interactive'",
            )
            .unwrap_or(false)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn browser_capture_failed(detail: String) -> BrowserCaptureError {
    BrowserCaptureError {
        status: SoftwareCompositorBrowserStatus::CaptureFailed,
        detail: format!("Browser overlay capture failed: {detail}"),
        custom_css_applied: false,
        custom_css_detail: None,
    }
}

fn browser_navigation_failed(detail: String) -> BrowserCaptureError {
    BrowserCaptureError {
        status: SoftwareCompositorBrowserStatus::NavigationFailed,
        detail: format!("Browser overlay navigation failed: {detail}"),
        custom_css_applied: false,
        custom_css_detail: None,
    }
}

fn custom_css_success_detail(custom_css: &str) -> Option<String> {
    if custom_css.trim().is_empty() {
        None
    } else {
        Some("Custom CSS applied in preview.".to_string())
    }
}

fn browser_viewport_for_node(node: &CompositorNode) -> (u32, u32) {
    config_size(&node.config, "viewport")
        .map(|size| {
            (
                size.width.max(1.0).round().min(3840.0) as u32,
                size.height.max(1.0).round().min(2160.0) as u32,
            )
        })
        .unwrap_or_else(|| {
            (
                node.transform.size.width.max(1.0).round().min(3840.0) as u32,
                node.transform.size.height.max(1.0).round().min(2160.0) as u32,
            )
        })
}

fn browser_metadata(
    status: SoftwareCompositorBrowserStatus,
    status_detail: String,
    url: Option<String>,
    viewport: (u32, u32),
    runtime: Option<(BrowserBinary, BrowserSnapshotRequest, bool, Option<String>)>,
) -> SoftwareCompositorBrowserMetadata {
    let (browser, request, custom_css_applied, custom_css_detail) = runtime.unwrap_or_else(|| {
        (
            BrowserBinary {
                name: String::new(),
                path: PathBuf::new(),
            },
            BrowserSnapshotRequest {
                url: url.clone().unwrap_or_default(),
                viewport_width: viewport.0,
                viewport_height: viewport.1,
                custom_css: String::new(),
                sample_time_ms: 0,
                sample_index: 0,
            },
            false,
            None,
        )
    });
    let browser_name = if browser.name.is_empty() {
        None
    } else {
        Some(browser.name)
    };
    let browser_path = if browser.path.as_os_str().is_empty() {
        None
    } else {
        Some(browser.path.display().to_string())
    };
    SoftwareCompositorBrowserMetadata {
        status,
        status_detail,
        url,
        viewport_width: viewport.0,
        viewport_height: viewport.1,
        custom_css_present: !request.custom_css.trim().is_empty(),
        custom_css_applied,
        custom_css_detail,
        browser_name,
        browser_path,
        sampled_frame_time_ms: (request.sample_time_ms > 0).then_some(request.sample_time_ms),
        sample_index: (request.sample_time_ms > 0).then_some(request.sample_index),
        checksum: None,
        capture_duration_ms: None,
        cache_hit: false,
    }
}

fn is_supported_browser_overlay_url(url: &str) -> bool {
    let trimmed = url.trim().to_ascii_lowercase();
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("file://")
}

fn quantized_browser_sample_time_ms(clock: &CompositorFrameClock) -> u64 {
    let pts_ms = clock.pts_nanos / 1_000_000;
    (pts_ms / BROWSER_SAMPLE_INTERVAL_MS) * BROWSER_SAMPLE_INTERVAL_MS
}

fn allocate_local_port() -> Result<u16, BrowserCaptureError> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| BrowserCaptureError {
            status: SoftwareCompositorBrowserStatus::CaptureFailed,
            detail: format!("Could not allocate a browser DevTools port: {error}"),
            custom_css_applied: false,
            custom_css_detail: None,
        })
}

fn wait_for_cdp_target(port: u16) -> Result<String, BrowserCaptureError> {
    let started_at = Instant::now();
    while started_at.elapsed() < Duration::from_secs(12) {
        if let Ok(targets) = http_get_json(port, "/json/list") {
            if let Some(web_socket_url) = targets
                .as_array()
                .and_then(|items| {
                    items.iter().find_map(|target| {
                        if target.get("type").and_then(serde_json::Value::as_str) != Some("page") {
                            return None;
                        }
                        target
                            .get("webSocketDebuggerUrl")
                            .and_then(serde_json::Value::as_str)
                    })
                })
                .map(ToOwned::to_owned)
            {
                return Ok(web_socket_url);
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err(BrowserCaptureError {
        status: SoftwareCompositorBrowserStatus::CaptureFailed,
        detail: "Timed out waiting for browser DevTools target.".to_string(),
        custom_css_applied: false,
        custom_css_detail: None,
    })
}

fn http_get_json(port: u16, path: &str) -> Result<serde_json::Value, String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|error| error.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .map_err(|error| error.to_string())?;
    let mut response_bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(bytes_read) => {
                response_bytes.extend_from_slice(&buffer[..bytes_read]);
                if http_response_body_complete(&response_bytes) {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) && !response_bytes.is_empty() =>
            {
                break;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let response = String::from_utf8_lossy(&response_bytes);
    let Some((headers, body)) = response.split_once("\r\n\r\n") else {
        return Err("DevTools response was malformed.".to_string());
    };
    if !headers.starts_with("HTTP/1.1 200") && !headers.starts_with("HTTP/1.0 200") {
        return Err(headers
            .lines()
            .next()
            .unwrap_or("DevTools HTTP error")
            .to_string());
    }
    serde_json::from_str(body).map_err(|error| error.to_string())
}

fn http_response_body_complete(response: &[u8]) -> bool {
    let Some(header_end) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let Some(content_length) = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    }) else {
        return false;
    };
    response.len().saturating_sub(header_end + 4) >= content_length
}

struct CdpClient {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    next_id: u64,
}

impl CdpClient {
    fn connect(web_socket_url: &str) -> Result<Self, String> {
        let (mut socket, _) =
            tungstenite::connect(web_socket_url).map_err(|error| error.to_string())?;
        if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
        }
        Ok(Self { socket, next_id: 1 })
    }

    fn send(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let payload = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        self.socket
            .send(Message::Text(payload.to_string().into()))
            .map_err(|error| error.to_string())?;
        loop {
            let message = self.socket.read().map_err(|error| error.to_string())?;
            let text = match message {
                Message::Text(text) => text.to_string(),
                Message::Binary(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
                Message::Close(_) => return Err("DevTools socket closed.".to_string()),
            };
            let value: serde_json::Value =
                serde_json::from_str(&text).map_err(|error| error.to_string())?;
            if value.get("id").and_then(serde_json::Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                let message = error
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("DevTools command failed");
                return Err(message.to_string());
            }
            return Ok(value
                .get("result")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})));
        }
    }

    fn evaluate_bool(&mut self, expression: &str) -> Result<bool, String> {
        let result = self.send(
            "Runtime.evaluate",
            serde_json::json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": true
            }),
        )?;
        if let Some(exception) = result.get("exceptionDetails") {
            return Err(exception.to_string());
        }
        Ok(result
            .get("result")
            .and_then(|result| result.get("value"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false))
    }
}

fn stop_browser_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
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

fn supported_video_extension(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match extension.as_str() {
        "mp4" | "mov" | "webm" | "mkv" => Some(extension),
        _ => None,
    }
}

fn quantized_video_sample_time_ms(clock: &CompositorFrameClock) -> u64 {
    let pts_ms = clock.pts_nanos / 1_000_000;
    (pts_ms / VIDEO_SAMPLE_INTERVAL_MS) * VIDEO_SAMPLE_INTERVAL_MS
}

fn find_ffmpeg_binary() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(explicit_path) = env::var_os("VAEXCORE_FFMPEG_PATH") {
        candidates.push(PathBuf::from(explicit_path));
    }
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            for executable_name in ffmpeg_executable_names() {
                candidates.push(directory.join(executable_name));
            }
        }
    }
    add_windows_ffmpeg_candidates(&mut candidates);
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/ffmpeg"),
        PathBuf::from("/usr/local/bin/ffmpeg"),
        PathBuf::from("/usr/bin/ffmpeg"),
    ]);

    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn find_browser_binary() -> Option<BrowserBinary> {
    let mut candidates = Vec::new();
    if let Some(explicit_path) = env::var_os("VAEXCORE_BROWSER_PATH") {
        candidates.push(PathBuf::from(explicit_path));
    }
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            for executable_name in browser_executable_names() {
                candidates.push(directory.join(executable_name));
            }
        }
    }
    add_platform_browser_candidates(&mut candidates);

    let mut seen = HashSet::new();
    candidates.into_iter().find_map(|candidate| {
        let key = candidate.display().to_string();
        if !seen.insert(key) || !candidate.is_file() {
            return None;
        }
        Some(BrowserBinary {
            name: browser_name_for_path(&candidate),
            path: candidate,
        })
    })
}

fn browser_executable_names() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &[
            "chrome.exe",
            "msedge.exe",
            "chromium.exe",
            "chrome",
            "msedge",
            "chromium",
        ]
    } else {
        &[
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "msedge",
            "chrome",
        ]
    }
}

fn add_platform_browser_candidates(candidates: &mut Vec<PathBuf>) {
    if cfg!(target_os = "macos") {
        candidates.extend([
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
            PathBuf::from("/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
        ]);
    } else if cfg!(target_os = "windows") {
        if let Some(program_files) = env::var_os("ProgramFiles") {
            let root = PathBuf::from(program_files);
            candidates.push(root.join("Google/Chrome/Application/chrome.exe"));
            candidates.push(root.join("Microsoft/Edge/Application/msedge.exe"));
        }
        if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
            let root = PathBuf::from(program_files_x86);
            candidates.push(root.join("Google/Chrome/Application/chrome.exe"));
            candidates.push(root.join("Microsoft/Edge/Application/msedge.exe"));
        }
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            let root = PathBuf::from(local_app_data);
            candidates.push(root.join("Google/Chrome/Application/chrome.exe"));
            candidates.push(root.join("Microsoft/Edge/Application/msedge.exe"));
        }
    } else {
        candidates.extend([
            PathBuf::from("/usr/bin/google-chrome"),
            PathBuf::from("/usr/bin/google-chrome-stable"),
            PathBuf::from("/usr/bin/chromium"),
            PathBuf::from("/usr/bin/chromium-browser"),
            PathBuf::from("/usr/bin/microsoft-edge"),
            PathBuf::from("/snap/bin/chromium"),
        ]);
    }
}

fn browser_name_for_path(path: &Path) -> String {
    let display = path.display().to_string();
    let lower = display.to_ascii_lowercase();
    if lower.contains("edge") || lower.contains("msedge") {
        "Microsoft Edge".to_string()
    } else if lower.contains("chromium") {
        "Chromium".to_string()
    } else if lower.contains("chrome") {
        "Google Chrome".to_string()
    } else {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Browser")
            .to_string()
    }
}

fn ffmpeg_executable_names() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["ffmpeg.exe", "ffmpeg"]
    } else {
        &["ffmpeg"]
    }
}

fn add_windows_ffmpeg_candidates(candidates: &mut Vec<PathBuf>) {
    if !cfg!(target_os = "windows") {
        return;
    }

    candidates.extend([
        PathBuf::from("C:\\ffmpeg\\bin\\ffmpeg.exe"),
        PathBuf::from("C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe"),
        PathBuf::from("C:\\ProgramData\\chocolatey\\bin\\ffmpeg.exe"),
    ]);
    if let Some(user_profile) = env::var_os("USERPROFILE") {
        candidates.push(PathBuf::from(user_profile).join("scoop\\shims\\ffmpeg.exe"));
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

fn cached_video_asset(key: &VideoAssetCacheKey) -> Option<CachedDecodedImage> {
    VIDEO_ASSET_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|cache| cache.get(key).cloned())
}

fn store_cached_video_asset(key: VideoAssetCacheKey, image: CachedDecodedImage) {
    let Ok(mut cache) = VIDEO_ASSET_CACHE
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

fn cached_browser_snapshot(key: &BrowserSnapshotCacheKey) -> Option<CachedBrowserSnapshot> {
    BROWSER_SNAPSHOT_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()
        .and_then(|cache| cache.get(key).cloned())
}

fn store_cached_browser_snapshot(key: BrowserSnapshotCacheKey, snapshot: CachedBrowserSnapshot) {
    let Ok(mut cache) = BROWSER_SNAPSHOT_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    else {
        return;
    };
    cache.retain(|existing_key, _| {
        existing_key.url != key.url
            || existing_key.viewport_width != key.viewport_width
            || existing_key.viewport_height != key.viewport_height
            || existing_key.custom_css_hash != key.custom_css_hash
            || existing_key.sample_time_ms == key.sample_time_ms
    });
    cache.insert(key, snapshot);
    if cache.len() > 32 {
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
        sampled_frame_time_ms: None,
        sample_index: None,
        decoder_name: None,
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

fn color_channel_to_unit(channel: u8) -> f64 {
    f64::from(channel) / 255.0
}

fn unit_to_color_channel(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn color_distance_unit(left: [u8; 3], right: [u8; 3]) -> f64 {
    let red = color_channel_to_unit(left[0]) - color_channel_to_unit(right[0]);
    let green = color_channel_to_unit(left[1]) - color_channel_to_unit(right[1]);
    let blue = color_channel_to_unit(left[2]) - color_channel_to_unit(right[2]);
    ((red * red + green * green + blue * blue) / 3.0).sqrt()
}

fn color_luma_unit(color: [u8; 3]) -> f64 {
    color_channel_to_unit(color[0]) * 0.2126
        + color_channel_to_unit(color[1]) * 0.7152
        + color_channel_to_unit(color[2]) * 0.0722
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

pub fn checksum_software_pixels(pixels: &[u8]) -> u64 {
    checksum_pixels(pixels)
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
    fn software_capture_input_reports_permission_placeholder() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let graph = build_compositor_graph(scene);
        let display = graph
            .nodes
            .iter()
            .find(|node| node.source_id == "source-main-display")
            .unwrap();

        let input = software_input_frame_for_node(display, &default_software_input_clock());
        let capture = input.capture.unwrap();

        assert_eq!(input.status, CompositorNodeStatus::PermissionRequired);
        assert_eq!(
            capture.status,
            SoftwareCompositorCaptureStatus::PermissionRequired
        );
        assert_eq!(capture.capture_source_id.as_deref(), Some("display:main"));
        assert_eq!(capture.frame_index, 0);
    }

    #[test]
    fn software_camera_capture_input_reports_missing_source_placeholder() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let graph = build_compositor_graph(scene);
        let camera = graph
            .nodes
            .iter()
            .find(|node| node.source_id == "source-camera-placeholder")
            .unwrap();

        let input = software_input_frame_for_node(camera, &default_software_input_clock());
        let capture = input.capture.unwrap();

        assert_eq!(input.status, CompositorNodeStatus::Placeholder);
        assert_eq!(capture.status, SoftwareCompositorCaptureStatus::NoSource);
        assert_eq!(capture.capture_kind, CaptureSourceKind::Camera);
        assert_eq!(capture.capture_source_id, None);
    }

    #[test]
    fn software_capture_input_accepts_mocked_display_pixels() {
        fn fake_provider(
            node: &CompositorNode,
            clock: &CompositorFrameClock,
        ) -> Result<DecodedCaptureFrame, CaptureFrameError> {
            Ok(DecodedCaptureFrame {
                capture_source_id: capture_source_id_for_node(node),
                width: 2,
                height: 2,
                frame_index: clock.frame_index,
                capture_duration_ms: 3,
                provider_name: "test-provider".to_string(),
                status_detail: "Captured mocked display frame.".to_string(),
                pixels: vec![
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
                ],
            })
        }

        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let mut graph = build_compositor_graph(scene);
        let display = graph
            .nodes
            .iter_mut()
            .find(|node| node.source_id == "source-main-display")
            .unwrap();
        display.status = CompositorNodeStatus::Ready;
        display.status_detail = "Display ready.".to_string();
        display.config["availability"] =
            serde_json::json!({ "state": "available", "detail": "Display ready." });

        let clock = CompositorFrameClock {
            frame_index: 9,
            framerate: 30,
            pts_nanos: 300_000_000,
            duration_nanos: 33_333_333,
        };
        let input = capture_input_frame_for_node_with_provider(display, &clock, fake_provider);
        let capture = input.capture.unwrap();

        assert_eq!(input.status, CompositorNodeStatus::Ready);
        assert_eq!(input.width, 2);
        assert_eq!(input.height, 2);
        assert_eq!(capture.status, SoftwareCompositorCaptureStatus::Rendered);
        assert_eq!(capture.frame_index, 9);
        assert_eq!(capture.capture_duration_ms, Some(3));
        assert_eq!(capture.provider_name, "test-provider");
        assert_eq!(capture.checksum, Some(input.checksum));
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
            SoftwareCompositorAssetStatus::MissingFile
        );

        let unavailable_path = dir.path().join("unavailable.webm");
        fs::write(&unavailable_path, b"not a video").unwrap();
        let unavailable = decode_video_asset_with_decoder(
            &unavailable_path.display().to_string(),
            &default_software_input_clock(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            unavailable.status,
            SoftwareCompositorAssetStatus::VideoPlaceholder
        );
    }

    #[test]
    fn software_compositor_decodes_video_preview_frames_when_ffmpeg_is_available() {
        let Some(ffmpeg_path) = find_ffmpeg_binary() else {
            eprintln!("skipping video preview decode test because ffmpeg is unavailable");
            return;
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("clip.mp4");
        if !write_test_video(&ffmpeg_path, &path, "red") {
            eprintln!("skipping video preview decode test because ffmpeg could not create fixture");
            return;
        }

        let result = render_test_image_source(&path.display().to_string(), Some("video"));
        let input = &result.input_frames[0];
        let asset = input.asset.as_ref().unwrap();

        assert_eq!(
            input.status,
            CompositorNodeStatus::Ready,
            "{:?}",
            input.browser
        );
        assert_eq!(asset.status, SoftwareCompositorAssetStatus::Decoded);
        assert_eq!(asset.format.as_deref(), Some("video:mp4"));
        assert_eq!(asset.decoder_name.as_deref(), Some("ffmpeg"));
        assert_eq!(asset.sampled_frame_time_ms, Some(0));
        assert!(asset.checksum.is_some_and(|checksum| checksum > 0));
        assert_ne!(
            software_test_pixel(&result.pixel_frames[0], 4, 4),
            [5, 7, 17, 255]
        );
    }

    #[test]
    fn software_compositor_reports_video_asset_errors_without_mutating_pixels() {
        let dir = tempdir().unwrap();
        let missing_path = dir.path().join("missing.mp4");
        let missing = render_test_image_source(&missing_path.display().to_string(), Some("video"));
        assert_eq!(
            missing.input_frames[0].asset.as_ref().unwrap().status,
            SoftwareCompositorAssetStatus::MissingFile
        );
        assert_eq!(
            missing.input_frames[0].status,
            CompositorNodeStatus::Placeholder
        );

        let unsupported_path = dir.path().join("clip.avi");
        fs::write(&unsupported_path, b"not a video").unwrap();
        let unsupported =
            render_test_image_source(&unsupported_path.display().to_string(), Some("video"));
        assert_eq!(
            unsupported.input_frames[0].asset.as_ref().unwrap().status,
            SoftwareCompositorAssetStatus::UnsupportedExtension
        );

        if find_ffmpeg_binary().is_some() {
            let broken_path = dir.path().join("broken.mp4");
            fs::write(&broken_path, b"not a video").unwrap();
            let broken =
                render_test_image_source(&broken_path.display().to_string(), Some("video"));
            assert_eq!(
                broken.input_frames[0].asset.as_ref().unwrap().status,
                SoftwareCompositorAssetStatus::DecodeFailed
            );
        }
    }

    #[test]
    fn software_compositor_caches_video_frames_by_sample_time_and_modified_time() {
        let Some(ffmpeg_path) = find_ffmpeg_binary() else {
            eprintln!("skipping video cache test because ffmpeg is unavailable");
            return;
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("clip.mp4");
        if !write_test_video(&ffmpeg_path, &path, "blue") {
            eprintln!("skipping video cache test because ffmpeg could not create fixture");
            return;
        }
        let scene = test_image_scene(&path.display().to_string(), Some("video"));

        let first = render_test_scene_with_target_at_frame(scene.clone(), 8, 8, 16);
        let second = render_test_scene_with_target_at_frame(scene.clone(), 8, 8, 17);
        assert!(!first.input_frames[0].asset.as_ref().unwrap().cache_hit);
        assert!(second.input_frames[0].asset.as_ref().unwrap().cache_hit);
        assert_eq!(
            second.input_frames[0]
                .asset
                .as_ref()
                .unwrap()
                .sampled_frame_time_ms,
            Some(500)
        );

        thread::sleep(Duration::from_millis(20));
        if !write_test_video(&ffmpeg_path, &path, "green") {
            eprintln!("skipping video cache invalidation assertion because fixture rewrite failed");
            return;
        }
        let rewritten = render_test_scene_with_target_at_frame(scene, 8, 8, 17);
        assert!(!rewritten.input_frames[0].asset.as_ref().unwrap().cache_hit);
    }

    #[test]
    fn software_compositor_applies_filters_and_transforms_to_decoded_video_frames() {
        let Some(ffmpeg_path) = find_ffmpeg_binary() else {
            eprintln!("skipping video transform test because ffmpeg is unavailable");
            return;
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("clip.mp4");
        if !write_test_video(&ffmpeg_path, &path, "yellow") {
            eprintln!("skipping video transform test because ffmpeg could not create fixture");
            return;
        }
        let mut scene = test_image_scene(&path.display().to_string(), Some("video"));
        scene.sources[0].position = ScenePoint { x: 2.0, y: 2.0 };
        scene.sources[0].size = SceneSize {
            width: 4.0,
            height: 4.0,
        };
        scene.sources[0].opacity = 0.5;
        scene.sources[0].bounds_mode = SceneSourceBoundsMode::Fit;
        scene.sources[0].filters = vec![test_filter(
            "filter-brightness",
            SceneSourceFilterKind::ColorCorrection,
            10,
            true,
            serde_json::json!({
                "brightness": -0.4,
                "contrast": 1.0,
                "saturation": 1.0,
                "gamma": 1.0
            }),
        )];

        let result = render_test_scene(scene);

        assert_eq!(
            result.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_eq!(
            result.frame.targets[0].nodes[0].status,
            CompositorNodeStatus::Ready
        );
        assert_ne!(
            software_test_pixel(&result.pixel_frames[0], 3, 3),
            [5, 7, 17, 255]
        );
    }

    #[test]
    fn software_compositor_reports_browser_overlay_placeholder_states() {
        let no_url = render_test_browser_source("", None);
        let browser = no_url.input_frames[0].browser.as_ref().unwrap();
        assert_eq!(browser.status, SoftwareCompositorBrowserStatus::NoUrl);
        assert_eq!(
            no_url.input_frames[0].status,
            CompositorNodeStatus::Placeholder
        );
        assert!(!no_url.frame.validation.warnings.is_empty());

        let unsupported = render_test_browser_source("ftp://example.test/overlay", None);
        assert_eq!(
            unsupported.input_frames[0].browser.as_ref().unwrap().status,
            SoftwareCompositorBrowserStatus::UnsupportedUrl
        );

        let scene = test_browser_scene("https://example.com/overlay", None);
        let graph = build_compositor_graph(&scene);
        let unavailable = browser_overlay_input_frame_for_node_with_browser(
            &graph.nodes[0],
            &default_software_input_clock(),
            None,
        );
        assert_eq!(
            unavailable.browser.as_ref().unwrap().status,
            SoftwareCompositorBrowserStatus::BrowserUnavailable
        );
        assert_eq!(unavailable.status, CompositorNodeStatus::Placeholder);
    }

    #[test]
    fn software_compositor_renders_browser_overlay_pixels_when_browser_is_available() {
        let Some(_) = find_browser_binary() else {
            eprintln!(
                "skipping browser overlay render test because no compatible browser is available"
            );
            return;
        };
        let dir = tempdir().unwrap();
        let url = write_browser_fixture(dir.path(), "overlay.html", "rgb(15, 80, 190)");

        let result = render_test_browser_source(&url, None);
        let input = &result.input_frames[0];
        let browser = input.browser.as_ref().unwrap();

        assert_eq!(input.status, CompositorNodeStatus::Ready);
        assert_eq!(browser.status, SoftwareCompositorBrowserStatus::Rendered);
        assert_eq!(browser.viewport_width, 128);
        assert_eq!(browser.viewport_height, 72);
        assert_eq!(browser.sampled_frame_time_ms, Some(0));
        assert!(browser.checksum.is_some_and(|checksum| checksum > 0));
        assert_ne!(
            software_test_pixel(&result.pixel_frames[0], 64, 36),
            [5, 7, 17, 255]
        );
    }

    #[test]
    fn software_compositor_applies_browser_custom_css_and_cache_keys() {
        let Some(_) = find_browser_binary() else {
            eprintln!("skipping browser overlay CSS/cache test because no compatible browser is available");
            return;
        };
        let dir = tempdir().unwrap();
        let url = write_browser_fixture(dir.path(), "cache.html", "rgb(180, 20, 20)");

        let baseline =
            render_test_scene_with_target_at_frame(test_browser_scene(&url, None), 128, 72, 31);
        let cached =
            render_test_scene_with_target_at_frame(test_browser_scene(&url, None), 128, 72, 32);
        assert!(!baseline.input_frames[0].browser.as_ref().unwrap().cache_hit);
        assert!(
            cached.input_frames[0].browser.as_ref().unwrap().cache_hit,
            "{:?}",
            cached.input_frames[0].browser
        );
        assert_eq!(
            cached.input_frames[0]
                .browser
                .as_ref()
                .unwrap()
                .sampled_frame_time_ms,
            Some(1000)
        );

        let styled = render_test_scene_with_target_at_frame(
            test_browser_scene(
                &url,
                Some("body { background: rgb(20, 170, 80) !important; }"),
            ),
            128,
            72,
            32,
        );
        let styled_browser = styled.input_frames[0].browser.as_ref().unwrap();
        assert_eq!(
            styled_browser.status,
            SoftwareCompositorBrowserStatus::Rendered
        );
        assert!(!styled_browser.cache_hit);
        assert!(styled_browser.custom_css_present);
        assert!(styled_browser.custom_css_applied);
        assert_ne!(
            baseline.input_frames[0].checksum,
            styled.input_frames[0].checksum
        );
    }

    #[test]
    fn software_compositor_applies_filters_and_transforms_to_browser_overlay_pixels() {
        let Some(_) = find_browser_binary() else {
            eprintln!("skipping browser overlay transform test because no compatible browser is available");
            return;
        };
        let dir = tempdir().unwrap();
        let url = write_browser_fixture(dir.path(), "filtered.html", "rgb(220, 190, 40)");
        let mut scene = test_browser_scene(&url, None);
        scene.canvas.width = 8;
        scene.canvas.height = 8;
        scene.sources[0].position = ScenePoint { x: 2.0, y: 2.0 };
        scene.sources[0].size = SceneSize {
            width: 4.0,
            height: 4.0,
        };
        scene.sources[0].opacity = 0.5;
        scene.sources[0].bounds_mode = SceneSourceBoundsMode::Fit;
        scene.sources[0].filters = vec![test_filter(
            "filter-browser-brightness",
            SceneSourceFilterKind::ColorCorrection,
            10,
            true,
            serde_json::json!({
                "brightness": -0.3,
                "contrast": 1.0,
                "saturation": 1.0,
                "gamma": 1.0
            }),
        )];

        let result = render_test_scene(scene);

        assert_eq!(
            result.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_eq!(
            result.frame.targets[0].nodes[0].status,
            CompositorNodeStatus::Ready,
            "{:?}",
            result.input_frames[0].browser
        );
        assert_ne!(
            software_test_pixel(&result.pixel_frames[0], 3, 3),
            [5, 7, 17, 255]
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
    fn software_compositor_applies_color_correction_filter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("gray.png");
        write_test_image(&path, ImageFormat::Png, [96, 96, 96, 255]);

        let baseline = render_test_image_source(&path.display().to_string(), None);
        let filtered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![test_filter(
                "filter-color",
                SceneSourceFilterKind::ColorCorrection,
                10,
                true,
                serde_json::json!({
                    "brightness": 0.25,
                    "contrast": 1.2,
                    "saturation": 1.0,
                    "gamma": 1.0
                }),
            )],
        );
        let filter = &filtered.input_frames[0].filters[0];

        assert_eq!(filter.status, SoftwareCompositorFilterStatus::Applied);
        assert_ne!(
            baseline.input_frames[0].checksum,
            filtered.input_frames[0].checksum
        );
        assert!(
            input_test_pixel(&filtered.input_frames[0], 0, 0)[0]
                > input_test_pixel(&baseline.input_frames[0], 0, 0)[0]
        );
    }

    #[test]
    fn software_compositor_applies_chroma_key_filter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("keyed.png");
        write_split_image(&path, [0, 255, 0, 255], [255, 0, 0, 255]);

        let filtered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![test_filter(
                "filter-key",
                SceneSourceFilterKind::ChromaKey,
                10,
                true,
                serde_json::json!({
                    "key_color": "#00ff00",
                    "similarity": 0.04,
                    "smoothness": 0.02
                }),
            )],
        );

        assert_eq!(
            filtered.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_eq!(input_test_pixel(&filtered.input_frames[0], 0, 0)[3], 0);
        assert_eq!(input_test_pixel(&filtered.input_frames[0], 3, 0)[3], 255);
    }

    #[test]
    fn software_compositor_reports_chroma_key_filter_errors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("keyed.png");
        write_test_image(&path, ImageFormat::Png, [0, 255, 0, 255]);

        let filtered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![test_filter(
                "filter-key",
                SceneSourceFilterKind::ChromaKey,
                10,
                true,
                serde_json::json!({
                    "key_color": "not-a-color",
                    "similarity": 0.04,
                    "smoothness": 0.02
                }),
            )],
        );
        let filter = &filtered.input_frames[0].filters[0];

        assert_eq!(filter.status, SoftwareCompositorFilterStatus::Error);
        assert!(filter.status_detail.contains("invalid"));
        assert!(filtered
            .frame
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("could not run")));
    }

    #[test]
    fn software_compositor_applies_crop_pad_filter_as_alpha_crop() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("solid.png");
        write_test_image_size(&path, ImageFormat::Png, [255, 0, 0, 255], 4, 4);

        let filtered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![test_filter(
                "filter-crop",
                SceneSourceFilterKind::CropPad,
                10,
                true,
                serde_json::json!({ "top": 1, "right": 0, "bottom": 0, "left": 2 }),
            )],
        );

        assert_eq!(
            filtered.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_eq!(input_test_pixel(&filtered.input_frames[0], 0, 2)[3], 0);
        assert_eq!(input_test_pixel(&filtered.input_frames[0], 3, 2)[3], 255);
        assert_eq!(input_test_pixel(&filtered.input_frames[0], 3, 0)[3], 0);
    }

    #[test]
    fn software_compositor_applies_blur_filter_deterministically() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("checker.png");
        write_checker_image(&path);

        let baseline = render_test_image_source(&path.display().to_string(), None);
        let filtered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![test_filter(
                "filter-blur",
                SceneSourceFilterKind::Blur,
                10,
                true,
                serde_json::json!({ "radius": 1 }),
            )],
        );

        assert_eq!(
            filtered.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_ne!(
            baseline.input_frames[0].checksum,
            filtered.input_frames[0].checksum
        );
        assert_ne!(
            input_test_pixel(&filtered.input_frames[0], 0, 0),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn software_compositor_applies_sharpen_filter_deterministically() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("soft.png");
        write_soft_spot_image(&path);

        let baseline = render_test_image_source(&path.display().to_string(), None);
        let filtered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![test_filter(
                "filter-sharpen",
                SceneSourceFilterKind::Sharpen,
                10,
                true,
                serde_json::json!({ "amount": 0.8 }),
            )],
        );

        assert_eq!(
            filtered.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_ne!(
            baseline.input_frames[0].checksum,
            filtered.input_frames[0].checksum
        );
    }

    #[test]
    fn software_compositor_skips_disabled_filters_without_mutating_pixels() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("gray.png");
        write_test_image(&path, ImageFormat::Png, [96, 96, 96, 255]);

        let baseline = render_test_image_source(&path.display().to_string(), None);
        let filtered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![test_filter(
                "filter-disabled",
                SceneSourceFilterKind::ColorCorrection,
                10,
                false,
                serde_json::json!({
                    "brightness": 0.8,
                    "contrast": 2.0,
                    "saturation": 2.0,
                    "gamma": 1.0
                }),
            )],
        );

        assert_eq!(
            filtered.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Skipped
        );
        assert_eq!(
            baseline.input_frames[0].checksum,
            filtered.input_frames[0].checksum
        );
    }

    #[test]
    fn software_compositor_applies_filters_in_deterministic_order() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("gray.png");
        write_test_image(&path, ImageFormat::Png, [96, 96, 96, 255]);
        let gamma = test_filter(
            "filter-gamma",
            SceneSourceFilterKind::ColorCorrection,
            20,
            true,
            serde_json::json!({
                "brightness": 0,
                "contrast": 1,
                "saturation": 1,
                "gamma": 2.0
            }),
        );
        let brightness = test_filter(
            "filter-brightness",
            SceneSourceFilterKind::ColorCorrection,
            10,
            true,
            serde_json::json!({
                "brightness": 0.18,
                "contrast": 1,
                "saturation": 1,
                "gamma": 1
            }),
        );

        let reversed = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![gamma.clone(), brightness.clone()],
        );
        let ordered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![brightness, gamma],
        );

        assert_eq!(
            reversed.input_frames[0].checksum,
            ordered.input_frames[0].checksum
        );
        assert_eq!(reversed.input_frames[0].filters[0].id, "filter-brightness");
        assert_eq!(reversed.input_frames[0].filters[1].id, "filter-gamma");
    }

    #[test]
    fn software_compositor_applies_mask_blend_alpha_shaping() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("solid.png");
        let mask_path = dir.path().join("mask.png");
        write_test_image(&source_path, ImageFormat::Png, [255, 0, 0, 255]);
        write_split_image(&mask_path, [0, 0, 0, 255], [255, 255, 255, 255]);

        let filtered = render_test_image_source_with_filters(
            &source_path.display().to_string(),
            vec![test_filter(
                "filter-mask",
                SceneSourceFilterKind::MaskBlend,
                10,
                true,
                serde_json::json!({
                    "mask_uri": mask_path.display().to_string(),
                    "blend_mode": "alpha"
                }),
            )],
        );

        assert_eq!(
            filtered.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_eq!(input_test_pixel(&filtered.input_frames[0], 0, 0)[3], 0);
        assert_eq!(input_test_pixel(&filtered.input_frames[0], 3, 0)[3], 255);
        assert_eq!(
            &input_test_pixel(&filtered.input_frames[0], 3, 0)[0..3],
            &[255, 0, 0]
        );
    }

    #[test]
    fn software_compositor_applies_mask_blend_modes_deterministically() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("solid.png");
        let mask_path = dir.path().join("mask.png");
        write_test_image(&source_path, ImageFormat::Png, [96, 128, 180, 255]);
        write_test_image(&mask_path, ImageFormat::Png, [64, 196, 128, 255]);

        let mut checksums = HashSet::new();
        for blend_mode in ["normal", "multiply", "screen", "overlay", "alpha"] {
            let filtered = render_test_image_source_with_filters(
                &source_path.display().to_string(),
                vec![test_filter(
                    &format!("filter-{blend_mode}"),
                    SceneSourceFilterKind::MaskBlend,
                    10,
                    true,
                    serde_json::json!({
                        "mask_uri": mask_path.display().to_string(),
                        "blend_mode": blend_mode
                    }),
                )],
            );

            assert_eq!(
                filtered.input_frames[0].filters[0].status,
                SoftwareCompositorFilterStatus::Applied
            );
            checksums.insert(filtered.input_frames[0].checksum);
        }

        assert_eq!(checksums.len(), 5);
    }

    #[test]
    fn software_compositor_reports_mask_blend_asset_errors_without_mutating_pixels() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("solid.png");
        write_test_image(&source_path, ImageFormat::Png, [255, 0, 0, 255]);

        let baseline = render_test_image_source(&source_path.display().to_string(), None);
        for (mask_uri, expected_detail) in [
            ("", "No mask image"),
            ("missing.png", "could not be loaded"),
            ("mask.txt", "Unsupported image extension"),
        ] {
            if mask_uri == "mask.txt" {
                fs::write(dir.path().join(mask_uri), b"not an image").unwrap();
            }
            let uri = if mask_uri.is_empty() {
                String::new()
            } else {
                dir.path().join(mask_uri).display().to_string()
            };
            let filtered = render_test_image_source_with_filters(
                &source_path.display().to_string(),
                vec![test_filter(
                    "filter-mask",
                    SceneSourceFilterKind::MaskBlend,
                    10,
                    true,
                    serde_json::json!({ "mask_uri": uri, "blend_mode": "normal" }),
                )],
            );
            let filter = &filtered.input_frames[0].filters[0];

            assert_eq!(filter.status, SoftwareCompositorFilterStatus::Error);
            assert!(filter.status_detail.contains(expected_detail));
            assert_eq!(
                baseline.input_frames[0].checksum,
                filtered.input_frames[0].checksum
            );
        }
    }

    #[test]
    fn software_compositor_reports_undecodable_mask_blend_assets() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("solid.png");
        let mask_path = dir.path().join("broken.png");
        write_test_image(&source_path, ImageFormat::Png, [255, 0, 0, 255]);
        fs::write(&mask_path, b"not a png").unwrap();

        let filtered = render_test_image_source_with_filters(
            &source_path.display().to_string(),
            vec![test_filter(
                "filter-mask",
                SceneSourceFilterKind::MaskBlend,
                10,
                true,
                serde_json::json!({
                    "mask_uri": mask_path.display().to_string(),
                    "blend_mode": "normal"
                }),
            )],
        );

        assert_eq!(
            filtered.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Error
        );
        assert!(filtered.input_frames[0].filters[0]
            .status_detail
            .contains("could not be decoded"));
    }

    #[test]
    fn software_compositor_applies_cube_lut_with_strength() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("source.png");
        let lut_path = dir.path().join("invert.cube");
        write_test_image(&source_path, ImageFormat::Png, [64, 128, 192, 255]);
        write_cube_lut(&lut_path, |red, green, blue| {
            [1.0 - red, 1.0 - green, 1.0 - blue]
        });

        let baseline = render_test_image_source(&source_path.display().to_string(), None);
        let zero = render_test_image_source_with_filters(
            &source_path.display().to_string(),
            vec![test_filter(
                "filter-lut-zero",
                SceneSourceFilterKind::Lut,
                10,
                true,
                serde_json::json!({ "lut_uri": lut_path.display().to_string(), "strength": 0 }),
            )],
        );
        let half = render_test_image_source_with_filters(
            &source_path.display().to_string(),
            vec![test_filter(
                "filter-lut-half",
                SceneSourceFilterKind::Lut,
                10,
                true,
                serde_json::json!({ "lut_uri": lut_path.display().to_string(), "strength": 0.5 }),
            )],
        );
        let full = render_test_image_source_with_filters(
            &source_path.display().to_string(),
            vec![test_filter(
                "filter-lut-full",
                SceneSourceFilterKind::Lut,
                10,
                true,
                serde_json::json!({ "lut_uri": lut_path.display().to_string(), "strength": 1 }),
            )],
        );

        assert_eq!(
            zero.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_eq!(
            baseline.input_frames[0].checksum,
            zero.input_frames[0].checksum
        );
        assert_ne!(
            baseline.input_frames[0].checksum,
            half.input_frames[0].checksum
        );
        assert_ne!(half.input_frames[0].checksum, full.input_frames[0].checksum);
        assert_eq!(
            full.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
    }

    #[test]
    fn software_compositor_reports_cube_lut_asset_errors_without_mutating_pixels() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("source.png");
        write_test_image(&source_path, ImageFormat::Png, [64, 128, 192, 255]);
        fs::write(dir.path().join("lut.txt"), "not a cube").unwrap();
        fs::write(dir.path().join("broken.cube"), "LUT_3D_SIZE 2\n0 0 0\n").unwrap();

        let baseline = render_test_image_source(&source_path.display().to_string(), None);
        for (lut_uri, expected_detail) in [
            ("", "No LUT file"),
            ("missing.cube", "does not exist"),
            ("lut.txt", "Unsupported LUT extension"),
            ("broken.cube", "could not be parsed"),
        ] {
            let uri = if lut_uri.is_empty() {
                String::new()
            } else {
                dir.path().join(lut_uri).display().to_string()
            };
            let filtered = render_test_image_source_with_filters(
                &source_path.display().to_string(),
                vec![test_filter(
                    "filter-lut",
                    SceneSourceFilterKind::Lut,
                    10,
                    true,
                    serde_json::json!({ "lut_uri": uri, "strength": 1 }),
                )],
            );
            let filter = &filtered.input_frames[0].filters[0];

            assert_eq!(filter.status, SoftwareCompositorFilterStatus::Error);
            assert!(filter.status_detail.contains(expected_detail));
            assert_eq!(
                baseline.input_frames[0].checksum,
                filtered.input_frames[0].checksum
            );
        }
    }

    #[test]
    fn software_compositor_applies_mask_and_lut_in_deterministic_order() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("source.png");
        let mask_path = dir.path().join("mask.png");
        let lut_path = dir.path().join("warm.cube");
        write_test_image(&source_path, ImageFormat::Png, [64, 128, 192, 255]);
        write_test_image(&mask_path, ImageFormat::Png, [255, 255, 255, 255]);
        write_cube_lut(&lut_path, |red, green, blue| {
            [(red + 0.2).min(1.0), green * 0.85, blue * 0.75]
        });
        let mask = test_filter(
            "filter-mask",
            SceneSourceFilterKind::MaskBlend,
            20,
            true,
            serde_json::json!({
                "mask_uri": mask_path.display().to_string(),
                "blend_mode": "multiply"
            }),
        );
        let lut = test_filter(
            "filter-lut",
            SceneSourceFilterKind::Lut,
            10,
            true,
            serde_json::json!({ "lut_uri": lut_path.display().to_string(), "strength": 1 }),
        );

        let reversed = render_test_image_source_with_filters(
            &source_path.display().to_string(),
            vec![mask.clone(), lut.clone()],
        );
        let ordered = render_test_image_source_with_filters(
            &source_path.display().to_string(),
            vec![lut, mask],
        );

        assert_eq!(
            reversed.input_frames[0].checksum,
            ordered.input_frames[0].checksum
        );
        assert_eq!(reversed.input_frames[0].filters[0].id, "filter-lut");
        assert_eq!(reversed.input_frames[0].filters[1].id, "filter-mask");
    }

    #[test]
    fn software_compositor_reports_audio_filters_deferred_without_mutating_pixels() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("solid.png");
        write_test_image(&path, ImageFormat::Png, [255, 0, 0, 255]);

        let baseline = render_test_image_source(&path.display().to_string(), None);
        let filtered = render_test_image_source_with_filters(
            &path.display().to_string(),
            vec![test_filter(
                "filter-audio",
                SceneSourceFilterKind::AudioGain,
                10,
                true,
                serde_json::json!({ "gain_db": 3 }),
            )],
        );
        let filter = &filtered.input_frames[0].filters[0];

        assert_eq!(filter.status, SoftwareCompositorFilterStatus::Deferred);
        assert_eq!(
            baseline.input_frames[0].checksum,
            filtered.input_frames[0].checksum
        );
        assert!(filtered
            .frame
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("deferred")));
    }

    #[test]
    fn software_compositor_applies_filters_to_text_source_pixels() {
        let baseline = render_test_text_source("VAEX", "Inter", "#808080", "center", 42.0);
        let filtered = render_test_text_source_with_filters(
            "VAEX",
            vec![test_filter(
                "filter-color",
                SceneSourceFilterKind::ColorCorrection,
                10,
                true,
                serde_json::json!({
                    "brightness": 0.2,
                    "contrast": 1.0,
                    "saturation": 1.0,
                    "gamma": 1.0
                }),
            )],
        );

        assert_eq!(
            filtered.input_frames[0].filters[0].status,
            SoftwareCompositorFilterStatus::Applied
        );
        assert_ne!(
            baseline.input_frames[0].checksum,
            filtered.input_frames[0].checksum
        );
        assert_eq!(
            filtered.input_frames[0].text.as_ref().unwrap().status,
            SoftwareCompositorTextStatus::Rendered
        );
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

    fn write_split_image(path: &Path, left: [u8; 4], right: [u8; 4]) {
        let mut image = RgbaImage::new(4, 2);
        for y in 0..2 {
            for x in 0..4 {
                image.put_pixel(x, y, Rgba(if x < 2 { left } else { right }));
            }
        }
        image.save_with_format(path, ImageFormat::Png).unwrap();
    }

    fn write_checker_image(path: &Path) {
        let mut image = RgbaImage::new(5, 5);
        for y in 0..5 {
            for x in 0..5 {
                let value = if (x + y) % 2 == 0 { 255 } else { 0 };
                image.put_pixel(x, y, Rgba([value, value, value, 255]));
            }
        }
        image.save_with_format(path, ImageFormat::Png).unwrap();
    }

    fn write_soft_spot_image(path: &Path) {
        let mut image = RgbaImage::from_pixel(5, 5, Rgba([96, 96, 96, 255]));
        image.put_pixel(2, 2, Rgba([168, 168, 168, 255]));
        image.save_with_format(path, ImageFormat::Png).unwrap();
    }

    fn write_cube_lut(path: &Path, map: impl Fn(f64, f64, f64) -> [f64; 3]) {
        let mut contents =
            String::from("TITLE \"test\"\nLUT_3D_SIZE 2\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\n");
        for blue in [0.0, 1.0] {
            for green in [0.0, 1.0] {
                for red in [0.0, 1.0] {
                    let [mapped_red, mapped_green, mapped_blue] = map(red, green, blue);
                    contents.push_str(&format!(
                        "{mapped_red:.6} {mapped_green:.6} {mapped_blue:.6}\n"
                    ));
                }
            }
        }
        fs::write(path, contents).unwrap();
    }

    fn write_test_video(ffmpeg_path: &Path, path: &Path, color: &str) -> bool {
        let status = Command::new(ffmpeg_path)
            .arg("-v")
            .arg("error")
            .arg("-y")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg(format!("color=c={color}:s=8x8:d=1:r=30"))
            .arg("-frames:v")
            .arg("30")
            .arg("-c:v")
            .arg("mpeg4")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg(path)
            .status();
        matches!(status, Ok(status) if status.success())
    }

    fn write_browser_fixture(dir: &Path, filename: &str, background: &str) -> String {
        let path = dir.join(filename);
        fs::write(
            &path,
            format!(
                r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <style>
      html, body {{ margin: 0; width: 100%; height: 100%; overflow: hidden; }}
      body {{ background: {background}; }}
      .label {{ position: fixed; inset: 16px; color: white; font: 700 24px system-ui; }}
    </style>
  </head>
  <body><div class="label">VaexCore Browser Overlay</div></body>
</html>"#
            ),
        )
        .unwrap();
        format!("file://{}", path.display())
    }

    fn render_test_browser_source(
        url: &str,
        custom_css: Option<&str>,
    ) -> SoftwareCompositorRenderResult {
        render_test_scene_with_target(test_browser_scene(url, custom_css), 128, 72)
    }

    fn test_browser_scene(url: &str, custom_css: Option<&str>) -> Scene {
        Scene {
            id: "scene-browser".to_string(),
            name: "Browser Scene".to_string(),
            canvas: crate::SceneCanvas {
                width: 128,
                height: 72,
                background_color: "#050711".to_string(),
            },
            sources: vec![SceneSource {
                id: "source-browser".to_string(),
                name: "Browser".to_string(),
                kind: SceneSourceKind::BrowserOverlay,
                position: ScenePoint { x: 0.0, y: 0.0 },
                size: SceneSize {
                    width: 128.0,
                    height: 72.0,
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
                    "url": url,
                    "viewport": { "width": 128, "height": 72 },
                    "custom_css": custom_css
                }),
            }],
        }
    }

    fn render_test_image_source(
        asset_uri: &str,
        media_type: Option<&str>,
    ) -> SoftwareCompositorRenderResult {
        render_test_scene(test_image_scene(asset_uri, media_type))
    }

    fn render_test_image_source_with_filters(
        asset_uri: &str,
        filters: Vec<SceneSourceFilter>,
    ) -> SoftwareCompositorRenderResult {
        let mut scene = test_image_scene(asset_uri, None);
        scene.sources[0].filters = filters;
        render_test_scene(scene)
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

    fn render_test_text_source_with_filters(
        text: &str,
        filters: Vec<SceneSourceFilter>,
    ) -> SoftwareCompositorRenderResult {
        let scene = Scene {
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
                filters,
                config: serde_json::json!({
                    "text": text,
                    "font_family": "Inter",
                    "font_size": 42,
                    "color": "#808080",
                    "align": "center"
                }),
            }],
        };
        render_test_scene_with_target(scene, 160, 80)
    }

    fn test_filter(
        id: &str,
        kind: SceneSourceFilterKind,
        order: i32,
        enabled: bool,
        config: serde_json::Value,
    ) -> SceneSourceFilter {
        SceneSourceFilter {
            id: id.to_string(),
            name: id.replace("filter-", "").replace('-', " "),
            kind,
            enabled,
            order,
            config,
        }
    }

    fn render_test_scene(scene: Scene) -> SoftwareCompositorRenderResult {
        render_test_scene_with_target(scene, 8, 8)
    }

    fn render_test_scene_with_target(
        scene: Scene,
        width: u32,
        height: u32,
    ) -> SoftwareCompositorRenderResult {
        render_test_scene_with_target_at_frame(scene, width, height, 0)
    }

    fn render_test_scene_with_target_at_frame(
        scene: Scene,
        width: u32,
        height: u32,
        frame_index: u64,
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
        render_software_compositor_frame(&plan, frame_index)
    }

    fn alpha_pixel_count(input: &SoftwareCompositorInputFrame) -> usize {
        input
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 0)
            .count()
    }

    fn input_test_pixel(input: &SoftwareCompositorInputFrame, x: usize, y: usize) -> [u8; 4] {
        let offset = (y * input.width as usize + x) * 4;
        [
            input.pixels[offset],
            input.pixels[offset + 1],
            input.pixels[offset + 2],
            input.pixels[offset + 3],
        ]
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
