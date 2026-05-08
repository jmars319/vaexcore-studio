use crate::{
    build_audio_mixer_plan, build_capture_frame_plan, build_compositor_graph,
    build_compositor_render_plan, build_performance_telemetry_plan, compositor_render_target,
    AudioMixerPlan, CaptureFramePlan, CaptureSourceSelection, CompositorGraph,
    CompositorRenderPlan, CompositorRenderTarget, CompositorRenderTargetKind, MediaProfile,
    PerformanceTelemetryPlan, Scene, StreamDestination,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineIntent {
    Recording,
    Stream,
    RecordingAndStream,
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
