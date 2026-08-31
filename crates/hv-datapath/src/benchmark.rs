//! Mock datapath throughput benchmark per `docs/benchmark.md`.

use alloc::vec::Vec;

use hv_platform_model::StaticPlatformIR;

use crate::error::{DatapathError, DatapathErrorKind};
use crate::forward::{forward_synthetic_frame, plan_datapath_forward, SYNTHETIC_FRAME_PAYLOAD};

/// Official target from `docs/benchmark.md` (Mbit/s).
pub const TARGET_THROUGHPUT_MBIT_PER_SEC: u64 = 200;

/// Minimum benchmark runs per `docs/benchmark.md`.
pub const BENCHMARK_MIN_RUNS: u32 = 5;

/// Warmup duration per `docs/benchmark.md` (seconds).
pub const BENCHMARK_WARMUP_SECS: u32 = 10;

/// Measurement duration per `docs/benchmark.md` (seconds).
pub const BENCHMARK_MEASUREMENT_SECS: u32 = 30;

/// Configuration for a mock datapath throughput benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatapathBenchmarkConfig {
    /// Warmup duration in seconds.
    pub warmup_secs: u32,
    /// Measurement duration in seconds.
    pub measurement_secs: u32,
    /// Minimum number of measurement runs.
    pub min_runs: u32,
    /// Mock nanoseconds per forwarded frame for deterministic no_std timing.
    pub mock_nanos_per_frame: u64,
    /// Assumed frames per second when converting seconds to frame counts.
    pub mock_frames_per_sec: u64,
}

impl Default for DatapathBenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_secs: BENCHMARK_WARMUP_SECS,
            measurement_secs: BENCHMARK_MEASUREMENT_SECS,
            min_runs: BENCHMARK_MIN_RUNS,
            // 8-byte payload => throughput_mbit = payload * 8 * 1000 / nanos = 64000/nanos
            mock_nanos_per_frame: 320,
            mock_frames_per_sec: 100,
        }
    }
}

/// Aggregate throughput statistics across benchmark runs (Mbit/s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatapathBenchmarkRunStats {
    /// Minimum observed throughput.
    pub min_mbit_per_sec: u64,
    /// Mean observed throughput.
    pub mean_mbit_per_sec: u64,
    /// Median observed throughput.
    pub median_mbit_per_sec: u64,
    /// 95th percentile observed throughput.
    pub p95_mbit_per_sec: u64,
}

/// Result of a mock datapath throughput benchmark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathBenchmarkResult {
    /// Useful UDP payload bytes per forwarded frame at OUT egress.
    pub payload_bytes_per_frame: u32,
    /// Frames forwarded during each measurement window.
    pub measurement_frames: u64,
    /// Number of completed measurement runs.
    pub runs_completed: u32,
    /// Aggregate statistics across runs.
    pub stats: DatapathBenchmarkRunStats,
    /// Whether the minimum run met the official 200 Mbit/s target.
    pub target_met: bool,
}

/// Computes throughput in Mbit/s from forwarded payload bytes and elapsed time.
pub fn throughput_mbit_from_frames(
    payload_bytes: u32,
    frames: u64,
    elapsed_nanos: u64,
) -> Result<u64, DatapathError> {
    if elapsed_nanos == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "zero elapsed time",
        ));
    }
    let bits = frames
        .saturating_mul(payload_bytes as u64)
        .saturating_mul(8);
    Ok(bits.saturating_mul(1_000) / elapsed_nanos)
}

/// Computes mock throughput from per-frame nanosecond budget.
pub fn mock_throughput_mbit(
    payload_bytes: u32,
    mock_nanos_per_frame: u64,
) -> Result<u64, DatapathError> {
    throughput_mbit_from_frames(payload_bytes, 1, mock_nanos_per_frame)
}

fn frames_for_duration(secs: u32, frames_per_sec: u64) -> u64 {
    (secs as u64).saturating_mul(frames_per_sec)
}

/// Runs a mock datapath benchmark using deterministic per-frame timing.
pub fn run_mock_datapath_benchmark(
    layout: &StaticPlatformIR,
    config: &DatapathBenchmarkConfig,
) -> Result<DatapathBenchmarkResult, DatapathError> {
    if config.min_runs == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "benchmark requires at least one run",
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

    for _ in 0..config.min_runs {
        let mut plan = plan_datapath_forward(layout)?;
        for _ in 0..warmup_frames {
            forward_synthetic_frame(&mut plan)?;
        }
        for _ in 0..measurement_frames {
            forward_synthetic_frame(&mut plan)?;
        }
        let elapsed_nanos = measurement_frames.saturating_mul(config.mock_nanos_per_frame);
        let mbit = throughput_mbit_from_frames(payload_bytes, measurement_frames, elapsed_nanos)?;
        run_throughputs.push(mbit);
    }

    let stats = compute_benchmark_run_stats(&run_throughputs);
    let target_met = stats.min_mbit_per_sec >= TARGET_THROUGHPUT_MBIT_PER_SEC;

    Ok(DatapathBenchmarkResult {
        payload_bytes_per_frame: payload_bytes,
        measurement_frames,
        runs_completed: config.min_runs,
        stats,
        target_met,
    })
}

/// Computes min/mean/median/p95 throughput statistics across benchmark runs.
pub fn compute_benchmark_run_stats(values: &[u64]) -> DatapathBenchmarkRunStats {
    if values.is_empty() {
        return DatapathBenchmarkRunStats {
            min_mbit_per_sec: 0,
            mean_mbit_per_sec: 0,
            median_mbit_per_sec: 0,
            p95_mbit_per_sec: 0,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let min = *sorted.first().unwrap_or(&0);
    let sum: u64 = sorted.iter().sum();
    let mean = if sorted.is_empty() {
        0
    } else {
        sum / sorted.len() as u64
    };
    let median = *sorted.get(sorted.len() / 2).unwrap_or(&0);
    let p95_idx = sorted.len().saturating_mul(95).saturating_add(99) / 100;
    let p95_idx = p95_idx.saturating_sub(1).min(sorted.len().saturating_sub(1));
    let p95 = *sorted.get(p95_idx).unwrap_or(&0);
    DatapathBenchmarkRunStats {
        min_mbit_per_sec: min,
        mean_mbit_per_sec: mean,
        median_mbit_per_sec: median,
        p95_mbit_per_sec: p95,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn default_mock_config_meets_target_throughput() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let result = run_mock_datapath_benchmark(&layout, &DatapathBenchmarkConfig::default())
            .expect("benchmark");
        assert_eq!(result.payload_bytes_per_frame, SYNTHETIC_FRAME_PAYLOAD.len() as u32);
        assert!(result.target_met);
        assert!(result.stats.min_mbit_per_sec >= TARGET_THROUGHPUT_MBIT_PER_SEC);
    }

    #[test]
    fn throughput_mbit_from_frames_matches_mock_budget() {
        let mbit = mock_throughput_mbit(SYNTHETIC_FRAME_PAYLOAD.len() as u32, 320).expect("mbit");
        assert_eq!(mbit, 200);
    }
}
