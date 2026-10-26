//! E2E bench driver (`IF-CFG-BENCH` / `IF-RPT-BENCH`).
//!
//! Prefers Sriov when configured and VFs are present; Inject only when config
//! selects it or `allow_inject_fallback` rewrites the backend (same policy as
//! smoke). While L2 VMLAUNCH is incomplete, Inject measurement runs the real
//! `guest_common` forward path on the host harness (same code as L1 inject
//! smoke). Sriov-with-VFs without a working L2 datapath fails the tool (no
//! silent Inject relabel).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use demo_config::{DemoConfig, VfBackend};
use guest_common::{
    forward_lan_poll, forward_wan_poll, ArenaCursor, InjectNic, SpscRing, Stats, StatsSnapshot,
};
use serde::Serialize;

use crate::qemu::{resolve_datapath_for_launch, NestedError};

/// Normative bench JSON (`IF-RPT-BENCH`).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct BenchReport {
    pub frame_size: u32,
    pub duration_secs: f64,
    pub bidirectional: bool,
    pub backend: String,
    /// Aggregate bidirectional wire rate (sum of both directions), Gbit/s.
    pub gbps: f64,
    /// Aggregate delivered packets per second (WAN TX + LAN TX).
    pub pps: f64,
    /// Loss ratio: `ring_drop_full / (drops + successful ring forwards)`.
    pub loss: f64,
    pub target_gbps: f64,
    pub target_met: bool,
    pub environment_class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wan_vf_bdf: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lan_vf_bdf: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    pub timestamp: String,
    pub frames_delivered: u64,
    pub bytes_delivered: u64,
    pub ring_drops: u64,
    pub gbps_basis: &'static str,
    pub loss_unit: &'static str,
}

impl BenchReport {
    pub fn from_measurement(
        cfg: &DemoConfig,
        backend: VfBackend,
        elapsed: Duration,
        delta: StatsSnapshot,
    ) -> Self {
        let secs = elapsed.as_secs_f64().max(1e-9);
        let frames = delta.wan_tx.saturating_add(delta.lan_tx);
        let bytes = delta.bytes_wan_tx.saturating_add(delta.bytes_lan_tx);
        let fwds = delta
            .ring_fwd_wan_to_lan
            .saturating_add(delta.ring_fwd_lan_to_wan);
        let drops = delta.ring_drop_full;
        let denom = drops.saturating_add(fwds).max(1) as f64;
        let loss = drops as f64 / denom;
        let gbps = (bytes as f64) * 8.0 / secs / 1_000_000_000.0;
        let pps = frames as f64 / secs;
        let target = cfg.bench.target_gbps;
        let backend_s = match backend {
            VfBackend::Sriov => "sriov",
            VfBackend::Inject => "inject",
        }
        .to_string();
        let environment_class = match backend {
            VfBackend::Sriov => "sriov-vf".into(),
            VfBackend::Inject => "inject-harness".into(),
        };
        Self {
            frame_size: cfg.bench.frame_size,
            duration_secs: secs,
            bidirectional: cfg.bench.bidirectional,
            backend: backend_s,
            gbps,
            pps,
            loss,
            target_gbps: target,
            target_met: gbps >= target,
            environment_class,
            wan_vf_bdf: cfg.datapath.wan_vf_bdf.clone(),
            lan_vf_bdf: cfg.datapath.lan_vf_bdf.clone(),
            git_sha: git_sha_short(),
            timestamp: unix_timestamp_isoish(),
            frames_delivered: frames,
            bytes_delivered: bytes,
            ring_drops: drops,
            gbps_basis: "aggregate_bidirectional_tx_bytes",
            loss_unit: "ratio_drops_over_drops_plus_forwards",
        }
    }
}

