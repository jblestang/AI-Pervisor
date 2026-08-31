//! In-VM guest datapath throughput benchmark (mock default, live REAL_HW opt-in).

use alloc::vec::Vec;

use hv_platform_model::StaticPlatformIR;

use crate::benchmark::{
    compute_benchmark_run_stats, DatapathBenchmarkConfig, DatapathBenchmarkResult,
    TARGET_THROUGHPUT_MBIT_PER_SEC,
};
use crate::error::{DatapathError, DatapathErrorKind};
use crate::forward::{plan_datapath_forward, SYNTHETIC_FRAME_PAYLOAD};
use crate::guest_runtime::GuestDatapathRuntime;

/// How guest throughput measurement completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestThroughputDisposition {
    /// Guest relay steps validated without live VM-exit measurement.
    ValidatedOnly,
    /// Guest throughput measured under live VMX.
    Executed,
    /// Live environment unavailable.
    Unavailable,
}

/// Result of an in-VM guest datapath throughput benchmark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestThroughputBenchmarkResult {
    /// Underlying benchmark statistics using the official metric.
    pub benchmark: DatapathBenchmarkResult,
    /// Total in→mid→out guest relay frames across all runs.
    pub guest_relay_frames: u64,
    /// How the guest throughput benchmark completed.
    pub disposition: GuestThroughputDisposition,
}

fn frames_for_duration(secs: u32, frames_per_sec: u64) -> u64 {
    (secs as u64).saturating_mul(frames_per_sec)
}

/// Runs a mock guest throughput benchmark using the guest runtime relay path.
pub fn run_mock_guest_throughput_benchmark(
    layout: &StaticPlatformIR,
    config: &DatapathBenchmarkConfig,
) -> Result<GuestThroughputBenchmarkResult, DatapathError> {
    if config.min_runs == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "guest throughput benchmark requires at least one run",
        ));
    }
    if config.mock_nanos_per_frame == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "mock nanos per frame must be non-zero",
        ));
    }

    let payload_bytes = SYNTHETIC_FRAME_PAYLOAD.len() as u32;
    let warmup_frames = frames_for_duration(config.warmup_secs, config.mock_frames_per_sec);
    let measurement_frames =
        frames_for_duration(config.measurement_secs, config.mock_frames_per_sec);
    let mut run_throughputs = Vec::with_capacity(config.min_runs as usize);
    let mut guest_relay_frames = 0u64;
    let mut runtime = GuestDatapathRuntime::new(plan_datapath_forward(layout)?);

    for _ in 0..config.min_runs {
        for _ in 0..warmup_frames {
            runtime.run(layout)?;
            guest_relay_frames = guest_relay_frames.saturating_add(1);
        }
        for _ in 0..measurement_frames {
            runtime.run(layout)?;
            guest_relay_frames = guest_relay_frames.saturating_add(1);
        }
        let elapsed_nanos = measurement_frames.saturating_mul(config.mock_nanos_per_frame);
        let mbit = crate::benchmark::throughput_mbit_from_frames(
            payload_bytes,
            measurement_frames,
            elapsed_nanos,
        )?;
        run_throughputs.push(mbit);
    }

    let stats = compute_benchmark_run_stats(&run_throughputs);
    let target_met = stats.min_mbit_per_sec >= TARGET_THROUGHPUT_MBIT_PER_SEC;

    Ok(GuestThroughputBenchmarkResult {
        benchmark: DatapathBenchmarkResult {
            payload_bytes_per_frame: payload_bytes,
            measurement_frames,
            runs_completed: config.min_runs,
            stats,
            target_met,
        },
        guest_relay_frames,
        disposition: GuestThroughputDisposition::ValidatedOnly,
    })
}

/// Maps guest throughput seam flags to a benchmark disposition.
pub fn guest_throughput_disposition_for_seam(
    live_measurement_completed: bool,
    skipped_no_hardware: bool,
) -> GuestThroughputDisposition {
    if live_measurement_completed {
        GuestThroughputDisposition::Executed
    } else if skipped_no_hardware {
        GuestThroughputDisposition::Unavailable
    } else {
        GuestThroughputDisposition::ValidatedOnly
    }
}

/// Applies a live seam disposition to a guest throughput benchmark result.
pub fn apply_guest_throughput_disposition(
    mut result: GuestThroughputBenchmarkResult,
    disposition: GuestThroughputDisposition,
) -> GuestThroughputBenchmarkResult {
    result.disposition = disposition;
    result
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn mock_guest_throughput_benchmark_meets_target() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let result =
            run_mock_guest_throughput_benchmark(&layout, &DatapathBenchmarkConfig::default())
                .expect("benchmark");
        assert!(result.benchmark.target_met);
        assert!(result.benchmark.stats.min_mbit_per_sec >= TARGET_THROUGHPUT_MBIT_PER_SEC);
        assert!(result.guest_relay_frames > 0);
        assert_eq!(
            result.disposition,
            GuestThroughputDisposition::ValidatedOnly
        );
    }

    #[test]
    fn guest_throughput_disposition_for_seam_maps_outcomes() {
        assert_eq!(
            guest_throughput_disposition_for_seam(true, false),
            GuestThroughputDisposition::Executed
        );
        assert_eq!(
            guest_throughput_disposition_for_seam(false, true),
            GuestThroughputDisposition::Unavailable
        );
        assert_eq!(
            guest_throughput_disposition_for_seam(false, false),
            GuestThroughputDisposition::ValidatedOnly
        );
    }
}
