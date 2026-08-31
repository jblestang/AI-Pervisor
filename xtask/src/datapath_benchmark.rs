//! Host-side datapath throughput benchmark using wall-clock timing.

use std::time::{Duration, Instant};

use hv_config_model::compile_config_from_str;
use hv_datapath::{
    compute_benchmark_run_stats, forward_synthetic_frame, plan_datapath_forward,
    run_mock_datapath_benchmark, throughput_mbit_from_frames, DatapathBenchmarkConfig,
    SYNTHETIC_FRAME_PAYLOAD, TARGET_THROUGHPUT_MBIT_PER_SEC,
};
use hv_platform_model::plan_static_platform_ir;

/// Runs a host wall-clock datapath benchmark for the given configuration path.
pub fn run_datapath_benchmark(config_path: &str) -> i32 {
    let workspace = crate::workspace_root();
    let config = workspace.join(config_path);
    let yaml = match std::fs::read_to_string(&config) {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!("failed to read config {}: {err}", config.display());
            return 1;
        }
    };

    let compiled = match compile_config_from_str(&yaml) {
        Ok(compiled) => compiled,
        Err(err) => {
            eprintln!("failed to compile config {}: {err}", config.display());
            return 1;
        }
    };
    let layout = match plan_static_platform_ir(&compiled.intent) {
        Ok(layout) => layout,
        Err(err) => {
            eprintln!("failed to plan static platform layout: {err}");
            return 1;
        }
    };

    let mock = match run_mock_datapath_benchmark(&layout, &DatapathBenchmarkConfig::default()) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("mock datapath benchmark failed: {}", err.message);
            return 1;
        }
    };

    let config = DatapathBenchmarkConfig::default();
    let warmup_duration = Duration::from_secs(config.warmup_secs as u64);
    let measurement_duration = Duration::from_secs(config.measurement_secs as u64);
    let payload_bytes = SYNTHETIC_FRAME_PAYLOAD.len() as u32;
    let mut run_throughputs = Vec::with_capacity(config.min_runs as usize);

    for run_idx in 0..config.min_runs {
        let mut plan = match plan_datapath_forward(&layout) {
            Ok(plan) => plan,
            Err(err) => {
                eprintln!("failed to plan datapath forward: {}", err.message);
                return 1;
            }
        };

        let warmup_deadline = Instant::now() + warmup_duration;
        while Instant::now() < warmup_deadline {
            if let Err(err) = forward_synthetic_frame(&mut plan) {
                if err.message == "ipc queue full" {
                    plan = match plan_datapath_forward(&layout) {
                        Ok(plan) => plan,
                        Err(err) => {
                            eprintln!("failed to replan datapath forward: {}", err.message);
                            return 1;
                        }
                    };
                    continue;
                }
                eprintln!("warmup forward failed: {}", err.message);
                return 1;
            }
        }

        let mut frames = 0u64;
        let start = Instant::now();
        while start.elapsed() < measurement_duration {
            if let Err(err) = forward_synthetic_frame(&mut plan) {
                if err.message == "ipc queue full" {
                    plan = match plan_datapath_forward(&layout) {
                        Ok(plan) => plan,
                        Err(err) => {
                            eprintln!("failed to replan datapath forward: {}", err.message);
                            return 1;
                        }
                    };
                    continue;
                }
                eprintln!("measurement forward failed: {}", err.message);
                return 1;
            }
            frames = frames.saturating_add(1);
        }
        let elapsed_nanos = start.elapsed().as_nanos() as u64;
        let mbit = match throughput_mbit_from_frames(payload_bytes, frames, elapsed_nanos) {
            Ok(mbit) => mbit,
            Err(err) => {
                eprintln!("throughput calculation failed on run {run_idx}: {}", err.message);
                return 1;
            }
        };
        run_throughputs.push(mbit);
    }

    let stats = compute_benchmark_run_stats(&run_throughputs);
    let target_met = stats.min_mbit_per_sec >= TARGET_THROUGHPUT_MBIT_PER_SEC;

    eprintln!("datapath benchmark (host wall-clock)");
    eprintln!("  config: {}", config_path);
    eprintln!("  payload bytes/frame: {payload_bytes}");
    eprintln!("  warmup/measurement: {}s/{}s", config.warmup_secs, config.measurement_secs);
    eprintln!("  runs: {}", config.min_runs);
    eprintln!(
        "  min/mean/median/p95 Mbit/s: {}/{}/{}/{}",
        stats.min_mbit_per_sec,
        stats.mean_mbit_per_sec,
        stats.median_mbit_per_sec,
        stats.p95_mbit_per_sec
    );
    eprintln!(
        "  mock validate-only min Mbit/s: {}",
        mock.stats.min_mbit_per_sec
    );
    eprintln!(
        "  target {} Mbit/s: {}",
        TARGET_THROUGHPUT_MBIT_PER_SEC,
        if target_met { "met" } else { "NOT MET" }
    );

    if target_met { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use hv_datapath::{compute_benchmark_run_stats, DatapathBenchmarkRunStats};

    #[test]
    fn compute_benchmark_run_stats_reports_min_mean_median_p95() {
        let stats = compute_benchmark_run_stats(&[100, 200, 300, 400, 500]);
        assert_eq!(
            stats,
            DatapathBenchmarkRunStats {
                min_mbit_per_sec: 100,
                mean_mbit_per_sec: 300,
                median_mbit_per_sec: 300,
                p95_mbit_per_sec: 500,
            }
        );
    }
}