fn unix_timestamp_isoish() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn git_sha_short() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Resolve report path: `HV_BENCH_REPORT` overrides `[bench].report_path`.
pub fn resolve_report_path(root: &Path, cfg: &DemoConfig) -> PathBuf {
    if let Ok(p) = std::env::var("HV_BENCH_REPORT") {
        let pb = PathBuf::from(p);
        if pb.is_absolute() {
            return pb;
        }
        return root.join(pb);
    }
    let p = PathBuf::from(&cfg.bench.report_path);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

/// Run the Inject datapath for `cfg.bench.duration_secs` using real rings/NIC.
pub fn measure_inject(cfg: &DemoConfig) -> Result<(Duration, StatsSnapshot), NestedError> {
    if cfg.bench.frame_size == 0 {
        return Err(NestedError::Message(
            "bench.frame_size must be > 0".into(),
        ));
    }
    if cfg.bench.duration_secs == 0 {
        return Err(NestedError::Message(
            "bench.duration_secs must be > 0".into(),
        ));
    }
    if cfg.bench.frame_size as u64 > cfg.ring.frame_capacity as u64 {
        return Err(NestedError::Message(format!(
            "bench.frame_size ({}) exceeds ring.frame_capacity ({})",
            cfg.bench.frame_size, cfg.ring.frame_capacity
        )));
    }

    let cap = cfg.ring.capacity_pow2.max(8);
    let q = cfg.ring.capacity_pow2.max(8) as usize;
    let frame_len = cfg.bench.frame_size as usize;

    // Ring header + descriptors need roughly 64 + 8*cap bytes; oversize safely.
    let ring_bytes = (64 + (cap as usize) * 16).next_power_of_two().max(4096);
    let mut ring_w2l_mem = vec![0u8; ring_bytes];
    let mut ring_l2w_mem = vec![0u8; ring_bytes];
    let arena_len = (cfg.ring.frame_capacity as usize)
        .saturating_mul(16)
        .max(65_536);
    let mut arena = vec![0u8; arena_len];
    let mut scratch = vec![0u8; cfg.ring.frame_capacity.max(frame_len as u32) as usize];

    let mut wan_to_lan = SpscRing::new_in(&mut ring_w2l_mem, cap).map_err(|e| {
        NestedError::Message(format!("bench: wan→lan ring init failed: {e:?}"))
    })?;
    let mut lan_to_wan = SpscRing::new_in(&mut ring_l2w_mem, cap).map_err(|e| {
        NestedError::Message(format!("bench: lan→wan ring init failed: {e:?}"))
    })?;

    let mut wan = InjectNic::with_capacity(q);
    let mut lan = InjectNic::with_capacity(q);
    let stats = Stats::new();
    let mut cursor = ArenaCursor::new();

    let frame = vec![0xA5u8; frame_len];
    let start_snap = stats.snapshot();
    let t0 = Instant::now();
    let deadline = t0 + Duration::from_secs(cfg.bench.duration_secs);

    while Instant::now() < deadline {
        // Fill RX as capacity allows; drain TX so queues never stall the path.
        let _ = wan.inject_rx(&frame);
        if cfg.bench.bidirectional {
            let _ = lan.inject_rx(&frame);
        }
        let _ = forward_wan_poll(
            &mut wan,
            &mut wan_to_lan,
            &mut lan_to_wan,
            &mut arena,
            &mut cursor,
            &mut scratch,
            &stats,
        );
        let _ = forward_lan_poll(
            &mut lan,
            &mut wan_to_lan,
            &mut lan_to_wan,
            &mut arena,
            &mut cursor,
            &mut scratch,
            &stats,
        );
        while wan.take_tx().is_some() {}
        while lan.take_tx().is_some() {}
    }

    let elapsed = t0.elapsed();
    let end_snap = stats.snapshot();
    Ok((elapsed, end_snap.delta(start_snap)))
}

fn write_report(path: &Path, report: &BenchReport) -> Result<(), NestedError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            NestedError::Message(format!("mkdir {}: {e}", parent.display()))
        })?;
    }
    let body = serde_json::to_string_pretty(report)
        .map_err(|e| NestedError::Message(format!("serialize bench report: {e}")))?;
    std::fs::write(path, body.as_bytes())
        .map_err(|e| NestedError::Message(format!("write {}: {e}", path.display())))?;
    Ok(())
}

