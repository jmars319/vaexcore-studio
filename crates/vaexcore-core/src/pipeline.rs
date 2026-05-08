use crate::{CaptureSourceSelection, MediaProfile, Scene, StreamDestination};
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
        MediaPipelineConfig {
            version: 1,
            dry_run: self.dry_run,
            intent: self.intent,
            capture_sources: self.capture_sources,
            active_scene: self.active_scene,
            recording_profile: self.recording_profile,
            stream_destinations: self.stream_destinations,
        }
    }
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
