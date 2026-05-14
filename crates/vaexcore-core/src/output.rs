use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    MediaPipelinePlan, MediaPipelineValidation, OutputPreflightPlan, PipelineIntent,
    SceneOutputReadyDiagnostic,
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutputJobState {
    Idle,
    Preparing,
    Ready,
    Blocked,
    Cancelled,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct OutputJobPrepareRequest {
    #[serde(default)]
    pub recording_profile_id: Option<String>,
    #[serde(default)]
    pub stream_destination_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OutputJobSummary {
    pub id: String,
    pub state: OutputJobState,
    pub detail: String,
    pub active_scene_id: Option<String>,
    pub active_scene_name: Option<String>,
    pub recording_profile_id: Option<String>,
    pub recording_profile_name: Option<String>,
    pub output_path_preview: Option<String>,
    pub stream_destination_count: usize,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OutputJob {
    pub version: u32,
    pub id: String,
    pub state: OutputJobState,
    pub detail: String,
    pub prepared_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub request: OutputJobPrepareRequest,
    pub intent: PipelineIntent,
    pub active_scene_id: Option<String>,
    pub active_scene_name: Option<String>,
    pub recording_profile_id: Option<String>,
    pub recording_profile_name: Option<String>,
    pub stream_destination_ids: Vec<String>,
    pub stream_destination_names: Vec<String>,
    pub output_path_preview: Option<String>,
    pub scene_output_ready: bool,
    pub media_pipeline_ready: bool,
    pub output_preflight_ready: bool,
    pub recording_target_ready: bool,
    pub stream_targets_ready: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub pipeline_validation: MediaPipelineValidation,
    pub output_preflight_plan: Option<OutputPreflightPlan>,
}

impl OutputJob {
    pub fn idle(now: DateTime<Utc>) -> Self {
        Self {
            version: 1,
            id: "output-job-idle".to_string(),
            state: OutputJobState::Idle,
            detail: "No output job has been prepared.".to_string(),
            prepared_at: now,
            updated_at: now,
            request: OutputJobPrepareRequest::default(),
            intent: PipelineIntent::Recording,
            active_scene_id: None,
            active_scene_name: None,
            recording_profile_id: None,
            recording_profile_name: None,
            stream_destination_ids: Vec::new(),
            stream_destination_names: Vec::new(),
            output_path_preview: None,
            scene_output_ready: false,
            media_pipeline_ready: false,
            output_preflight_ready: false,
            recording_target_ready: false,
            stream_targets_ready: true,
            blockers: Vec::new(),
            warnings: Vec::new(),
            pipeline_validation: MediaPipelineValidation {
                ready: false,
                warnings: Vec::new(),
                errors: Vec::new(),
            },
            output_preflight_plan: None,
        }
    }

    pub fn prepared(
        id: String,
        request: OutputJobPrepareRequest,
        pipeline: &MediaPipelinePlan,
        scene_output_ready: &SceneOutputReadyDiagnostic,
        now: DateTime<Utc>,
    ) -> Self {
        let pipeline_validation = pipeline.validation();
        let output_preflight_plan = pipeline.config.output_preflight_plan.clone();
        let output_preflight_ready = output_preflight_plan
            .as_ref()
            .is_some_and(|plan| plan.validation.ready);
        let recording_target = output_preflight_plan
            .as_ref()
            .and_then(|plan| plan.recording_target.as_ref());
        let recording_target_ready = recording_target.is_some_and(|target| target.ready);
        let stream_targets = output_preflight_plan
            .as_ref()
            .map(|plan| plan.streaming_targets.as_slice())
            .unwrap_or(&[]);
        let stream_targets_ready = stream_targets.iter().all(|target| target.ready);

        let mut blockers = Vec::new();
        let mut warnings = Vec::new();
        if !scene_output_ready.ready {
            push_all_or_message(
                &mut blockers,
                &scene_output_ready.blockers,
                "Scene output readiness is blocked.",
            );
        }
        if !pipeline_validation.ready {
            push_all_or_message(
                &mut blockers,
                &pipeline_validation.errors,
                "Media pipeline validation is blocked.",
            );
        }
        match &output_preflight_plan {
            Some(plan) if !plan.validation.ready => push_all_or_message(
                &mut blockers,
                &plan.validation.errors,
                "Output preflight validation is blocked.",
            ),
            None => blockers.push("Output preflight plan is unavailable.".to_string()),
            _ => {}
        }
        match recording_target {
            Some(target) if !target.ready => push_all_or_message(
                &mut blockers,
                &target.errors,
                "Recording target is not ready.",
            ),
            None => blockers.push("Recording target is missing.".to_string()),
            _ => {}
        }
        for target in stream_targets.iter().filter(|target| !target.ready) {
            push_all_or_message(
                &mut blockers,
                &target.errors,
                &format!("Stream target {} is not ready.", target.destination_name),
            );
        }

        warnings.extend(scene_output_ready.warnings.iter().cloned());
        warnings.extend(pipeline_validation.warnings.iter().cloned());
        if let Some(plan) = &output_preflight_plan {
            warnings.extend(plan.validation.warnings.iter().cloned());
            if let Some(target) = &plan.recording_target {
                warnings.extend(target.warnings.iter().cloned());
            }
            for target in &plan.streaming_targets {
                warnings.extend(target.warnings.iter().cloned());
            }
        }
        sort_and_dedup(&mut blockers);
        sort_and_dedup(&mut warnings);

        let ready = scene_output_ready.ready
            && pipeline_validation.ready
            && output_preflight_ready
            && recording_target_ready
            && stream_targets_ready
            && blockers.is_empty();
        let state = if ready {
            OutputJobState::Ready
        } else {
            OutputJobState::Blocked
        };
        let detail = if ready {
            "Dry-run output job is prepared and ready.".to_string()
        } else {
            format!("{} output job blocker(s) must be resolved.", blockers.len())
        };
        let recording_profile = pipeline.config.recording_profile.as_ref();

        Self {
            version: 1,
            id,
            state,
            detail,
            prepared_at: now,
            updated_at: now,
            request,
            intent: pipeline.config.intent.clone(),
            active_scene_id: pipeline
                .config
                .active_scene
                .as_ref()
                .map(|scene| scene.id.clone()),
            active_scene_name: pipeline
                .config
                .active_scene
                .as_ref()
                .map(|scene| scene.name.clone()),
            recording_profile_id: recording_profile.map(|profile| profile.id.clone()),
            recording_profile_name: recording_profile.map(|profile| profile.name.clone()),
            stream_destination_ids: pipeline
                .config
                .stream_destinations
                .iter()
                .map(|destination| destination.id.clone())
                .collect(),
            stream_destination_names: pipeline
                .config
                .stream_destinations
                .iter()
                .map(|destination| destination.name.clone())
                .collect(),
            output_path_preview: recording_target.map(|target| target.output_path_preview.clone()),
            scene_output_ready: scene_output_ready.ready,
            media_pipeline_ready: pipeline_validation.ready,
            output_preflight_ready,
            recording_target_ready,
            stream_targets_ready,
            blockers,
            warnings,
            pipeline_validation,
            output_preflight_plan,
        }
    }

    pub fn cancelled(mut self, now: DateTime<Utc>) -> Self {
        self.state = OutputJobState::Cancelled;
        self.detail = "Prepared output job was cancelled.".to_string();
        self.updated_at = now;
        self
    }

    pub fn summary(&self) -> OutputJobSummary {
        OutputJobSummary {
            id: self.id.clone(),
            state: self.state.clone(),
            detail: self.detail.clone(),
            active_scene_id: self.active_scene_id.clone(),
            active_scene_name: self.active_scene_name.clone(),
            recording_profile_id: self.recording_profile_id.clone(),
            recording_profile_name: self.recording_profile_name.clone(),
            output_path_preview: self.output_path_preview.clone(),
            stream_destination_count: self.stream_destination_ids.len(),
            blockers: self.blockers.clone(),
            warnings: self.warnings.clone(),
            updated_at: self.updated_at,
        }
    }
}

fn push_all_or_message(target: &mut Vec<String>, source: &[String], message: &str) {
    if source.is_empty() {
        target.push(message.to_string());
    } else {
        target.extend(source.iter().cloned());
    }
}

fn sort_and_dedup(values: &mut Vec<String>) {
    values.retain(|value| !value.trim().is_empty());
    values.sort();
    values.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        default_capture_sources, DesignerRuntimeReadinessState, MediaPipelinePlanRequest,
        MediaProfile, SceneCollection, StreamDestinationInput,
    };

    #[test]
    fn prepared_output_job_becomes_ready_when_pipeline_contracts_are_ready() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap().clone();
        let profile = MediaProfile::default_local();
        let destination = crate::StreamDestination::from_input(
            StreamDestinationInput {
                name: "Dry Run".to_string(),
                platform: crate::PlatformKind::CustomRtmp,
                ingest_url: Some("rtmp://localhost/live".to_string()),
                stream_key: Some(crate::SensitiveString::new("secret")),
                enabled: Some(true),
            },
            Some(crate::SecretRef {
                provider: crate::LOCAL_SQLITE_SECRET_PROVIDER.to_string(),
                id: "stream-key".to_string(),
            }),
        );
        let config = MediaPipelinePlanRequest {
            dry_run: true,
            intent: PipelineIntent::RecordingAndStream,
            capture_sources: default_capture_sources(),
            active_scene: Some(scene.clone()),
            recording_profile: Some(profile),
            stream_destinations: vec![destination],
        }
        .into_config();
        let pipeline = MediaPipelinePlan {
            pipeline_name: "dry-run".to_string(),
            dry_run: true,
            ready: true,
            config,
            steps: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        let output_ready = ready_scene_output(&scene);

        let job = OutputJob::prepared(
            "job-ready".to_string(),
            OutputJobPrepareRequest::default(),
            &pipeline,
            &output_ready,
            crate::now_utc(),
        );

        assert_eq!(job.state, OutputJobState::Ready);
        assert!(job.scene_output_ready);
        assert!(job.media_pipeline_ready);
        assert!(job.output_preflight_ready);
        assert!(job.recording_target_ready);
        assert!(job.stream_targets_ready);
        assert!(job.output_path_preview.is_some());
    }

    #[test]
    fn prepared_output_job_is_blocked_by_scene_output_readiness() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap().clone();
        let config = MediaPipelinePlanRequest {
            dry_run: true,
            intent: PipelineIntent::Recording,
            capture_sources: default_capture_sources(),
            active_scene: Some(scene.clone()),
            recording_profile: Some(MediaProfile::default_local()),
            stream_destinations: Vec::new(),
        }
        .into_config();
        let pipeline = MediaPipelinePlan {
            pipeline_name: "dry-run".to_string(),
            dry_run: true,
            ready: true,
            config,
            steps: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        let mut output_ready = ready_scene_output(&scene);
        output_ready.ready = false;
        output_ready.state = DesignerRuntimeReadinessState::Blocked;
        output_ready
            .blockers
            .push("Program preview frame is not ready.".to_string());

        let job = OutputJob::prepared(
            "job-blocked".to_string(),
            OutputJobPrepareRequest::default(),
            &pipeline,
            &output_ready,
            crate::now_utc(),
        );

        assert_eq!(job.state, OutputJobState::Blocked);
        assert!(job
            .blockers
            .iter()
            .any(|blocker| blocker.contains("Program preview")));
    }

    #[test]
    fn cancelled_output_job_preserves_identity() {
        let now = crate::now_utc();
        let cancelled = OutputJob::idle(now).cancelled(now);

        assert_eq!(cancelled.id, "output-job-idle");
        assert_eq!(cancelled.state, OutputJobState::Cancelled);
    }

    fn ready_scene_output(scene: &crate::Scene) -> SceneOutputReadyDiagnostic {
        SceneOutputReadyDiagnostic {
            version: 1,
            ready: true,
            state: DesignerRuntimeReadinessState::Ready,
            active_scene_id: scene.id.clone(),
            active_scene_name: scene.name.clone(),
            program_preview_frame_ready: true,
            compositor_render_plan_ready: true,
            output_preflight_ready: true,
            media_pipeline_ready: true,
            detail: "ready".to_string(),
            blockers: Vec::new(),
            warnings: Vec::new(),
        }
    }
}