/// `cargo xtask bench` entry point.
pub fn cmd_bench(root: &Path, mut cfg: DemoConfig) -> ExitCode {
    let backend = match resolve_datapath_for_launch(&mut cfg) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("xtask bench: {e}");
            return ExitCode::FAILURE;
        }
    };

    eprintln!(
        "xtask bench: backend={backend:?} frame_size={} duration_secs={} bidirectional={} target_gbps={}",
        cfg.bench.frame_size,
        cfg.bench.duration_secs,
        cfg.bench.bidirectional,
        cfg.bench.target_gbps
    );

    let (elapsed, delta) = match backend {
        VfBackend::Inject => match measure_inject(&cfg) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("xtask bench: measurement failed: {e}");
                return ExitCode::FAILURE;
            }
        },
        VfBackend::Sriov => {
            eprintln!(
                "xtask bench: datapath.backend=sriov with VFs present, but L2 VMLAUNCH \
                 datapath is not yet complete — cannot produce an honest Sriov E2E \
                 measurement.\n  hint: for lab/CI Inject figures use \
                 HV_VF_BACKEND=inject or config/bench.inject.ci.toml; official \
                 Sriov figures need VF + guest VMLAUNCH (see docs/HARNESS.md)"
            );
            return ExitCode::FAILURE;
        }
    };

    let report = BenchReport::from_measurement(&cfg, backend, elapsed, delta);
    let path = resolve_report_path(root, &cfg);
    if let Err(e) = write_report(&path, &report) {
        eprintln!("xtask bench: {e}");
        return ExitCode::FAILURE;
    }

    println!(
        "xtask bench: gbps={:.6} pps={:.0} loss={:.6} target_met={} → {}",
        report.gbps,
        report.pps,
        report.loss,
        report.target_met,
        path.display()
    );
    // Exit 0 whenever measurement completed — target miss is reported, not a tool failure.
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use demo_config::DemoConfig;

    fn inject_cfg() -> DemoConfig {
        let s = r#"
[lab]
qemu_bin = "qemu-system-x86_64"
memory_mb = 512
smp = 1
enable_iommu = false

[datapath]
backend = "inject"
allow_inject_fallback = true

[guests]
wan_memory_mb = 64
lan_memory_mb = 64
wan_vcpus = 1
lan_vcpus = 1

[ring]
capacity_pow2 = 64
frame_capacity = 2048

[bench]
frame_size = 64
duration_secs = 1
bidirectional = true
target_gbps = 1000.0
report_path = "target/bench-report-test.json"

[hv]
gpa_window_base = 0x10000000
uart_io_port = 0x3F8

[paths]
hv_image = "target/x86_64-unknown-none/debug/hv"
guest_wan_image = "target/x86_64-unknown-none/debug/guest-wan"
guest_lan_image = "target/x86_64-unknown-none/debug/guest-lan"
"#;
        DemoConfig::parse_str(s).expect("cfg")
    }

    #[test]
    fn report_marks_target_miss_without_failing_schema() {
        let cfg = inject_cfg();
        let delta = StatsSnapshot {
            wan_tx: 100,
            lan_tx: 100,
            bytes_wan_tx: 6400,
            bytes_lan_tx: 6400,
            ring_fwd_wan_to_lan: 100,
            ring_fwd_lan_to_wan: 100,
            ring_drop_full: 1,
            ..StatsSnapshot::default()
        };
        let report =
            BenchReport::from_measurement(&cfg, VfBackend::Inject, Duration::from_secs(1), delta);
        assert_eq!(report.frame_size, 64);
        assert!(report.bidirectional);
        assert_eq!(report.backend, "inject");
        assert!(!report.target_met); // 1000 Gbps target unmet
        assert!(report.loss > 0.0);
        assert_eq!(report.loss_unit, "ratio_drops_over_drops_plus_forwards");
        let json = serde_json::to_value(&report).unwrap();
        for key in [
            "frame_size",
            "duration_secs",
            "bidirectional",
            "backend",
            "gbps",
            "pps",
            "loss",
            "target_gbps",
            "target_met",
        ] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
    }

    #[test]
    fn inject_measure_produces_progress() {
        let mut cfg = inject_cfg();
        cfg.bench.duration_secs = 1;
        let (elapsed, delta) = measure_inject(&cfg).expect("measure");
        assert!(elapsed.as_millis() >= 900);
        assert!(
            delta.lan_tx + delta.wan_tx > 0,
            "expected delivered frames, got {delta:?}"
        );
        assert!(delta.bytes_lan_tx + delta.bytes_wan_tx > 0);
    }
}
