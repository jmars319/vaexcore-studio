use std::{collections::HashSet, env, path::PathBuf, process::Command, time::Instant};

use serde::{Deserialize, Serialize};

use crate::{
    CaptureSourceKind, Scene, SceneSource, SceneSourceFilter, SceneSourceFilterKind,
    SceneSourceKind,
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AudioMixBusKind {
    Master,
    Monitor,
    Recording,
    Stream,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AudioMixSourceStatus {
    Ready,
    Placeholder,
    PermissionRequired,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AudioGraphInputMode {
    Live,
    Simulated,
    Silent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioMixBus {
    pub id: String,
    pub name: String,
    pub kind: AudioMixBusKind,
    pub sample_rate: u32,
    pub channels: u16,
    pub gain_db: f64,
    pub muted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioMixSource {
    pub scene_source_id: String,
    pub name: String,
    pub capture_source_id: Option<String>,
    pub capture_kind: CaptureSourceKind,
    pub gain_db: f64,
    pub muted: bool,
    pub monitor_enabled: bool,
    pub meter_enabled: bool,
    pub sync_offset_ms: i32,
    pub status: AudioMixSourceStatus,
    pub status_detail: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<SceneSourceFilter>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioMixerPlan {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sources: Vec<AudioMixSource>,
    pub buses: Vec<AudioMixBus>,
    pub validation: AudioMixerValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AudioMixerValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioGraphRuntimeSource {
    pub scene_source_id: String,
    pub name: String,
    pub capture_source_id: Option<String>,
    pub capture_kind: CaptureSourceKind,
    pub gain_db: f64,
    pub muted: bool,
    pub monitor_enabled: bool,
    pub meter_enabled: bool,
    pub sync_offset_ms: i32,
    pub input_mode: AudioGraphInputMode,
    pub provider_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_count: u32,
    pub capture_duration_ms: u64,
    pub latency_ms: f64,
    pub pre_filter_level_db: f64,
    pub pre_filter_peak_db: f64,
    pub pre_filter_linear_level: f64,
    pub post_filter_level_db: f64,
    pub post_filter_peak_db: f64,
    pub post_filter_linear_level: f64,
    pub level_db: f64,
    pub peak_db: f64,
    pub linear_level: f64,
    pub decay_level_db: f64,
    pub peak_hold_db: f64,
    pub status: AudioMixSourceStatus,
    pub status_detail: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<AudioFilterRuntimeMetadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AudioFilterRuntimeStatus {
    Applied,
    Skipped,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioFilterRuntimeMetadata {
    pub id: String,
    pub name: String,
    pub kind: SceneSourceFilterKind,
    pub enabled: bool,
    pub order: i32,
    pub status: AudioFilterRuntimeStatus,
    pub status_detail: String,
    pub input_level_db: f64,
    pub output_level_db: f64,
    pub input_peak_db: f64,
    pub output_peak_db: f64,
    pub level_change_db: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gain_reduction_db: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attenuation_db: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_summary: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioGraphRuntimeBus {
    pub id: String,
    pub name: String,
    pub kind: AudioMixBusKind,
    pub gain_db: f64,
    pub muted: bool,
    pub source_count: u32,
    pub active_source_count: u32,
    pub level_db: f64,
    pub peak_db: f64,
    pub linear_level: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AudioGraphRuntimeValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioGraphRuntimeSnapshot {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sources: Vec<AudioGraphRuntimeSource>,
    pub buses: Vec<AudioGraphRuntimeBus>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub validation: AudioGraphRuntimeValidation,
}

pub fn build_audio_mixer_plan(scene: &Scene) -> AudioMixerPlan {
    let sources = scene
        .sources
        .iter()
        .filter(|source| source.visible && source.kind == SceneSourceKind::AudioMeter)
        .map(audio_mix_source)
        .collect::<Vec<_>>();
    let mut plan = AudioMixerPlan {
        version: 1,
        scene_id: scene.id.clone(),
        scene_name: scene.name.clone(),
        sample_rate: 48_000,
        channels: 2,
        sources,
        buses: default_audio_buses(),
        validation: AudioMixerValidation {
            ready: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        },
    };
    plan.validation = validate_audio_mixer_plan(&plan);
    plan
}

pub fn validate_audio_mixer_plan(plan: &AudioMixerPlan) -> AudioMixerValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if plan.version == 0 {
        errors.push("audio mixer plan version must be greater than zero".to_string());
    }
    if plan.scene_id.trim().is_empty() {
        errors.push("audio mixer plan scene id is required".to_string());
    }
    if plan.scene_name.trim().is_empty() {
        errors.push("audio mixer plan scene name is required".to_string());
    }
    if plan.sample_rate == 0 {
        errors.push("audio mixer sample rate must be greater than zero".to_string());
    }
    if plan.channels == 0 {
        errors.push("audio mixer channels must be greater than zero".to_string());
    }
    if plan.sources.is_empty() {
        warnings.push("audio mixer has no audio scene sources".to_string());
    }
    if !plan
        .buses
        .iter()
        .any(|bus| bus.kind == AudioMixBusKind::Master)
    {
        errors.push("audio mixer requires a master bus".to_string());
    }

    for source in &plan.sources {
        if source.scene_source_id.trim().is_empty() {
            errors.push("audio mix source scene source id is required".to_string());
        }
        if source.name.trim().is_empty() {
            errors.push(format!(
                "audio mix source \"{}\" name is required",
                source.scene_source_id
            ));
        }
        validate_gain(source.gain_db, &source.name, &mut errors);
        match source.status {
            AudioMixSourceStatus::Ready => {}
            AudioMixSourceStatus::Placeholder => warnings.push(format!(
                "{} is waiting for an audio input: {}",
                source.name, source.status_detail
            )),
            AudioMixSourceStatus::PermissionRequired => warnings.push(format!(
                "{} requires audio permission: {}",
                source.name, source.status_detail
            )),
            AudioMixSourceStatus::Unavailable => warnings.push(format!(
                "{} audio is unavailable: {}",
                source.name, source.status_detail
            )),
        }
    }

    for bus in &plan.buses {
        if bus.id.trim().is_empty() {
            errors.push("audio mix bus id is required".to_string());
        }
        if bus.name.trim().is_empty() {
            errors.push(format!("audio mix bus \"{}\" name is required", bus.id));
        }
        if bus.sample_rate == 0 {
            errors.push(format!(
                "audio mix bus \"{}\" sample rate is required",
                bus.id
            ));
        }
        if bus.channels == 0 {
            errors.push(format!(
                "audio mix bus \"{}\" channels are required",
                bus.id
            ));
        }
        validate_gain(bus.gain_db, &bus.name, &mut errors);
    }

    AudioMixerValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

pub fn build_audio_graph_runtime_snapshot(
    scene: &Scene,
    frame_index: u64,
) -> AudioGraphRuntimeSnapshot {
    build_audio_graph_runtime_snapshot_with_probe(scene, frame_index, &simulated_audio_input)
}

pub fn build_live_audio_graph_runtime_snapshot(
    scene: &Scene,
    frame_index: u64,
) -> AudioGraphRuntimeSnapshot {
    build_audio_graph_runtime_snapshot_with_probe(scene, frame_index, &audio_input_for_source)
}

fn build_audio_graph_runtime_snapshot_with_probe(
    scene: &Scene,
    frame_index: u64,
    input_probe: &dyn Fn(&AudioMixSource, u64) -> AudioRuntimeInput,
) -> AudioGraphRuntimeSnapshot {
    let plan = build_audio_mixer_plan(scene);
    let sources = plan
        .sources
        .iter()
        .map(|source| audio_graph_runtime_source(source, frame_index, input_probe))
        .collect::<Vec<_>>();
    let buses = plan
        .buses
        .iter()
        .map(|bus| audio_graph_runtime_bus(bus, &sources))
        .collect::<Vec<_>>();
    let mut snapshot = AudioGraphRuntimeSnapshot {
        version: 1,
        scene_id: plan.scene_id,
        scene_name: plan.scene_name,
        sample_rate: plan.sample_rate,
        channels: plan.channels,
        sources,
        buses,
        generated_at: crate::now_utc(),
        validation: AudioGraphRuntimeValidation {
            ready: true,
            warnings: plan.validation.warnings,
            errors: plan.validation.errors,
        },
    };
    snapshot.validation = validate_audio_graph_runtime_snapshot(&snapshot);
    snapshot
}

pub fn validate_audio_graph_runtime_snapshot(
    snapshot: &AudioGraphRuntimeSnapshot,
) -> AudioGraphRuntimeValidation {
    let mut warnings = snapshot.validation.warnings.clone();
    let mut errors = snapshot.validation.errors.clone();
    let mut source_ids = HashSet::new();

    if snapshot.version == 0 {
        errors.push("audio graph runtime version must be greater than zero".to_string());
    }
    if snapshot.scene_id.trim().is_empty() {
        errors.push("audio graph runtime scene id is required".to_string());
    }
    if snapshot.scene_name.trim().is_empty() {
        errors.push("audio graph runtime scene name is required".to_string());
    }
    if snapshot.sample_rate == 0 {
        errors.push("audio graph runtime sample rate must be greater than zero".to_string());
    }
    if snapshot.channels == 0 {
        errors.push("audio graph runtime channels must be greater than zero".to_string());
    }
    if snapshot.sources.is_empty() {
        warnings.push("audio graph runtime has no audio meter sources".to_string());
    }

    for source in &snapshot.sources {
        if !source_ids.insert(source.scene_source_id.as_str()) {
            errors.push(format!(
                "duplicate audio graph source \"{}\"",
                source.scene_source_id
            ));
        }
        if source.name.trim().is_empty() {
            errors.push(format!(
                "audio graph source \"{}\" name is required",
                source.scene_source_id
            ));
        }
        if source.provider_name.trim().is_empty() {
            errors.push(format!(
                "audio graph source \"{}\" provider name is required",
                source.scene_source_id
            ));
        }
        if source.sample_rate == 0 {
            errors.push(format!(
                "audio graph source \"{}\" sample rate is required",
                source.scene_source_id
            ));
        }
        if source.channels == 0 {
            errors.push(format!(
                "audio graph source \"{}\" channels are required",
                source.scene_source_id
            ));
        }
        if !source.latency_ms.is_finite() || source.latency_ms < 0.0 {
            errors.push(format!(
                "audio graph source \"{}\" latency must be zero or greater",
                source.scene_source_id
            ));
        }
        match source.status {
            AudioMixSourceStatus::Ready => {}
            AudioMixSourceStatus::Placeholder => warnings.push(format!(
                "{} audio runtime is waiting for input: {}",
                source.name, source.status_detail
            )),
            AudioMixSourceStatus::PermissionRequired => warnings.push(format!(
                "{} audio runtime requires permission: {}",
                source.name, source.status_detail
            )),
            AudioMixSourceStatus::Unavailable => warnings.push(format!(
                "{} audio runtime is unavailable: {}",
                source.name, source.status_detail
            )),
        }
        validate_level(source.level_db, &source.name, &mut errors);
        validate_level(source.peak_db, &source.name, &mut errors);
        validate_level(source.decay_level_db, &source.name, &mut errors);
        validate_level(source.peak_hold_db, &source.name, &mut errors);
        validate_level(source.pre_filter_level_db, &source.name, &mut errors);
        validate_level(source.pre_filter_peak_db, &source.name, &mut errors);
        validate_level(source.post_filter_level_db, &source.name, &mut errors);
        validate_level(source.post_filter_peak_db, &source.name, &mut errors);
        validate_linear_level(
            source.linear_level,
            &source.name,
            "linear level",
            &mut errors,
        );
        validate_linear_level(
            source.pre_filter_linear_level,
            &source.name,
            "pre-filter linear level",
            &mut errors,
        );
        validate_linear_level(
            source.post_filter_linear_level,
            &source.name,
            "post-filter linear level",
            &mut errors,
        );
        for filter in &source.filters {
            validate_level(filter.input_level_db, &filter.name, &mut errors);
            validate_level(filter.output_level_db, &filter.name, &mut errors);
            validate_level(filter.input_peak_db, &filter.name, &mut errors);
            validate_level(filter.output_peak_db, &filter.name, &mut errors);
            if !filter.level_change_db.is_finite() {
                errors.push(format!("{} level change must be finite", filter.name));
            }
            if matches!(filter.gain_reduction_db, Some(value) if !value.is_finite() || value < 0.0)
            {
                errors.push(format!(
                    "{} gain reduction must be zero or greater",
                    filter.name
                ));
            }
            if matches!(filter.attenuation_db, Some(value) if !value.is_finite() || value < 0.0) {
                errors.push(format!(
                    "{} attenuation must be zero or greater",
                    filter.name
                ));
            }
        }
    }

    for bus in &snapshot.buses {
        if bus.active_source_count > bus.source_count {
            errors.push(format!(
                "{} active source count cannot exceed source count",
                bus.name
            ));
        }
        validate_level(bus.level_db, &bus.name, &mut errors);
        validate_level(bus.peak_db, &bus.name, &mut errors);
        validate_linear_level(bus.linear_level, &bus.name, "linear bus level", &mut errors);
    }

    AudioGraphRuntimeValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

fn audio_mix_source(source: &SceneSource) -> AudioMixSource {
    let capture_source_id = source.capture_identity();
    let (status, status_detail) = audio_source_status(source, capture_source_id.as_deref());
    AudioMixSource {
        scene_source_id: source.id.clone(),
        name: source.name.clone(),
        capture_source_id,
        capture_kind: audio_capture_kind(source),
        gain_db: config_number(source, "gain_db").unwrap_or(0.0),
        muted: config_bool(source, "muted").unwrap_or(false),
        monitor_enabled: config_bool(source, "monitor_enabled").unwrap_or(false),
        meter_enabled: config_bool(source, "meter_enabled").unwrap_or(true),
        sync_offset_ms: config_number(source, "sync_offset_ms")
            .map(|value| value.round() as i32)
            .unwrap_or(0),
        status,
        status_detail,
        filters: source.filters.clone(),
    }
}

fn audio_graph_runtime_source(
    source: &AudioMixSource,
    frame_index: u64,
    input_probe: &dyn Fn(&AudioMixSource, u64) -> AudioRuntimeInput,
) -> AudioGraphRuntimeSource {
    let input = input_probe(source, frame_index);
    let pre_filter_level_db = input.level_db;
    let pre_filter_peak_db = input.peak_db;
    let pre_filter_linear_level = db_to_linear(pre_filter_level_db);
    let filtered = apply_audio_filters(source, pre_filter_level_db, pre_filter_peak_db);
    let post_filter_level_db = filtered.level_db;
    let post_filter_peak_db = filtered.peak_db;
    let post_filter_linear_level = db_to_linear(post_filter_level_db);
    let decay_level_db = meter_decay_level(post_filter_level_db, post_filter_peak_db, frame_index);
    let peak_hold_db = meter_peak_hold_level(post_filter_peak_db, decay_level_db);
    AudioGraphRuntimeSource {
        scene_source_id: source.scene_source_id.clone(),
        name: source.name.clone(),
        capture_source_id: source.capture_source_id.clone(),
        capture_kind: source.capture_kind.clone(),
        gain_db: source.gain_db,
        muted: source.muted,
        monitor_enabled: source.monitor_enabled,
        meter_enabled: source.meter_enabled,
        sync_offset_ms: source.sync_offset_ms,
        input_mode: input.input_mode,
        provider_name: input.provider_name,
        sample_rate: input.sample_rate,
        channels: input.channels,
        sample_count: input.sample_count,
        capture_duration_ms: input.capture_duration_ms,
        latency_ms: input.latency_ms,
        pre_filter_level_db,
        pre_filter_peak_db,
        pre_filter_linear_level,
        post_filter_level_db,
        post_filter_peak_db,
        post_filter_linear_level,
        level_db: post_filter_level_db,
        peak_db: post_filter_peak_db,
        linear_level: post_filter_linear_level,
        decay_level_db,
        peak_hold_db,
        status: input.status,
        status_detail: input.status_detail,
        filters: filtered.filters,
    }
}

fn audio_graph_runtime_bus(
    bus: &AudioMixBus,
    sources: &[AudioGraphRuntimeSource],
) -> AudioGraphRuntimeBus {
    let source_count = sources.len() as u32;
    let active_source_count = sources
        .iter()
        .filter(|source| {
            source.input_mode != AudioGraphInputMode::Silent
                && source.linear_level > 0.0
                && !source.muted
                && source.meter_enabled
        })
        .count() as u32;
    let linear_level = if bus.muted || sources.is_empty() {
        0.0
    } else {
        sources
            .iter()
            .map(|source| source.linear_level)
            .fold(0.0, f64::max)
            .min(1.0)
    };
    let level_db = linear_to_db(linear_level) + bus.gain_db;
    let peak_db = (level_db + 4.5).min(6.0);

    AudioGraphRuntimeBus {
        id: bus.id.clone(),
        name: bus.name.clone(),
        kind: bus.kind.clone(),
        gain_db: bus.gain_db,
        muted: bus.muted,
        source_count,
        active_source_count,
        level_db: level_db.clamp(-90.0, 6.0),
        peak_db: peak_db.clamp(-90.0, 6.0),
        linear_level,
    }
}

#[derive(Clone, Debug)]
struct AudioRuntimeInput {
    input_mode: AudioGraphInputMode,
    provider_name: String,
    sample_rate: u32,
    channels: u16,
    sample_count: u32,
    capture_duration_ms: u64,
    latency_ms: f64,
    level_db: f64,
    peak_db: f64,
    status: AudioMixSourceStatus,
    status_detail: String,
}

struct AudioFilterRuntimeResult {
    level_db: f64,
    peak_db: f64,
    filters: Vec<AudioFilterRuntimeMetadata>,
}

struct AudioFilterApplyResult {
    level_db: f64,
    peak_db: f64,
    status_detail: String,
    gain_reduction_db: Option<f64>,
    attenuation_db: Option<f64>,
    control_summary: Option<String>,
}

struct AudioFilterMetadataLevels {
    input_level_db: f64,
    input_peak_db: f64,
    output_level_db: f64,
    output_peak_db: f64,
}

fn apply_audio_filters(
    source: &AudioMixSource,
    level_db: f64,
    peak_db: f64,
) -> AudioFilterRuntimeResult {
    let mut current_level_db = level_db;
    let mut current_peak_db = peak_db;
    let mut metadata = Vec::with_capacity(source.filters.len());

    for filter in sorted_audio_source_filters(&source.filters) {
        let input_level_db = current_level_db;
        let input_peak_db = current_peak_db;
        if !filter.enabled {
            metadata.push(audio_filter_metadata(
                filter,
                AudioFilterRuntimeStatus::Skipped,
                "Filter is disabled.".to_string(),
                AudioFilterMetadataLevels {
                    input_level_db,
                    input_peak_db,
                    output_level_db: input_level_db,
                    output_peak_db: input_peak_db,
                },
                None,
                None,
                None,
            ));
            continue;
        }

        match apply_audio_filter(input_level_db, input_peak_db, filter) {
            Ok(result) => {
                current_level_db = result.level_db;
                current_peak_db = result.peak_db;
                metadata.push(audio_filter_metadata(
                    filter,
                    AudioFilterRuntimeStatus::Applied,
                    result.status_detail,
                    AudioFilterMetadataLevels {
                        input_level_db,
                        input_peak_db,
                        output_level_db: current_level_db,
                        output_peak_db: current_peak_db,
                    },
                    result.gain_reduction_db,
                    result.attenuation_db,
                    result.control_summary,
                ));
            }
            Err((status, detail)) => {
                metadata.push(audio_filter_metadata(
                    filter,
                    status,
                    detail,
                    AudioFilterMetadataLevels {
                        input_level_db,
                        input_peak_db,
                        output_level_db: input_level_db,
                        output_peak_db: input_peak_db,
                    },
                    None,
                    None,
                    None,
                ));
            }
        }
    }

    AudioFilterRuntimeResult {
        level_db: current_level_db,
        peak_db: current_peak_db,
        filters: metadata,
    }
}

fn sorted_audio_source_filters(filters: &[SceneSourceFilter]) -> Vec<&SceneSourceFilter> {
    let mut sorted = filters.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.id.cmp(&right.id))
    });
    sorted
}

fn apply_audio_filter(
    level_db: f64,
    peak_db: f64,
    filter: &SceneSourceFilter,
) -> Result<AudioFilterApplyResult, (AudioFilterRuntimeStatus, String)> {
    match filter.kind {
        SceneSourceFilterKind::AudioGain => apply_audio_gain_filter(level_db, peak_db, filter),
        SceneSourceFilterKind::NoiseGate => apply_noise_gate_filter(level_db, peak_db, filter),
        SceneSourceFilterKind::Compressor => apply_compressor_filter(level_db, peak_db, filter),
        _ => Err((
            AudioFilterRuntimeStatus::Skipped,
            "Non-audio filter is not applied in the audio graph runtime.".to_string(),
        )),
    }
}

fn apply_audio_gain_filter(
    level_db: f64,
    peak_db: f64,
    filter: &SceneSourceFilter,
) -> Result<AudioFilterApplyResult, (AudioFilterRuntimeStatus, String)> {
    let gain_db = audio_filter_number(filter, "gain_db", -60.0, 24.0)?;
    let output_level_db = clamp_audio_level(level_db + gain_db);
    let output_peak_db = clamp_audio_level(peak_db + gain_db);
    Ok(AudioFilterApplyResult {
        level_db: output_level_db,
        peak_db: output_peak_db,
        status_detail: format!("Applied {gain_db:.1} dB audio gain."),
        gain_reduction_db: None,
        attenuation_db: (gain_db < 0.0).then_some((level_db - output_level_db).max(0.0)),
        control_summary: Some(format!("{gain_db:+.1} dB")),
    })
}

fn apply_noise_gate_filter(
    level_db: f64,
    peak_db: f64,
    filter: &SceneSourceFilter,
) -> Result<AudioFilterApplyResult, (AudioFilterRuntimeStatus, String)> {
    let close_threshold_db = audio_filter_number(filter, "close_threshold_db", -100.0, 0.0)?;
    let open_threshold_db = audio_filter_number(filter, "open_threshold_db", -100.0, 0.0)?;
    if close_threshold_db >= open_threshold_db {
        return Err((
            AudioFilterRuntimeStatus::Error,
            "Noise gate open threshold must be greater than close threshold.".to_string(),
        ));
    }
    let attack_ms = audio_filter_number(filter, "attack_ms", 0.0, 5_000.0)?;
    let release_ms = audio_filter_number(filter, "release_ms", 0.0, 5_000.0)?;

    let (output_level_db, output_peak_db, detail) = if level_db <= close_threshold_db {
        (
            -90.0,
            -90.0,
            format!("Gate closed below {close_threshold_db:.1} dB."),
        )
    } else if level_db >= open_threshold_db {
        (
            level_db,
            peak_db,
            format!("Gate open above {open_threshold_db:.1} dB."),
        )
    } else {
        let openness = ((level_db - close_threshold_db) / (open_threshold_db - close_threshold_db))
            .clamp(0.0, 1.0);
        let output_level_db = linear_to_db(db_to_linear(level_db) * openness);
        let output_peak_db = linear_to_db(db_to_linear(peak_db) * openness);
        (
            output_level_db,
            output_peak_db,
            "Gate applied deterministic threshold-band attenuation.".to_string(),
        )
    };
    let attenuation_db = (level_db - output_level_db).max(0.0);
    Ok(AudioFilterApplyResult {
        level_db: output_level_db,
        peak_db: output_peak_db,
        status_detail: detail,
        gain_reduction_db: None,
        attenuation_db: Some(attenuation_db),
        control_summary: Some(format!(
            "close {close_threshold_db:.1} dB / open {open_threshold_db:.1} dB / attack {attack_ms:.0} ms / release {release_ms:.0} ms"
        )),
    })
}

fn apply_compressor_filter(
    level_db: f64,
    peak_db: f64,
    filter: &SceneSourceFilter,
) -> Result<AudioFilterApplyResult, (AudioFilterRuntimeStatus, String)> {
    let threshold_db = audio_filter_number(filter, "threshold_db", -100.0, 0.0)?;
    let ratio = audio_filter_number(filter, "ratio", 1.0, 20.0)?;
    let attack_ms = audio_filter_number(filter, "attack_ms", 0.0, 5_000.0)?;
    let release_ms = audio_filter_number(filter, "release_ms", 0.0, 5_000.0)?;
    let makeup_gain_db = audio_filter_number(filter, "makeup_gain_db", -24.0, 24.0)?;
    let compressed_level_db = compress_audio_level(level_db, threshold_db, ratio);
    let compressed_peak_db = compress_audio_level(peak_db, threshold_db, ratio);
    let gain_reduction_db = (level_db - compressed_level_db).max(0.0);
    let output_level_db = clamp_audio_level(compressed_level_db + makeup_gain_db);
    let output_peak_db = clamp_audio_level(compressed_peak_db + makeup_gain_db);
    Ok(AudioFilterApplyResult {
        level_db: output_level_db,
        peak_db: output_peak_db,
        status_detail: format!(
            "Compressed above {threshold_db:.1} dB at {ratio:.1}:1 with {makeup_gain_db:+.1} dB makeup."
        ),
        gain_reduction_db: Some(gain_reduction_db),
        attenuation_db: (output_level_db < level_db).then_some(level_db - output_level_db),
        control_summary: Some(format!(
            "threshold {threshold_db:.1} dB / ratio {ratio:.1}:1 / attack {attack_ms:.0} ms / release {release_ms:.0} ms / makeup {makeup_gain_db:+.1} dB"
        )),
    })
}

fn audio_filter_number(
    filter: &SceneSourceFilter,
    key: &str,
    min: f64,
    max: f64,
) -> Result<f64, (AudioFilterRuntimeStatus, String)> {
    let Some(value) = filter.config.get(key).and_then(serde_json::Value::as_f64) else {
        return Err((
            AudioFilterRuntimeStatus::Error,
            format!("Filter config {key} must be a number."),
        ));
    };
    if !value.is_finite() || value < min || value > max {
        return Err((
            AudioFilterRuntimeStatus::Error,
            format!("Filter config {key} must be between {min} and {max}."),
        ));
    }
    Ok(value)
}

fn compress_audio_level(level_db: f64, threshold_db: f64, ratio: f64) -> f64 {
    if level_db <= threshold_db {
        level_db
    } else {
        threshold_db + ((level_db - threshold_db) / ratio)
    }
}

fn audio_filter_metadata(
    filter: &SceneSourceFilter,
    status: AudioFilterRuntimeStatus,
    status_detail: String,
    levels: AudioFilterMetadataLevels,
    gain_reduction_db: Option<f64>,
    attenuation_db: Option<f64>,
    control_summary: Option<String>,
) -> AudioFilterRuntimeMetadata {
    AudioFilterRuntimeMetadata {
        id: filter.id.clone(),
        name: filter.name.clone(),
        kind: filter.kind.clone(),
        enabled: filter.enabled,
        order: filter.order,
        status,
        status_detail,
        input_level_db: levels.input_level_db,
        output_level_db: levels.output_level_db,
        input_peak_db: levels.input_peak_db,
        output_peak_db: levels.output_peak_db,
        level_change_db: levels.output_level_db - levels.input_level_db,
        gain_reduction_db,
        attenuation_db,
        control_summary,
    }
}

fn clamp_audio_level(value: f64) -> f64 {
    value.clamp(-90.0, 6.0)
}

fn audio_input_for_source(source: &AudioMixSource, frame_index: u64) -> AudioRuntimeInput {
    if source.muted || !source.meter_enabled {
        return silent_audio_input(
            source,
            if source.muted {
                "Source is muted."
            } else {
                "Metering is disabled for this source."
            },
        );
    }

    if source.status == AudioMixSourceStatus::Ready && source.capture_source_id.is_some() {
        return match live_audio_input_for_source(source) {
            Ok(input) => apply_source_gain_to_input(source, input),
            Err(error) => AudioRuntimeInput {
                input_mode: AudioGraphInputMode::Silent,
                provider_name: error.provider_name,
                sample_rate: 48_000,
                channels: 1,
                sample_count: 0,
                capture_duration_ms: 0,
                latency_ms: 0.0,
                level_db: -90.0,
                peak_db: -90.0,
                status: error.status,
                status_detail: error.detail,
            },
        };
    }

    simulated_audio_input(source, frame_index)
}

fn simulated_audio_input(source: &AudioMixSource, frame_index: u64) -> AudioRuntimeInput {
    if source.muted || !source.meter_enabled {
        return silent_audio_input(
            source,
            if source.muted {
                "Source is muted."
            } else {
                "Metering is disabled for this source."
            },
        );
    }

    let seed = stable_audio_seed(&source.scene_source_id);
    let phase = ((frame_index.wrapping_mul(17) + seed) % 100) as f64 / 100.0;
    let wave = (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5;
    let status_offset = match source.status {
        AudioMixSourceStatus::Ready => 0.0,
        AudioMixSourceStatus::Placeholder => -8.0,
        AudioMixSourceStatus::PermissionRequired => -12.0,
        AudioMixSourceStatus::Unavailable => -18.0,
    };
    let level_db = (-48.0 + wave * 32.0 + source.gain_db + status_offset).clamp(-90.0, 6.0);
    let peak_db = (level_db + 5.0 + (f64::from((seed % 7) as u8) * 0.35)).clamp(-90.0, 6.0);
    AudioRuntimeInput {
        input_mode: AudioGraphInputMode::Simulated,
        provider_name: "deterministic-simulator".to_string(),
        sample_rate: 48_000,
        channels: 2,
        sample_count: 960,
        capture_duration_ms: 0,
        latency_ms: 0.0,
        level_db,
        peak_db,
        status: source.status.clone(),
        status_detail: source.status_detail.clone(),
    }
}

fn silent_audio_input(source: &AudioMixSource, detail: &str) -> AudioRuntimeInput {
    AudioRuntimeInput {
        input_mode: AudioGraphInputMode::Silent,
        provider_name: "silence".to_string(),
        sample_rate: 48_000,
        channels: 2,
        sample_count: 0,
        capture_duration_ms: 0,
        latency_ms: 0.0,
        level_db: -90.0,
        peak_db: -90.0,
        status: source.status.clone(),
        status_detail: detail.to_string(),
    }
}

fn apply_source_gain_to_input(
    source: &AudioMixSource,
    mut input: AudioRuntimeInput,
) -> AudioRuntimeInput {
    input.level_db = clamp_audio_level(input.level_db + source.gain_db);
    input.peak_db = clamp_audio_level(input.peak_db + source.gain_db);
    input
}

struct AudioRuntimeInputError {
    status: AudioMixSourceStatus,
    provider_name: String,
    detail: String,
}

#[cfg(target_os = "macos")]
fn live_audio_input_for_source(
    source: &AudioMixSource,
) -> Result<AudioRuntimeInput, AudioRuntimeInputError> {
    let Some(capture_id) = source.capture_source_id.as_deref() else {
        return Err(AudioRuntimeInputError {
            status: AudioMixSourceStatus::Placeholder,
            provider_name: "macos-avfoundation-ffmpeg".to_string(),
            detail: "No audio capture source has been assigned.".to_string(),
        });
    };
    let Some(ffmpeg_path) = find_ffmpeg_binary() else {
        return Err(AudioRuntimeInputError {
            status: AudioMixSourceStatus::Unavailable,
            provider_name: "macos-avfoundation-ffmpeg".to_string(),
            detail: "FFmpeg is required for live audio meter preview V1 and was not found."
                .to_string(),
        });
    };
    let Some(audio_index) = macos_audio_index_from_source_id(capture_id) else {
        return Err(AudioRuntimeInputError {
            status: AudioMixSourceStatus::Unavailable,
            provider_name: "macos-avfoundation-ffmpeg".to_string(),
            detail: format!("Unsupported macOS audio source id \"{capture_id}\"."),
        });
    };

    let input_name = format!(":{audio_index}");
    let started_at = Instant::now();
    let output = Command::new(ffmpeg_path)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-f",
            "avfoundation",
            "-i",
            &input_name,
            "-t",
            "0.12",
            "-vn",
            "-ac",
            "1",
            "-ar",
            "48000",
            "-f",
            "f32le",
            "pipe:1",
        ])
        .output()
        .map_err(|error| AudioRuntimeInputError {
            status: AudioMixSourceStatus::Unavailable,
            provider_name: "macos-avfoundation-ffmpeg".to_string(),
            detail: format!("FFmpeg audio meter preview could not be started: {error}"),
        })?;
    let capture_duration_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    if !output.status.success() {
        let detail = ffmpeg_audio_error_detail(&output.stderr);
        return Err(AudioRuntimeInputError {
            status: ffmpeg_audio_error_status(&detail),
            provider_name: "macos-avfoundation-ffmpeg".to_string(),
            detail,
        });
    }
    let (level_db, peak_db, sample_count) =
        decode_f32le_audio_level(&output.stdout).map_err(|detail| AudioRuntimeInputError {
            status: AudioMixSourceStatus::Unavailable,
            provider_name: "macos-avfoundation-ffmpeg".to_string(),
            detail,
        })?;

    Ok(AudioRuntimeInput {
        input_mode: AudioGraphInputMode::Live,
        provider_name: "macos-avfoundation-ffmpeg".to_string(),
        sample_rate: 48_000,
        channels: 1,
        sample_count,
        capture_duration_ms,
        latency_ms: capture_duration_ms as f64,
        level_db,
        peak_db,
        status: AudioMixSourceStatus::Ready,
        status_detail: format!("Captured live macOS audio level from {capture_id}."),
    })
}

#[cfg(not(target_os = "macos"))]
fn live_audio_input_for_source(
    source: &AudioMixSource,
) -> Result<AudioRuntimeInput, AudioRuntimeInputError> {
    Err(AudioRuntimeInputError {
        status: AudioMixSourceStatus::Unavailable,
        provider_name: "unsupported-platform".to_string(),
        detail: format!(
            "Live audio meter preview is not implemented on this platform for {}.",
            source
                .capture_source_id
                .as_deref()
                .unwrap_or("the selected audio source")
        ),
    })
}

#[cfg(target_os = "macos")]
fn macos_audio_index_from_source_id(capture_id: &str) -> Option<u32> {
    if capture_id == "microphone:default" || capture_id == "system_audio:default" {
        return Some(0);
    }
    capture_id
        .strip_prefix("microphone:")
        .or_else(|| capture_id.strip_prefix("system_audio:"))
        .or_else(|| capture_id.strip_prefix("system:"))
        .and_then(|value| value.parse::<u32>().ok())
}

fn decode_f32le_audio_level(bytes: &[u8]) -> Result<(f64, f64, u32), String> {
    let mut sum_square = 0.0_f64;
    let mut peak = 0.0_f64;
    let mut count = 0_u32;

    for chunk in bytes.chunks_exact(4) {
        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64;
        if !sample.is_finite() {
            continue;
        }
        let absolute = sample.abs().clamp(0.0, 1.0);
        sum_square += absolute * absolute;
        peak = peak.max(absolute);
        count = count.saturating_add(1);
    }

    if count == 0 {
        return Err("FFmpeg returned no audio samples for live meter preview.".to_string());
    }

    let rms = (sum_square / f64::from(count)).sqrt().clamp(0.0, 1.0);
    Ok((linear_to_db(rms), linear_to_db(peak), count))
}

#[cfg(target_os = "macos")]
fn ffmpeg_audio_error_detail(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if stderr.is_empty() {
        "FFmpeg audio meter preview failed. Check Microphone permission and source availability."
            .to_string()
    } else {
        format!("FFmpeg audio meter preview failed: {stderr}")
    }
}

#[cfg(target_os = "macos")]
fn ffmpeg_audio_error_status(detail: &str) -> AudioMixSourceStatus {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("not authorized")
        || lower.contains("permission")
        || lower.contains("privacy")
        || lower.contains("denied")
    {
        AudioMixSourceStatus::PermissionRequired
    } else {
        AudioMixSourceStatus::Unavailable
    }
}

fn meter_decay_level(level_db: f64, peak_db: f64, frame_index: u64) -> f64 {
    let decay_db = 1.5 + f64::from((frame_index % 4) as u8) * 0.5;
    level_db.max(peak_db - decay_db).clamp(-90.0, 6.0)
}

fn meter_peak_hold_level(peak_db: f64, decay_level_db: f64) -> f64 {
    peak_db.max(decay_level_db).clamp(-90.0, 6.0)
}

fn find_ffmpeg_binary() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(explicit_path) = env::var_os("VAEXCORE_FFMPEG_PATH") {
        candidates.push(PathBuf::from(explicit_path));
    }
    if let Some(paths) = env::var_os("PATH") {
        for path in env::split_paths(&paths) {
            for executable_name in ffmpeg_executable_names() {
                candidates.push(path.join(executable_name));
            }
        }
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin/ffmpeg"),
        PathBuf::from("/usr/local/bin/ffmpeg"),
        PathBuf::from("/usr/bin/ffmpeg"),
    ]);

    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn ffmpeg_executable_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["ffmpeg.exe", "ffmpeg"]
    } else {
        &["ffmpeg"]
    }
}

fn db_to_linear(value: f64) -> f64 {
    if value <= -90.0 {
        0.0
    } else {
        10_f64.powf(value / 20.0).clamp(0.0, 1.0)
    }
}

fn linear_to_db(value: f64) -> f64 {
    if value <= 0.0 {
        -90.0
    } else {
        (20.0 * value.log10()).clamp(-90.0, 6.0)
    }
}

fn stable_audio_seed(value: &str) -> u64 {
    value.bytes().fold(17_u64, |hash, byte| {
        hash.wrapping_mul(37).wrapping_add(u64::from(byte))
    })
}

fn audio_capture_kind(source: &SceneSource) -> CaptureSourceKind {
    match source
        .config
        .get("channel")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("microphone")
    {
        "system" => CaptureSourceKind::SystemAudio,
        _ => CaptureSourceKind::Microphone,
    }
}

fn audio_source_status(
    source: &SceneSource,
    capture_source_id: Option<&str>,
) -> (AudioMixSourceStatus, String) {
    if capture_source_id.is_none() {
        return (
            AudioMixSourceStatus::Placeholder,
            "No audio capture source has been assigned.".to_string(),
        );
    }

    let Some(availability) = source.config.get("availability") else {
        return (
            AudioMixSourceStatus::Ready,
            "Audio capture source is configured.".to_string(),
        );
    };
    let detail = availability
        .get("detail")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Audio capture availability has not been checked.")
        .to_string();
    let status = match availability
        .get("state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
    {
        "available" => AudioMixSourceStatus::Ready,
        "permission_required" => AudioMixSourceStatus::PermissionRequired,
        "unavailable" => AudioMixSourceStatus::Unavailable,
        _ => AudioMixSourceStatus::Placeholder,
    };

    (status, detail)
}

fn default_audio_buses() -> Vec<AudioMixBus> {
    vec![
        audio_bus("bus-master", "Master", AudioMixBusKind::Master),
        audio_bus("bus-monitor", "Monitor", AudioMixBusKind::Monitor),
        audio_bus("bus-recording", "Recording", AudioMixBusKind::Recording),
        audio_bus("bus-stream", "Stream", AudioMixBusKind::Stream),
    ]
}

fn audio_bus(id: &str, name: &str, kind: AudioMixBusKind) -> AudioMixBus {
    AudioMixBus {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        sample_rate: 48_000,
        channels: 2,
        gain_db: 0.0,
        muted: false,
    }
}

fn config_bool(source: &SceneSource, key: &str) -> Option<bool> {
    source.config.get(key).and_then(serde_json::Value::as_bool)
}

fn config_number(source: &SceneSource, key: &str) -> Option<f64> {
    source.config.get(key).and_then(serde_json::Value::as_f64)
}

fn validate_gain(value: f64, label: &str, errors: &mut Vec<String>) {
    if !value.is_finite() || !(-60.0..=24.0).contains(&value) {
        errors.push(format!("{label} gain must be between -60 dB and 24 dB"));
    }
}

fn validate_level(value: f64, label: &str, errors: &mut Vec<String>) {
    if !value.is_finite() || !(-90.0..=6.0).contains(&value) {
        errors.push(format!("{label} level must be between -90 dB and 6 dB"));
    }
}

fn validate_linear_level(value: f64, label: &str, field: &str, errors: &mut Vec<String>) {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        errors.push(format!("{label} {field} must be between zero and one"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn audio_mixer_plan_describes_default_meter() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let plan = build_audio_mixer_plan(scene);

        assert_eq!(plan.scene_id, "scene-main");
        assert_eq!(plan.sample_rate, 48_000);
        assert_eq!(plan.channels, 2);
        assert_eq!(plan.sources.len(), 1);
        assert_eq!(plan.buses.len(), 4);
        assert!(plan.validation.ready, "{:?}", plan.validation.errors);
        assert!(plan
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("waiting for an audio input")));

        let source = &plan.sources[0];
        assert_eq!(source.scene_source_id, "source-mic-meter");
        assert_eq!(source.capture_kind, CaptureSourceKind::Microphone);
        assert_eq!(source.gain_db, 0.0);
        assert!(source.meter_enabled);
    }

    #[test]
    fn audio_mixer_validation_rejects_bad_gain() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let mut plan = build_audio_mixer_plan(scene);
        plan.sources[0].gain_db = 100.0;

        let validation = validate_audio_mixer_plan(&plan);

        assert!(!validation.ready);
        assert!(validation.errors.iter().any(|error| error.contains("gain")));
    }

    #[test]
    fn audio_graph_runtime_simulates_meter_levels() {
        let collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();

        let first = build_audio_graph_runtime_snapshot(scene, 1);
        let second = build_audio_graph_runtime_snapshot(scene, 5);

        assert_eq!(first.scene_id, "scene-main");
        assert_eq!(first.sources.len(), 1);
        assert_eq!(first.buses.len(), 4);
        assert!(first.validation.ready, "{:?}", first.validation.errors);
        assert!((0.0..=1.0).contains(&first.sources[0].linear_level));
        assert_ne!(first.sources[0].level_db, second.sources[0].level_db);
        assert_eq!(first.sources[0].sync_offset_ms, 0);
        assert!(first
            .buses
            .iter()
            .any(|bus| bus.kind == AudioMixBusKind::Master));
        assert_eq!(first.sources[0].input_mode, AudioGraphInputMode::Simulated);
        assert_eq!(first.sources[0].provider_name, "deterministic-simulator");
        assert_eq!(first.sources[0].sample_rate, 48_000);
        assert!(first.sources[0].peak_hold_db >= first.sources[0].decay_level_db);
        assert_eq!(first.buses[0].source_count, 1);
    }

    #[test]
    fn audio_graph_runtime_uses_live_probe_levels_when_available() {
        let scene = test_audio_scene(Vec::new());

        let snapshot =
            build_audio_graph_runtime_snapshot_with_probe(&scene, 12, &mock_live_audio_input);
        let source = &snapshot.sources[0];

        assert_eq!(source.input_mode, AudioGraphInputMode::Live);
        assert_eq!(source.provider_name, "mock-live-audio");
        assert_eq!(source.sample_count, 480);
        assert_eq!(source.status, AudioMixSourceStatus::Ready);
        assert_approx_eq(source.pre_filter_level_db, -18.0);
        assert_approx_eq(source.level_db, -18.0);
        assert_eq!(snapshot.buses[0].active_source_count, 1);
        assert_approx_eq(snapshot.buses[0].level_db, source.level_db);
    }

    #[test]
    fn audio_graph_runtime_reports_silent_live_failure_from_probe() {
        let scene = test_audio_scene(Vec::new());

        let snapshot = build_audio_graph_runtime_snapshot_with_probe(
            &scene,
            12,
            &mock_unavailable_audio_input,
        );
        let source = &snapshot.sources[0];

        assert_eq!(source.input_mode, AudioGraphInputMode::Silent);
        assert_eq!(source.status, AudioMixSourceStatus::Unavailable);
        assert_eq!(source.level_db, -90.0);
        assert_eq!(snapshot.buses[0].active_source_count, 0);
        assert!(source.status_detail.contains("unavailable"));
    }

    #[test]
    fn audio_graph_runtime_applies_audio_gain_filter() {
        let scene = test_audio_scene(vec![test_filter(
            "filter-gain",
            SceneSourceFilterKind::AudioGain,
            true,
            0,
            json!({ "gain_db": 6.0 }),
        )]);

        let snapshot = build_audio_graph_runtime_snapshot(&scene, 3);
        let source = &snapshot.sources[0];

        assert_eq!(source.filters[0].status, AudioFilterRuntimeStatus::Applied);
        assert_approx_eq(
            source.post_filter_level_db - source.pre_filter_level_db,
            6.0,
        );
        assert_eq!(source.level_db, source.post_filter_level_db);
        assert_approx_eq(snapshot.buses[0].level_db, source.level_db);
    }

    #[test]
    fn audio_graph_runtime_applies_noise_gate_states() {
        let base_scene = test_audio_scene(Vec::new());
        let base = build_audio_graph_runtime_snapshot(&base_scene, 4);
        let input_level = base.sources[0].pre_filter_level_db;

        let closed = build_audio_graph_runtime_snapshot(
            &test_audio_scene(vec![test_filter(
                "filter-gate-closed",
                SceneSourceFilterKind::NoiseGate,
                true,
                0,
                json!({
                    "close_threshold_db": input_level + 1.0,
                    "open_threshold_db": input_level + 4.0,
                    "attack_ms": 10.0,
                    "release_ms": 120.0
                }),
            )]),
            4,
        );
        assert_eq!(closed.sources[0].level_db, -90.0);
        assert!(closed.sources[0].filters[0]
            .status_detail
            .contains("closed"));

        let open = build_audio_graph_runtime_snapshot(
            &test_audio_scene(vec![test_filter(
                "filter-gate-open",
                SceneSourceFilterKind::NoiseGate,
                true,
                0,
                json!({
                    "close_threshold_db": input_level - 8.0,
                    "open_threshold_db": input_level - 4.0,
                    "attack_ms": 10.0,
                    "release_ms": 120.0
                }),
            )]),
            4,
        );
        assert_approx_eq(open.sources[0].level_db, input_level);
        assert!(open.sources[0].filters[0].status_detail.contains("open"));

        let band = build_audio_graph_runtime_snapshot(
            &test_audio_scene(vec![test_filter(
                "filter-gate-band",
                SceneSourceFilterKind::NoiseGate,
                true,
                0,
                json!({
                    "close_threshold_db": input_level - 3.0,
                    "open_threshold_db": input_level + 3.0,
                    "attack_ms": 10.0,
                    "release_ms": 120.0
                }),
            )]),
            4,
        );
        assert!(band.sources[0].level_db < input_level);
        assert!(band.sources[0].level_db > -90.0);
        assert!(band.sources[0].filters[0]
            .status_detail
            .contains("threshold-band"));
    }

    #[test]
    fn audio_graph_runtime_applies_compressor_filter() {
        let base_scene = test_audio_scene(Vec::new());
        let base = build_audio_graph_runtime_snapshot(&base_scene, 6);
        let input_level = base.sources[0].pre_filter_level_db;
        let scene = test_audio_scene(vec![test_filter(
            "filter-compressor",
            SceneSourceFilterKind::Compressor,
            true,
            0,
            json!({
                "threshold_db": input_level - 6.0,
                "ratio": 3.0,
                "attack_ms": 8.0,
                "release_ms": 120.0,
                "makeup_gain_db": 0.0
            }),
        )]);

        let snapshot = build_audio_graph_runtime_snapshot(&scene, 6);
        let filter = &snapshot.sources[0].filters[0];

        assert_eq!(filter.status, AudioFilterRuntimeStatus::Applied);
        assert!(snapshot.sources[0].level_db < input_level);
        assert!(filter.gain_reduction_db.unwrap_or_default() > 0.0);
        assert!(filter
            .control_summary
            .as_deref()
            .unwrap_or_default()
            .contains("ratio 3.0:1"));
    }

    #[test]
    fn audio_graph_runtime_skips_disabled_filters() {
        let scene = test_audio_scene(vec![test_filter(
            "filter-disabled",
            SceneSourceFilterKind::AudioGain,
            false,
            0,
            json!({ "gain_db": 12.0 }),
        )]);

        let snapshot = build_audio_graph_runtime_snapshot(&scene, 2);
        let source = &snapshot.sources[0];

        assert_eq!(source.filters[0].status, AudioFilterRuntimeStatus::Skipped);
        assert_approx_eq(source.level_db, source.pre_filter_level_db);
    }

    #[test]
    fn audio_graph_runtime_reports_malformed_audio_filters_without_mutating_level() {
        let scene = test_audio_scene(vec![test_filter(
            "filter-bad-gain",
            SceneSourceFilterKind::AudioGain,
            true,
            0,
            json!({ "gain_db": 99.0 }),
        )]);

        let snapshot = build_audio_graph_runtime_snapshot(&scene, 2);
        let source = &snapshot.sources[0];

        assert_eq!(source.filters[0].status, AudioFilterRuntimeStatus::Error);
        assert!(source.filters[0].status_detail.contains("between"));
        assert_approx_eq(source.level_db, source.pre_filter_level_db);
    }

    #[test]
    fn audio_graph_runtime_applies_filters_in_deterministic_order() {
        let base_scene = test_audio_scene(Vec::new());
        let base = build_audio_graph_runtime_snapshot(&base_scene, 9);
        let input_level = base.sources[0].pre_filter_level_db;
        let compressor_config = json!({
            "threshold_db": input_level + 5.0,
            "ratio": 2.0,
            "attack_ms": 8.0,
            "release_ms": 120.0,
            "makeup_gain_db": 0.0
        });
        let reversed_input = test_audio_scene(vec![
            test_filter(
                "filter-compressor",
                SceneSourceFilterKind::Compressor,
                true,
                20,
                compressor_config,
            ),
            test_filter(
                "filter-gain",
                SceneSourceFilterKind::AudioGain,
                true,
                10,
                json!({ "gain_db": 12.0 }),
            ),
        ]);

        let snapshot = build_audio_graph_runtime_snapshot(&reversed_input, 9);
        let source = &snapshot.sources[0];

        assert_eq!(source.filters[0].id, "filter-gain");
        assert_eq!(source.filters[1].id, "filter-compressor");
        assert!(source.level_db < input_level + 12.0);
        assert!(source.filters[1].gain_reduction_db.unwrap_or_default() > 0.0);
    }

    #[test]
    fn audio_graph_runtime_skips_non_audio_filters_on_audio_sources() {
        let scene = test_audio_scene(vec![test_filter(
            "filter-color",
            SceneSourceFilterKind::ColorCorrection,
            true,
            0,
            json!({ "brightness": 0.5, "contrast": 1.0, "saturation": 1.0, "gamma": 1.0 }),
        )]);

        let snapshot = build_audio_graph_runtime_snapshot(&scene, 2);
        let source = &snapshot.sources[0];

        assert_eq!(source.filters[0].status, AudioFilterRuntimeStatus::Skipped);
        assert!(source.filters[0].status_detail.contains("Non-audio filter"));
        assert_approx_eq(source.level_db, source.pre_filter_level_db);
    }

    fn test_audio_scene(filters: Vec<SceneSourceFilter>) -> crate::Scene {
        let mut collection = crate::SceneCollection::default_collection(crate::now_utc());
        let scene = collection.scenes.get_mut(0).unwrap();
        let source = scene
            .sources
            .iter_mut()
            .find(|source| source.kind == SceneSourceKind::AudioMeter)
            .unwrap();
        source.config["device_id"] = json!("microphone:default");
        source.config["availability"] = json!({
            "state": "available",
            "detail": "Default microphone is available."
        });
        source.filters = filters;
        scene.clone()
    }

    fn test_filter(
        id: &str,
        kind: SceneSourceFilterKind,
        enabled: bool,
        order: i32,
        config: serde_json::Value,
    ) -> SceneSourceFilter {
        SceneSourceFilter {
            id: id.to_string(),
            name: id.to_string(),
            kind,
            enabled,
            order,
            config,
        }
    }

    fn mock_live_audio_input(source: &AudioMixSource, _: u64) -> AudioRuntimeInput {
        AudioRuntimeInput {
            input_mode: AudioGraphInputMode::Live,
            provider_name: "mock-live-audio".to_string(),
            sample_rate: 48_000,
            channels: 1,
            sample_count: 480,
            capture_duration_ms: 10,
            latency_ms: 10.0,
            level_db: -18.0 + source.gain_db,
            peak_db: -12.0 + source.gain_db,
            status: AudioMixSourceStatus::Ready,
            status_detail: "Mock live audio probe captured samples.".to_string(),
        }
    }

    fn mock_unavailable_audio_input(_: &AudioMixSource, _: u64) -> AudioRuntimeInput {
        AudioRuntimeInput {
            input_mode: AudioGraphInputMode::Silent,
            provider_name: "mock-live-audio".to_string(),
            sample_rate: 48_000,
            channels: 1,
            sample_count: 0,
            capture_duration_ms: 0,
            latency_ms: 0.0,
            level_db: -90.0,
            peak_db: -90.0,
            status: AudioMixSourceStatus::Unavailable,
            status_detail: "Mock live audio unavailable.".to_string(),
        }
    }

    fn assert_approx_eq(left: f64, right: f64) {
        assert!(
            (left - right).abs() < 0.000_001,
            "expected {left} to approximately equal {right}"
        );
    }
}
