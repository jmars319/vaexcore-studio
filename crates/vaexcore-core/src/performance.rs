use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{CompositorRenderPlan, CompositorRenderTarget, CompositorRenderTargetKind};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PerformanceTargetBudget {
    pub target_id: String,
    pub target_name: String,
    pub target_kind: CompositorRenderTargetKind,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub frame_budget_nanos: u64,
    pub render_budget_nanos: u64,
    pub encode_budget_nanos: u64,
    pub max_latency_ms: u32,
    pub max_dropped_frames_per_minute: u32,
    pub pixel_count: u64,
    pub estimated_rgba_bytes_per_frame: u64,
    pub estimated_rgba_bytes_per_second: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PerformanceTelemetryPlan {
    pub version: u32,
    pub scene_id: String,
    pub scene_name: String,
    pub sample_window_seconds: u32,
    pub cpu_warning_percent: f64,
    pub gpu_warning_percent: f64,
    pub targets: Vec<PerformanceTargetBudget>,
    pub validation: PerformanceTelemetryValidation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PerformanceTelemetryValidation {
    pub ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn build_performance_telemetry_plan(
    render_plan: &CompositorRenderPlan,
) -> PerformanceTelemetryPlan {
    let targets = render_plan
        .targets
        .iter()
        .filter(|target| target.enabled)
        .map(performance_target_budget)
        .collect::<Vec<_>>();
    let mut plan = PerformanceTelemetryPlan {
        version: 1,
        scene_id: render_plan.graph.scene_id.clone(),
        scene_name: render_plan.graph.scene_name.clone(),
        sample_window_seconds: 10,
        cpu_warning_percent: 85.0,
        gpu_warning_percent: 85.0,
        targets,
        validation: PerformanceTelemetryValidation {
            ready: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        },
    };
    plan.validation = validate_performance_telemetry_plan(&plan);
    plan
}

pub fn validate_performance_telemetry_plan(
    plan: &PerformanceTelemetryPlan,
) -> PerformanceTelemetryValidation {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut target_ids = HashSet::new();

    if plan.version == 0 {
        errors.push("performance telemetry plan version must be greater than zero".to_string());
    }
    if plan.scene_id.trim().is_empty() {
        errors.push("performance telemetry scene id is required".to_string());
    }
    if plan.scene_name.trim().is_empty() {
        errors.push("performance telemetry scene name is required".to_string());
    }
    if plan.sample_window_seconds == 0 {
        errors.push("performance telemetry sample window must be greater than zero".to_string());
    }
    if !plan.cpu_warning_percent.is_finite()
        || plan.cpu_warning_percent <= 0.0
        || plan.cpu_warning_percent > 100.0
    {
        errors.push("performance telemetry CPU warning percent must be 1-100".to_string());
    }
    if !plan.gpu_warning_percent.is_finite()
        || plan.gpu_warning_percent <= 0.0
        || plan.gpu_warning_percent > 100.0
    {
        errors.push("performance telemetry GPU warning percent must be 1-100".to_string());
    }
    if plan.targets.is_empty() {
        warnings.push("performance telemetry has no enabled render targets".to_string());
    }

    for target in &plan.targets {
        if !target_ids.insert(target.target_id.as_str()) {
            errors.push(format!(
                "duplicate performance target id \"{}\"",
                target.target_id
            ));
        }
        if target.target_id.trim().is_empty() {
            errors.push("performance target id is required".to_string());
        }
        if target.target_name.trim().is_empty() {
            errors.push(format!(
                "performance target \"{}\" name is required",
                target.target_id
            ));
        }
        if target.width == 0 || target.height == 0 {
            errors.push(format!(
                "performance target \"{}\" dimensions must be greater than zero",
                target.target_id
            ));
        }
        if target.framerate == 0 {
            errors.push(format!(
                "performance target \"{}\" framerate must be greater than zero",
                target.target_id
            ));
        }
        if target.frame_budget_nanos == 0 || target.render_budget_nanos == 0 {
            errors.push(format!(
                "performance target \"{}\" frame budget must be greater than zero",
                target.target_id
            ));
        }
        if target.framerate > 120 {
            warnings.push(format!(
                "{} targets {} fps; validate frame pacing on target hardware",
                target.target_name, target.framerate
            ));
        }
        if target.estimated_rgba_bytes_per_frame > 33_177_600 {
            warnings.push(format!(
                "{} exceeds a 4K RGBA frame budget; validate GPU and encoder load",
                target.target_name
            ));
        }
    }

    let total_rgba_bytes_per_second = plan
        .targets
        .iter()
        .map(|target| target.estimated_rgba_bytes_per_second)
        .sum::<u64>();
    if total_rgba_bytes_per_second > 2_000_000_000 {
        warnings.push(format!(
            "estimated RGBA throughput is {} MB/s across enabled targets",
            total_rgba_bytes_per_second / 1_000_000
        ));
    }

    PerformanceTelemetryValidation {
        ready: errors.is_empty(),
        warnings,
        errors,
    }
}

fn performance_target_budget(target: &CompositorRenderTarget) -> PerformanceTargetBudget {
    let frame_budget_nanos = frame_budget_nanos(target.framerate);
    let pixel_count = u64::from(target.width) * u64::from(target.height);
    let estimated_rgba_bytes_per_frame = pixel_count * 4;

    PerformanceTargetBudget {
        target_id: target.id.clone(),
        target_name: target.name.clone(),
        target_kind: target.kind.clone(),
        width: target.width,
        height: target.height,
        framerate: target.framerate,
        frame_budget_nanos,
        render_budget_nanos: frame_budget_nanos * 70 / 100,
        encode_budget_nanos: frame_budget_nanos * 20 / 100,
        max_latency_ms: ceil_div_u64(frame_budget_nanos * 2, 1_000_000) as u32,
        max_dropped_frames_per_minute: ((u64::from(target.framerate) * 60) / 200).max(1) as u32,
        pixel_count,
        estimated_rgba_bytes_per_frame,
        estimated_rgba_bytes_per_second: estimated_rgba_bytes_per_frame
            * u64::from(target.framerate),
    }
}

fn frame_budget_nanos(framerate: u32) -> u64 {
    1_000_000_000 / u64::from(framerate.max(1))
}

fn ceil_div_u64(value: u64, divisor: u64) -> u64 {
    value.div_ceil(divisor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build_compositor_graph, build_compositor_render_plan, compositor_render_target,
        CompositorRenderTargetKind, SceneCollection,
    };

    #[test]
    fn performance_plan_describes_enabled_render_targets() {
        let collection = SceneCollection::default_collection(crate::now_utc());
        let scene = collection.active_scene().unwrap();
        let graph = build_compositor_graph(scene);
        let plan = build_compositor_render_plan(
            &graph,
            vec![compositor_render_target(
                "target-program",
                "Program",
                CompositorRenderTargetKind::Program,
                1920,
                1080,
                60,
            )],
        );

        let telemetry = build_performance_telemetry_plan(&plan);

        assert!(telemetry.validation.ready, "{:?}", telemetry.validation);
        assert_eq!(telemetry.scene_id, "scene-main");
        assert_eq!(telemetry.targets.len(), 1);
        assert_eq!(telemetry.targets[0].frame_budget_nanos, 16_666_666);
        assert_eq!(telemetry.targets[0].render_budget_nanos, 11_666_666);
        assert_eq!(telemetry.targets[0].max_dropped_frames_per_minute, 18);
        assert_eq!(
            telemetry.targets[0].estimated_rgba_bytes_per_frame,
            8_294_400
        );
    }

    #[test]
    fn performance_validation_warns_on_heavy_targets() {
        let mut telemetry = PerformanceTelemetryPlan {
            version: 1,
            scene_id: "scene-main".to_string(),
            scene_name: "Main".to_string(),
            sample_window_seconds: 10,
            cpu_warning_percent: 85.0,
            gpu_warning_percent: 85.0,
            targets: vec![PerformanceTargetBudget {
                target_id: "target-8k".to_string(),
                target_name: "8K Program".to_string(),
                target_kind: CompositorRenderTargetKind::Program,
                width: 7680,
                height: 4320,
                framerate: 144,
                frame_budget_nanos: 6_944_444,
                render_budget_nanos: 4_861_110,
                encode_budget_nanos: 1_388_888,
                max_latency_ms: 14,
                max_dropped_frames_per_minute: 43,
                pixel_count: 33_177_600,
                estimated_rgba_bytes_per_frame: 132_710_400,
                estimated_rgba_bytes_per_second: 19_110_297_600,
            }],
            validation: PerformanceTelemetryValidation {
                ready: true,
                warnings: Vec::new(),
                errors: Vec::new(),
            },
        };

        telemetry.validation = validate_performance_telemetry_plan(&telemetry);

        assert!(telemetry.validation.ready);
        assert!(telemetry
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("144 fps")));
        assert!(telemetry
            .validation
            .warnings
            .iter()
            .any(|warning| warning.contains("throughput")));
    }
}
