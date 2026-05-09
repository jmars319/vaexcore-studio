use crate::{
    build_audio_mixer_plan, build_capture_frame_plan, build_compositor_graph,
    build_compositor_render_plan, build_performance_telemetry_plan, compositor_render_target,
    AudioMixerPlan, CaptureFramePlan, CaptureSourceSelection, CompositorFrameFormat,
    CompositorGraph, CompositorRenderPlan, CompositorRenderTarget, CompositorRenderTargetKind,
    CompositorScaleMode, EncoderPreference, MediaProfile, PerformanceTelemetryPlan, PlatformKind,
    RecordingContainer, Scene, StreamDestination,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineIntent {
    Recording,
    Stream,
    RecordingAndStream,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RenderTargetProfile {
    pub id: String,
    pub name: String,
    pub kind: CompositorRenderTargetKind,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub frame_format: CompositorFrameFormat,
    pub scale_mode: CompositorScaleMode,
    pub enabled: bool,
    pub encoder_preference: EncoderPreference,
    pub bitrate_kbps: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RecordingTargetContract {
    pub id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub render_target_id: String,
    pub output_folder: String,
    pub filename_pattern: String,
    pub container: RecordingContainer,
    pub resolution: crate::Resolution,
    pub framerate: u32,
    pub bitrate_kbps: u32,
    pub encoder_preference: EncoderPreference,
    pub output_path_preview: String,
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct StreamingTargetContract {
    pub id: String,
    pub destination_id: String,
    pub destination_name: String,
    pub platform: PlatformKind,
    pub render_target_id: String,
    pub ingest_url: String,
    pub stream_key_required: bool,
    pub has_stream_key: bool,
    pub bandwidth_test: bool,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub bitrate_kbps: u32,
    pub encoder_preference: EncoderPreference,
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct OutputPreflightPlan {
    pub version: u32,
    pub intent: PipelineIntent,
    pub active_scene_id: Option<String>,
    pub active_scene_name: Option<String>,
    pub render_targets: Vec<RenderTargetProfile>,
    pub recording_target: Option<RecordingTargetContract>,
    pub streaming_targets: Vec<StreamingTargetContract>,
    pub validation: OutputPreflightValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct OutputPreflightValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MediaPipelineConfig {
    pub version: u32,
    pub dry_run: bool,
    pub intent: PipelineIntent,
    pub capture_sources: Vec<CaptureSourceSelection>,
    #[serde(default)]
    pub active_scene: Option<Scene>,
    #[serde(default)]
    pub capture_frame_plan: Option<CaptureFramePlan>,
    #[serde(default)]
    pub audio_mixer_plan: Option<AudioMixerPlan>,
    #[serde(default)]
    pub compositor_graph: Option<CompositorGraph>,
    #[serde(default)]
    pub compositor_render_plan: Option<CompositorRenderPlan>,
    #[serde(default)]
    pub performance_telemetry_plan: Option<PerformanceTelemetryPlan>,
    #[serde(default)]
    pub output_preflight_plan: Option<OutputPreflightPlan>,
    pub recording_profile: Option<MediaProfile>,
    pub stream_destinations: Vec<StreamDestination>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MediaPipelinePlanRequest {
    pub dry_run: bool,
    pub intent: PipelineIntent,
    pub capture_sources: Vec<CaptureSourceSelection>,
    #[serde(default)]
    pub active_scene: Option<Scene>,
    pub recording_profile: Option<MediaProfile>,
    pub stream_destinations: Vec<StreamDestination>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MediaPipelineStep {
    pub id: String,
    pub label: String,
    pub status: PipelineStepStatus,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStepStatus {
    Ready,
    Warning,
    Blocked,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MediaPipelinePlan {
    pub pipeline_name: String,
    pub dry_run: bool,
    pub ready: bool,
    pub config: MediaPipelineConfig,
    pub steps: Vec<MediaPipelineStep>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MediaPipelineValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl MediaPipelinePlanRequest {
    pub fn into_config(self) -> MediaPipelineConfig {
        let capture_frame_plan = self.active_scene.as_ref().map(build_capture_frame_plan);
        let audio_mixer_plan = self.active_scene.as_ref().map(build_audio_mixer_plan);
        let compositor_graph = self.active_scene.as_ref().map(build_compositor_graph);
        let compositor_render_plan = compositor_graph.as_ref().map(|graph| {
            build_compositor_render_plan(
                graph,
                default_compositor_render_targets(
                    &self.intent,
                    graph,
                    self.recording_profile.as_ref(),
                    &self.stream_destinations,
                ),
            )
        });
        let performance_telemetry_plan = compositor_render_plan
            .as_ref()
            .map(build_performance_telemetry_plan);
        let output_preflight_plan = Some(build_output_preflight_plan(
            &self.intent,
            self.active_scene.as_ref(),
            compositor_render_plan.as_ref(),
            self.recording_profile.as_ref(),
            &self.stream_destinations,
        ));

        MediaPipelineConfig {
            version: 1,
            dry_run: self.dry_run,
            intent: self.intent,
            capture_sources: self.capture_sources,
            active_scene: self.active_scene,
            capture_frame_plan,
            audio_mixer_plan,
            compositor_graph,
            compositor_render_plan,
            performance_telemetry_plan,
            output_preflight_plan,
            recording_profile: self.recording_profile,
            stream_destinations: self.stream_destinations,
        }
    }
}

fn default_compositor_render_targets(
    intent: &PipelineIntent,
    graph: &CompositorGraph,
    recording_profile: Option<&MediaProfile>,
    stream_destinations: &[StreamDestination],
) -> Vec<CompositorRenderTarget> {
    let mut targets = vec![
        compositor_render_target(
            "target-preview",
            "Preview",
            CompositorRenderTargetKind::Preview,
            graph.output.width,
            graph.output.height,
            recording_profile
                .map(|profile| profile.framerate)
                .unwrap_or(60),
        ),
        compositor_render_target(
            "target-program",
            "Program",
            CompositorRenderTargetKind::Program,
            graph.output.width,
            graph.output.height,
            recording_profile
                .map(|profile| profile.framerate)
                .unwrap_or(60),
        ),
    ];

    if matches!(
        intent,
        PipelineIntent::Recording | PipelineIntent::RecordingAndStream
    ) {
        targets.push(compositor_render_target(
            "target-recording",
            "Recording Output",
            CompositorRenderTargetKind::Recording,
            recording_profile
                .map(|profile| profile.resolution.width)
                .unwrap_or(graph.output.width),
            recording_profile
                .map(|profile| profile.resolution.height)
                .unwrap_or(graph.output.height),
            recording_profile
                .map(|profile| profile.framerate)
                .unwrap_or(60),
        ));
    }

    if matches!(
        intent,
        PipelineIntent::Stream | PipelineIntent::RecordingAndStream
    ) {
        if stream_destinations.is_empty() {
            targets.push(compositor_render_target(
                "target-stream",
                "Stream Output",
                CompositorRenderTargetKind::Stream,
                graph.output.width,
                graph.output.height,
                recording_profile
                    .map(|profile| profile.framerate)
                    .unwrap_or(60),
            ));
        } else {
            targets.extend(stream_destinations.iter().map(|destination| {
                compositor_render_target(
                    format!("target-stream-{}", destination.id),
                    format!("Stream Output: {}", destination.name),
                    CompositorRenderTargetKind::Stream,
                    graph.output.width,
                    graph.output.height,
                    recording_profile
                        .map(|profile| profile.framerate)
                        .unwrap_or(60),
                )
            }));
        }
    }

    targets
}

impl MediaPipelinePlan {
    pub fn validation(&self) -> MediaPipelineValidation {
        MediaPipelineValidation {
            ready: self.ready,
            warnings: self.warnings.clone(),
            errors: self.errors.clone(),
        }
    }
}

pub fn build_output_preflight_plan(
    intent: &PipelineIntent,
    active_scene: Option<&Scene>,
    render_plan: Option<&CompositorRenderPlan>,
    recording_profile: Option<&MediaProfile>,
    stream_destinations: &[StreamDestination],
) -> OutputPreflightPlan {
    let render_targets = render_plan
        .map(|plan| {
            plan.targets
                .iter()
                .map(|target| render_target_profile(target, recording_profile))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let recording_target = if matches!(
        intent,
        PipelineIntent::Recording | PipelineIntent::RecordingAndStream
    ) {
        recording_profile.map(|profile| {
            recording_target_contract(profile, preferred_target_id(&render_targets, "recording"))
        })
    } else {
        None
    };
    let streaming_targets = if matches!(
        intent,
        PipelineIntent::Stream | PipelineIntent::RecordingAndStream
    ) {
        stream_destinations
            .iter()
            .filter(|destination| destination.enabled)
            .map(|destination| {
                streaming_target_contract(
                    destination,
                    recording_profile,
                    stream_target_profile(&render_targets, destination),
                )
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut plan = OutputPreflightPlan {
        version: 1,
        intent: intent.clone(),
        active_scene_id: active_scene.map(|scene| scene.id.clone()),
        active_scene_name: active_scene.map(|scene| scene.name.clone()),
        render_targets,
        recording_target,
        streaming_targets,
        validation: OutputPreflightValidation {
            ready: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        },
    };
    plan.validation = validate_output_preflight_plan(&plan);
    plan
}

pub fn validate_output_preflight_plan(plan: &OutputPreflightPlan) -> OutputPreflightValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut target_ids = std::collections::HashSet::new();
    let needs_recording = matches!(
        plan.intent,
        PipelineIntent::Recording | PipelineIntent::RecordingAndStream
    );
    let needs_stream = matches!(
        plan.intent,
        PipelineIntent::Stream | PipelineIntent::RecordingAndStream
    );

    if plan.version == 0 {
        errors.push("output preflight plan version must be greater than zero".to_string());
    }
    if plan.active_scene_id.is_none() {
        warnings.push("output preflight has no active scene".to_string());
    }
    if plan.render_targets.is_empty() {
        errors.push("output preflight requires at least one render target".to_string());
    }

    for target in &plan.render_targets {
        if !target_ids.insert(target.id.as_str()) {
            errors.push(format!("duplicate render target profile \"{}\"", target.id));
        }
        if target.id.trim().is_empty() {
            errors.push("render target profile id is required".to_string());
        }
        if target.name.trim().is_empty() {
            errors.push(format!(
                "render target profile \"{}\" name is required",
                target.id
            ));
        }
        if target.width == 0 || target.height == 0 {
            errors.push(format!(
                "render target profile \"{}\" dimensions must be greater than zero",
                target.id
            ));
        }
        if target.framerate == 0 {
            errors.push(format!(
                "render target profile \"{}\" framerate must be greater than zero",
                target.id
            ));
        }
        if !target.enabled {
            warnings.push(format!(
                "render target profile \"{}\" is disabled",
                target.id
            ));
        }
        validate_encoder_preference(
            &target.encoder_preference,
            &format!("render target profile \"{}\"", target.id),
            &mut warnings,
            &mut errors,
        );
    }

    if needs_recording {
        match &plan.recording_target {
            Some(recording) => {
                warnings.extend(recording.warnings.iter().cloned());
                errors.extend(recording.errors.iter().cloned());
                if !target_ids.contains(recording.render_target_id.as_str()) {
                    errors.push(format!(
                        "recording target references unknown render target \"{}\"",
                        recording.render_target_id
                    ));
                }
            }
            None => {
                errors.push("recording output preflight requires a recording target".to_string())
            }
        }
    }

    if needs_stream {
        if plan.streaming_targets.is_empty() {
            errors
                .push("stream output preflight requires at least one streaming target".to_string());
        }
        for target in &plan.streaming_targets {
            warnings.extend(target.warnings.iter().cloned());
            errors.extend(target.errors.iter().cloned());
            if !target_ids.contains(target.render_target_id.as_str()) {
                errors.push(format!(
                    "streaming target \"{}\" references unknown render target \"{}\"",
                    target.destination_name, target.render_target_id
                ));
            }
        }
    }

    OutputPreflightValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

fn render_target_profile(
    target: &CompositorRenderTarget,
    recording_profile: Option<&MediaProfile>,
) -> RenderTargetProfile {
    RenderTargetProfile {
        id: target.id.clone(),
        name: target.name.clone(),
        kind: target.kind.clone(),
        width: target.width,
        height: target.height,
        framerate: target.framerate,
        frame_format: target.frame_format.clone(),
        scale_mode: target.scale_mode.clone(),
        enabled: target.enabled,
        encoder_preference: recording_profile
            .map(|profile| profile.encoder_preference.clone())
            .unwrap_or(EncoderPreference::Auto),
        bitrate_kbps: target_bitrate_kbps(target.kind.clone(), recording_profile),
    }
}

fn recording_target_contract(
    profile: &MediaProfile,
    render_target_id: String,
) -> RecordingTargetContract {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if profile.output_folder.trim().is_empty() {
        errors.push("recording output folder is required".to_string());
    }
    if profile.filename_pattern.trim().is_empty() {
        errors.push("recording filename pattern is required".to_string());
    }
    if profile.filename_pattern.contains('/') || profile.filename_pattern.contains('\\') {
        warnings.push("recording filename pattern includes path separators".to_string());
    }
    if profile.resolution.width == 0 || profile.resolution.height == 0 {
        errors.push("recording resolution must be greater than zero".to_string());
    }
    if profile.framerate == 0 {
        errors.push("recording framerate must be greater than zero".to_string());
    }
    if profile.bitrate_kbps == 0 {
        errors.push("recording bitrate must be greater than zero".to_string());
    }
    validate_encoder_preference(
        &profile.encoder_preference,
        "recording target",
        &mut warnings,
        &mut errors,
    );

    RecordingTargetContract {
        id: format!("recording-target-{}", profile.id),
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        render_target_id,
        output_folder: profile.output_folder.clone(),
        filename_pattern: profile.filename_pattern.clone(),
        container: profile.container.clone(),
        resolution: profile.resolution.clone(),
        framerate: profile.framerate,
        bitrate_kbps: profile.bitrate_kbps,
        encoder_preference: profile.encoder_preference.clone(),
        output_path_preview: recording_output_path_preview(profile),
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

fn streaming_target_contract(
    destination: &StreamDestination,
    recording_profile: Option<&MediaProfile>,
    render_target: Option<&RenderTargetProfile>,
) -> StreamingTargetContract {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let encoder_preference = recording_profile
        .map(|profile| profile.encoder_preference.clone())
        .unwrap_or(EncoderPreference::Auto);

    if destination.ingest_url.trim().is_empty() {
        errors.push(format!(
            "stream destination \"{}\" requires an ingest URL",
            destination.name
        ));
    }
    if destination.stream_key_ref.is_none() {
        warnings.push(format!(
            "stream destination \"{}\" has no stored stream key",
            destination.name
        ));
    }
    validate_encoder_preference(
        &encoder_preference,
        &format!("streaming target \"{}\"", destination.name),
        &mut warnings,
        &mut errors,
    );

    StreamingTargetContract {
        id: format!("streaming-target-{}", destination.id),
        destination_id: destination.id.clone(),
        destination_name: destination.name.clone(),
        platform: destination.platform.clone(),
        render_target_id: render_target
            .map(|target| target.id.clone())
            .unwrap_or_else(|| "target-stream".to_string()),
        ingest_url: destination.ingest_url.clone(),
        stream_key_required: true,
        has_stream_key: destination.stream_key_ref.is_some(),
        bandwidth_test: false,
        width: render_target.map(|target| target.width).unwrap_or(1920),
        height: render_target.map(|target| target.height).unwrap_or(1080),
        framerate: render_target.map(|target| target.framerate).unwrap_or(60),
        bitrate_kbps: recording_profile
            .map(|profile| profile.bitrate_kbps)
            .unwrap_or(6_000),
        encoder_preference,
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

fn stream_target_profile<'a>(
    targets: &'a [RenderTargetProfile],
    destination: &StreamDestination,
) -> Option<&'a RenderTargetProfile> {
    targets
        .iter()
        .find(|target| {
            target.kind == CompositorRenderTargetKind::Stream
                && target.id == format!("target-stream-{}", destination.id)
        })
        .or_else(|| {
            targets
                .iter()
                .find(|target| target.kind == CompositorRenderTargetKind::Stream)
        })
        .or_else(|| {
            targets
                .iter()
                .find(|target| target.kind == CompositorRenderTargetKind::Program)
        })
}

fn preferred_target_id(targets: &[RenderTargetProfile], kind: &str) -> String {
    let target_kind = match kind {
        "recording" => CompositorRenderTargetKind::Recording,
        "stream" => CompositorRenderTargetKind::Stream,
        _ => CompositorRenderTargetKind::Program,
    };
    targets
        .iter()
        .find(|target| target.kind == target_kind)
        .or_else(|| {
            targets
                .iter()
                .find(|target| target.kind == CompositorRenderTargetKind::Program)
        })
        .or_else(|| targets.first())
        .map(|target| target.id.clone())
        .unwrap_or_else(|| format!("target-{kind}"))
}

fn target_bitrate_kbps(
    kind: CompositorRenderTargetKind,
    recording_profile: Option<&MediaProfile>,
) -> Option<u32> {
    match kind {
        CompositorRenderTargetKind::Recording | CompositorRenderTargetKind::Stream => Some(
            recording_profile
                .map(|profile| profile.bitrate_kbps)
                .unwrap_or(6_000),
        ),
        CompositorRenderTargetKind::Preview | CompositorRenderTargetKind::Program => None,
    }
}

fn validate_encoder_preference(
    encoder: &EncoderPreference,
    label: &str,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    match encoder {
        EncoderPreference::Named(name) if name.trim().is_empty() => {
            errors.push(format!("{label} named encoder cannot be empty"));
        }
        EncoderPreference::Hardware => {
            warnings.push(format!(
                "{label} requests hardware encoding; validate availability on target machine"
            ));
        }
        EncoderPreference::Auto | EncoderPreference::Software | EncoderPreference::Named(_) => {}
    }
}

fn recording_output_path_preview(profile: &MediaProfile) -> String {
    let extension = match profile.container {
        RecordingContainer::Mkv => "mkv",
        RecordingContainer::Mp4 => "mp4",
    };
    let filename = profile
        .filename_pattern
        .replace("{date}", "2026-05-09")
        .replace("{time}", "12-00-00")
        .replace("{profile}", &slug(&profile.name));
    format!("{}/{}.{}", profile.output_folder, filename, extension)
}

fn slug(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }

    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        default_capture_sources, now_utc, MediaProfileInput, SecretRef, StreamDestination,
    };

    #[test]
    fn output_preflight_plan_describes_recording_and_stream_targets() {
        let collection = crate::SceneCollection::default_collection(now_utc());
        let scene = collection.active_scene().unwrap().clone();
        let profile = MediaProfile::default_local();
        let destination = StreamDestination {
            id: "stream-dest-main".to_string(),
            name: "Twitch Main".to_string(),
            platform: PlatformKind::Twitch,
            ingest_url: "rtmp://live.twitch.tv/app".to_string(),
            stream_key_ref: Some(SecretRef {
                provider: "test".to_string(),
                id: "stream-key".to_string(),
            }),
            enabled: true,
            created_at: now_utc(),
            updated_at: now_utc(),
        };

        let config = MediaPipelinePlanRequest {
            dry_run: true,
            intent: PipelineIntent::RecordingAndStream,
            capture_sources: default_capture_sources(),
            active_scene: Some(scene),
            recording_profile: Some(profile),
            stream_destinations: vec![destination],
        }
        .into_config();
        let preflight = config.output_preflight_plan.unwrap();

        assert!(preflight.validation.ready, "{:?}", preflight.validation);
        assert_eq!(preflight.render_targets.len(), 4);
        assert_eq!(
            preflight.recording_target.unwrap().render_target_id,
            "target-recording"
        );
        assert_eq!(preflight.streaming_targets.len(), 1);
        assert_eq!(
            preflight.streaming_targets[0].render_target_id,
            "target-stream-stream-dest-main"
        );
    }

    #[test]
    fn output_preflight_validation_blocks_invalid_encoder_contracts() {
        let mut profile = MediaProfile::from_input(MediaProfileInput::default());
        profile.encoder_preference = EncoderPreference::Named(" ".to_string());
        let plan = build_output_preflight_plan(
            &PipelineIntent::Recording,
            None,
            None,
            Some(&profile),
            &[],
        );

        assert!(!plan.validation.ready);
        assert!(plan
            .validation
            .errors
            .iter()
            .any(|error| error.contains("named encoder")));
    }
}
