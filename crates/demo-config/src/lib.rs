//! Demonstrator configuration loaded from TOML + optional env overrides.
//!
//! No lab/demo constants live in callers: parse a complete file (or refuse).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// PCI VF / inject backend selection (`datapath.backend`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VfBackend {
    Sriov,
    Inject,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LabConfig {
    pub qemu_bin: String,
    pub memory_mb: u64,
    pub smp: u32,
    pub enable_iommu: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DatapathConfig {
    pub backend: VfBackend,
    pub allow_inject_fallback: bool,
    #[serde(default)]
    pub wan_vf_bdf: Option<String>,
    #[serde(default)]
    pub lan_vf_bdf: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GuestsConfig {
    pub wan_memory_mb: u64,
    pub lan_memory_mb: u64,
    pub wan_vcpus: u32,
    pub lan_vcpus: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RingConfig {
    pub capacity_pow2: u32,
    pub frame_capacity: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BenchConfig {
    pub frame_size: u32,
    pub duration_secs: u64,
    pub bidirectional: bool,
    pub target_gbps: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PathsConfig {
    pub hv_image: String,
    pub guest_wan_image: String,
    pub guest_lan_image: String,
}

/// Complete demonstrator configuration (all sections mandatory).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DemoConfig {
    pub lab: LabConfig,
    pub datapath: DatapathConfig,
    pub guests: GuestsConfig,
    pub ring: RingConfig,
    pub bench: BenchConfig,
    pub paths: PathsConfig,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("incomplete config: missing {0}")]
    MissingField(String),
    #[error("invalid config: {0}")]
    Invalid(String),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Required dotted keys in a complete `demo.toml`.
const REQUIRED_KEYS: &[&str] = &[
    "lab.qemu_bin",
    "lab.memory_mb",
    "lab.smp",
    "lab.enable_iommu",
    "datapath.backend",
    "datapath.allow_inject_fallback",
    "guests.wan_memory_mb",
    "guests.lan_memory_mb",
    "guests.wan_vcpus",
    "guests.lan_vcpus",
    "ring.capacity_pow2",
    "ring.frame_capacity",
    "bench.frame_size",
    "bench.duration_secs",
    "bench.bidirectional",
    "bench.target_gbps",
    "paths.hv_image",
    "paths.guest_wan_image",
    "paths.guest_lan_image",
];

fn table_get<'a>(root: &'a toml::Value, path: &str) -> Option<&'a toml::Value> {
    let mut cur = root;
    for part in path.split('.') {
        cur = cur.as_table()?.get(part)?;
    }
    Some(cur)
}

fn collect_missing(root: &toml::Value) -> Vec<&'static str> {
    REQUIRED_KEYS
        .iter()
        .copied()
        .filter(|k| table_get(root, k).is_none())
        .collect()
}

fn validate_semantic(cfg: &DemoConfig) -> Result<(), ConfigError> {
    if cfg.datapath.backend == VfBackend::Sriov {
        let wan = cfg.datapath.wan_vf_bdf.as_deref().unwrap_or("").trim();
        let lan = cfg.datapath.lan_vf_bdf.as_deref().unwrap_or("").trim();
        if wan.is_empty() {
            return Err(ConfigError::MissingField("wan_vf_bdf".into()));
        }
        if lan.is_empty() {
            return Err(ConfigError::MissingField("lan_vf_bdf".into()));
        }
    }
    if cfg.ring.capacity_pow2 == 0 || !cfg.ring.capacity_pow2.is_power_of_two() {
        return Err(ConfigError::Invalid(
            "ring.capacity_pow2 must be a non-zero power of two".into(),
        ));
    }
    Ok(())
}

impl DemoConfig {
    /// Parse a TOML string; refuse incomplete configs.
    pub fn parse_str(s: &str) -> Result<Self, ConfigError> {
        let value: toml::Value = s.parse()?;
        let missing = collect_missing(&value);
        if !missing.is_empty() {
            // Prefer leaf names (matches Task 1 test expectation for `lan_memory_mb`).
            let leaves: Vec<&str> = missing
                .iter()
                .map(|k| k.rsplit('.').next().unwrap_or(k))
                .collect();
            return Err(ConfigError::MissingField(leaves.join(", ")));
        }
        let cfg: DemoConfig = value.try_into()?;
        validate_semantic(&cfg)?;
        Ok(cfg)
    }

    /// Load from a filesystem path (no env overrides).
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path)?;
        Self::parse_str(&text)
    }

    /// Resolve config path from `HV_DEMO_CONFIG` or `default_path`, load, then apply env overrides.
    pub fn load(default_path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = std::env::var_os("HV_DEMO_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| default_path.as_ref().to_path_buf());
        let mut cfg = Self::from_path(&path)?;
        cfg.apply_env_overrides()?;
        Ok(cfg)
    }

    /// Apply `HV_VF_BACKEND`, `HV_WAN_VF_BDF`, `HV_LAN_VF_BDF` if set.
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(backend) = std::env::var("HV_VF_BACKEND") {
            self.datapath.backend = match backend.trim().to_ascii_lowercase().as_str() {
                "sriov" => VfBackend::Sriov,
                "inject" => VfBackend::Inject,
                other => {
                    return Err(ConfigError::Invalid(format!(
                        "HV_VF_BACKEND must be sriov|inject, got {other:?}"
                    )));
                }
            };
        }
        if let Ok(bdf) = std::env::var("HV_WAN_VF_BDF") {
            self.datapath.wan_vf_bdf = Some(bdf);
        }
        if let Ok(bdf) = std::env::var("HV_LAN_VF_BDF") {
            self.datapath.lan_vf_bdf = Some(bdf);
        }
        validate_semantic(self)?;
        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn sriov_requires_bdfs() {
        let mut s = include_str!("../../../config/demo.example.toml").to_string();
        s = s.replace("wan_vf_bdf = \"0000:01:10.0\"", "wan_vf_bdf = \"\"");
        let err = DemoConfig::parse_str(&s).unwrap_err();
        assert!(err.to_string().contains("wan_vf_bdf"));
    }

    #[test]
    fn env_overrides_backend_and_bdfs() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/demo.example.toml");
        // SAFETY: test-only; single-threaded cargo test for this crate unit module.
        std::env::set_var("HV_DEMO_CONFIG", path);
        std::env::set_var("HV_VF_BACKEND", "inject");
        std::env::set_var("HV_WAN_VF_BDF", "0000:aa:00.0");
        std::env::set_var("HV_LAN_VF_BDF", "0000:aa:00.1");
        let c = DemoConfig::load("/nonexistent/default.toml").expect("load via HV_DEMO_CONFIG");
        assert_eq!(c.datapath.backend, VfBackend::Inject);
        assert_eq!(c.datapath.wan_vf_bdf.as_deref(), Some("0000:aa:00.0"));
        assert_eq!(c.datapath.lan_vf_bdf.as_deref(), Some("0000:aa:00.1"));
        std::env::remove_var("HV_DEMO_CONFIG");
        std::env::remove_var("HV_VF_BACKEND");
        std::env::remove_var("HV_WAN_VF_BDF");
        std::env::remove_var("HV_LAN_VF_BDF");
    }

    #[test]
    fn inject_backend_ok_without_bdf() {
        let s = r#"
[lab]
qemu_bin = "qemu-system-x86_64"
memory_mb = 4096
smp = 4
enable_iommu = true

[datapath]
backend = "inject"
allow_inject_fallback = true

[guests]
wan_memory_mb = 128
lan_memory_mb = 128
wan_vcpus = 1
lan_vcpus = 1

[ring]
capacity_pow2 = 1024
frame_capacity = 2048

[bench]
frame_size = 64
duration_secs = 30
bidirectional = true
target_gbps = 1.0

[paths]
hv_image = "target/x86_64-unknown-none/release/hv"
guest_wan_image = "target/x86_64-unknown-none/release/guest-wan"
guest_lan_image = "target/x86_64-unknown-none/release/guest-lan"
"#;
        let c = DemoConfig::parse_str(s).expect("inject config");
        assert_eq!(c.datapath.backend, VfBackend::Inject);
    }
}
