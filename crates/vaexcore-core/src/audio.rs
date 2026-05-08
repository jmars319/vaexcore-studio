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
}
