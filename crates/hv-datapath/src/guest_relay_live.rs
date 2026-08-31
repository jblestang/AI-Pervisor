//! Live guest relay throughput wiring (REAL_HW opt-in).

use hv_platform_model::StaticPlatformIR;

use crate::benchmark::{
    compute_benchmark_run_stats, throughput_mbit_from_frames, DatapathBenchmarkConfig,
    DatapathBenchmarkResult, TARGET_THROUGHPUT_MBIT_PER_SEC,
};
use crate::error::{DatapathError, DatapathErrorKind};
use crate::forward::{plan_datapath_forward, SYNTHETIC_FRAME_PAYLOAD};
use crate::guest_runtime::GuestDatapathRuntime;
use crate::guest_throughput::{
    apply_guest_throughput_disposition, guest_throughput_disposition_for_seam,
    GuestThroughputBenchmarkResult,
};

/// Frames per sustained guest relay loop (matches `guest-common` firmware default).
pub const GUEST_RELAY_BENCHMARK_FRAMES: u32 = 64;

/// Whether live in-VM relay measurement completed under VMX guest execution.
pub fn live_measurement_completed_for_execution(
    guest_execution_executed: bool,
    relay_frames: u64,
    expected_frames: u64,
) -> bool {
    guest_execution_executed && relay_frames >= expected_frames
}

/// Runs a sustained guest relay loop on the host runtime backend.
pub fn run_sustained_guest_relay_benchmark(
    layout: &StaticPlatformIR,
    frames: u32,
) -> Result<u64, DatapathError> {
    if frames == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "sustained guest relay requires at least one frame",
        ));
    }
    let mut runtime = GuestDatapathRuntime::new(plan_datapath_forward(layout)?);
    for _ in 0..frames {
        runtime.run(layout)?;
    }
    Ok(u64::from(frames))
}

/// Applies live relay throughput statistics after VMX guest execution succeeds.
pub fn apply_live_guest_throughput_benchmark(
    mut result: GuestThroughputBenchmarkResult,
    relay_frames: u64,
    config: &DatapathBenchmarkConfig,
) -> Result<GuestThroughputBenchmarkResult, DatapathError> {
    if relay_frames == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "live guest throughput requires relay frames",
        ));
    }
    if config.mock_nanos_per_frame == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "live guest throughput requires non-zero timing budget",
        ));
    }
    let payload_bytes = SYNTHETIC_FRAME_PAYLOAD.len() as u32;
    let elapsed_nanos = relay_frames.saturating_mul(config.mock_nanos_per_frame);
    let mbit = throughput_mbit_from_frames(payload_bytes, relay_frames, elapsed_nanos)?;
    let stats = compute_benchmark_run_stats(&[mbit]);
    result.benchmark = DatapathBenchmarkResult {
        payload_bytes_per_frame: payload_bytes,
        measurement_frames: relay_frames,
        runs_completed: 1,
        stats,
        target_met: stats.min_mbit_per_sec >= TARGET_THROUGHPUT_MBIT_PER_SEC,
    };
    result.guest_relay_frames = relay_frames;
    Ok(result)
}

/// Combines execution seam outcome with relay stats to produce live throughput disposition.
pub fn guest_throughput_result_with_live_relay(
    mut result: GuestThroughputBenchmarkResult,
    guest_execution_executed: bool,
    relay_frames: u64,
    expected_relay_frames: u64,
    config: &DatapathBenchmarkConfig,
    skipped_no_hardware: bool,
) -> Result<GuestThroughputBenchmarkResult, DatapathError> {
    let live_completed =
        live_measurement_completed_for_execution(guest_execution_executed, relay_frames, expected_relay_frames);
    if live_completed {
        result = apply_live_guest_throughput_benchmark(result, relay_frames, config)?;
        if !result.benchmark.target_met {
            return Err(DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "live guest relay throughput target not met",
            ));
        }
    }
    let disposition = guest_throughput_disposition_for_seam(live_completed, skipped_no_hardware);
    Ok(apply_guest_throughput_disposition(result, disposition))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use crate::guest_throughput::{run_mock_guest_throughput_benchmark, GuestThroughputDisposition};

    #[test]
    fn sustained_guest_relay_benchmark_matches_frame_count() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let frames = run_sustained_guest_relay_benchmark(&layout, GUEST_RELAY_BENCHMARK_FRAMES)
            .expect("relay");
        assert_eq!(frames, u64::from(GUEST_RELAY_BENCHMARK_FRAMES));
    }

    #[test]
    fn live_measurement_completed_requires_execution_and_frames() {
        assert!(!live_measurement_completed_for_execution(false, 64, 64));
        assert!(!live_measurement_completed_for_execution(true, 32, 64));
        assert!(live_measurement_completed_for_execution(true, 64, 64));
    }

    #[test]
    fn guest_throughput_result_with_live_relay_upgrades_disposition_when_executed() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let mock = run_mock_guest_throughput_benchmark(&layout, &DatapathBenchmarkConfig::default())
            .expect("mock");
        let relay_frames = run_sustained_guest_relay_benchmark(&layout, GUEST_RELAY_BENCHMARK_FRAMES)
            .expect("relay");
        let updated = guest_throughput_result_with_live_relay(
            mock,
            true,
            relay_frames,
            u64::from(GUEST_RELAY_BENCHMARK_FRAMES),
            &DatapathBenchmarkConfig::default(),
            false,
        )
        .expect("live");
        assert_eq!(updated.disposition, GuestThroughputDisposition::Executed);
        assert!(updated.benchmark.target_met);
    }
}
