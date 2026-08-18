//! Datapath benchmark fixture tests.

#![allow(clippy::expect_used)]

use hv_config_model::compile_config_from_str;
use hv_datapath::{
    run_mock_datapath_benchmark, DatapathBenchmarkConfig, TARGET_THROUGHPUT_MBIT_PER_SEC,
    BENCHMARK_MEASUREMENT_SECS, BENCHMARK_MIN_RUNS, BENCHMARK_WARMUP_SECS,
};
use hv_platform_model::plan_static_platform_ir;

#[test]
fn mock_benchmark_reports_official_procedure_durations() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let config = DatapathBenchmarkConfig::default();
    assert_eq!(config.warmup_secs, BENCHMARK_WARMUP_SECS);
    assert_eq!(config.measurement_secs, BENCHMARK_MEASUREMENT_SECS);
    assert_eq!(config.min_runs, BENCHMARK_MIN_RUNS);

    let result = run_mock_datapath_benchmark(&layout, &config).expect("benchmark");
    assert_eq!(result.runs_completed, BENCHMARK_MIN_RUNS);
    assert!(result.target_met);
    assert!(result.stats.min_mbit_per_sec >= TARGET_THROUGHPUT_MBIT_PER_SEC);
}
