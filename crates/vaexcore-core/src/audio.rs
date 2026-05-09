use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{CaptureSourceKind, Scene, SceneSource, SceneSourceKind};

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
    pub level_db: f64,
    pub peak_db: f64,
    pub linear_level: f64,
    pub status: AudioMixSourceStatus,
    pub status_detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioGraphRuntimeBus {
    pub id: String,
    pub name: String,
    pub kind: AudioMixBusKind,
    pub gain_db: f64,
    pub muted: bool,
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
    let plan = build_audio_mixer_plan(scene);
    let sources = plan
        .sources
        .iter()
        .map(|source| audio_graph_runtime_source(source, frame_index))
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
        validate_level(source.level_db, &source.name, &mut errors);
        validate_level(source.peak_db, &source.name, &mut errors);
        if !source.linear_level.is_finite()
            || source.linear_level < 0.0
            || source.linear_level > 1.0
        {
            errors.push(format!(
                "{} linear level must be between zero and one",
                source.name
            ));
        }
    }

    for bus in &snapshot.buses {
        validate_level(bus.level_db, &bus.name, &mut errors);
        validate_level(bus.peak_db, &bus.name, &mut errors);
        if !bus.linear_level.is_finite() || bus.linear_level < 0.0 || bus.linear_level > 1.0 {
            errors.push(format!(
                "{} linear bus level must be between zero and one",
                bus.name
            ));
        }
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
    }
}

fn audio_graph_runtime_source(
    source: &AudioMixSource,
    frame_index: u64,
) -> AudioGraphRuntimeSource {
    let (level_db, peak_db, linear_level) = simulated_audio_level(source, frame_index);
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
        level_db,
        peak_db,
        linear_level,
        status: source.status.clone(),
        status_detail: source.status_detail.clone(),
    }
}

fn audio_graph_runtime_bus(
    bus: &AudioMixBus,
    sources: &[AudioGraphRuntimeSource],
) -> AudioGraphRuntimeBus {
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
        level_db: level_db.clamp(-90.0, 6.0),
        peak_db: peak_db.clamp(-90.0, 6.0),
        linear_level,
    }
}

fn simulated_audio_level(source: &AudioMixSource, frame_index: u64) -> (f64, f64, f64) {
    if source.muted || !source.meter_enabled {
        return (-90.0, -90.0, 0.0);
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
    (level_db, peak_db, db_to_linear(level_db))
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

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
