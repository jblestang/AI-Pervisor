//! Flattened hypervisor config — values originate from `DemoConfig`, never lab literals in call sites.

/// Magic for the Multiboot/initrd flat-config blob (`docs/HARNESS.md`).
pub const FLAT_HV_MAGIC: u32 = u32::from_le_bytes(*b"FLAT");
/// Schema version for [`FlatHvConfigBlob`].
pub const FLAT_HV_VERSION: u32 = 1;

/// Flat bring-up parameters consumed by layout / EPT / boot-info builders.
///
/// Built by the host (`FlatHvConfig::from_demo`) or passed in a boot blob by the
/// nested loader (Task 7). Freestanding `hv` never parses TOML itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct FlatHvConfig {
    pub wan_memory_mb: u64,
    pub lan_memory_mb: u64,
    pub wan_vcpus: u32,
    pub lan_vcpus: u32,
    pub ring_capacity_pow2: u32,
    pub frame_capacity: u32,
    pub gpa_window_base: u64,
    pub uart_io_port: u16,
    pub _uart_pad: u16,
    /// Inject queue tip capacity mirrored into boot info (0 = unspecified).
    pub inject_queue_capacity: u32,
    /// Non-zero when the *resolved* backend is Inject (after VF open policy).
    pub nic_backend_inject: u32,
}

/// Multiboot module 0 payload: magic + version + flat config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct FlatHvConfigBlob {
    pub magic: u32,
    pub version: u32,
    pub cfg: FlatHvConfig,
}

impl FlatHvConfigBlob {
    pub fn new(cfg: FlatHvConfig) -> Self {
        Self {
            magic: FLAT_HV_MAGIC,
            version: FLAT_HV_VERSION,
            cfg,
        }
    }

    pub fn parse(bytes: &[u8]) -> Option<FlatHvConfig> {
        if bytes.len() < core::mem::size_of::<Self>() {
            return None;
        }
        // SAFETY: layout is `repr(C)` POD; length checked above.
        let blob = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const Self) };
        if blob.magic != FLAT_HV_MAGIC || blob.version != FLAT_HV_VERSION {
            return None;
        }
        Some(blob.cfg)
    }
}

impl FlatHvConfig {
    pub fn nic_is_inject(&self) -> bool {
        self.nic_backend_inject != 0
    }
}

impl FlatHvConfig {
    /// Total vCPUs the demonstrator brings up (one domain per guest by design).
    pub fn total_vcpus(&self) -> u32 {
        self.wan_vcpus.saturating_add(self.lan_vcpus)
    }
}

#[cfg(feature = "std")]
impl FlatHvConfig {
    /// Lift sizes / tips from parsed `DemoConfig` (host / xtask only).
    pub fn from_demo(cfg: &demo_config::DemoConfig) -> Self {
        use demo_config::VfBackend;
        Self {
            wan_memory_mb: cfg.guests.wan_memory_mb,
            lan_memory_mb: cfg.guests.lan_memory_mb,
            wan_vcpus: cfg.guests.wan_vcpus,
            lan_vcpus: cfg.guests.lan_vcpus,
            ring_capacity_pow2: cfg.ring.capacity_pow2,
            frame_capacity: cfg.ring.frame_capacity,
            gpa_window_base: cfg.hv.gpa_window_base,
            uart_io_port: cfg.hv.uart_io_port,
            _uart_pad: 0,
            inject_queue_capacity: cfg.ring.capacity_pow2,
            // Reflect the *resolved* backend only. Callers (xtask) must run
            // `resolve_datapath_for_launch` first so Inject fallback rewrites
            // `datapath.backend` before this lift. Never OR with
            // `allow_inject_fallback` — that flag only authorises a rewrite
            // when VFs are missing; it must not silently mark Sriov as Inject
            // when VFs are present (SWR-NIC-023 / Task 10).
            nic_backend_inject: u32::from(matches!(
                cfg.datapath.backend,
                VfBackend::Inject
            )),
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod flat_backend_tests {
    use super::*;
    use alloc::format;
    use alloc::string::String;
    use demo_config::{DemoConfig, VfBackend};

    fn base_toml(backend: &str, fallback: bool) -> String {
        format!(
            r#"
[lab]
qemu_bin = "qemu-system-x86_64"
memory_mb = 512
smp = 2
enable_iommu = true

[datapath]
backend = "{backend}"
allow_inject_fallback = {fallback}
wan_vf_bdf = "0000:01:10.0"
lan_vf_bdf = "0000:01:10.1"

[guests]
wan_memory_mb = 64
lan_memory_mb = 64
wan_vcpus = 1
lan_vcpus = 1

[ring]
capacity_pow2 = 64
frame_capacity = 256

[bench]
frame_size = 64
duration_secs = 1
bidirectional = true
target_gbps = 1.0

[hv]
gpa_window_base = 0x10000000
uart_io_port = 0x3F8

[paths]
hv_image = "target/x86_64-unknown-none/debug/hv"
guest_wan_image = "target/x86_64-unknown-none/debug/guest-wan"
guest_lan_image = "target/x86_64-unknown-none/debug/guest-lan"
"#
        )
    }

    #[test]
    fn sriov_with_fallback_flag_does_not_mark_inject() {
        // allow_inject_fallback=true must NOT flip nic_backend_inject while
        // backend remains Sriov (VFs present / resolve kept Sriov).
        let cfg = DemoConfig::parse_str(&base_toml("sriov", true)).expect("parse");
        assert_eq!(cfg.datapath.backend, VfBackend::Sriov);
        assert!(cfg.datapath.allow_inject_fallback);
        let flat = FlatHvConfig::from_demo(&cfg);
        assert!(!flat.nic_is_inject(), "must not silent-Inject when Sriov selected");
    }

    #[test]
    fn inject_backend_marks_inject() {
        let cfg = DemoConfig::parse_str(&base_toml("inject", true)).expect("parse");
        let flat = FlatHvConfig::from_demo(&cfg);
        assert!(flat.nic_is_inject());
    }

    #[test]
    fn sriov_without_fallback_marks_sriov() {
        let cfg = DemoConfig::parse_str(&base_toml("sriov", false)).expect("parse");
        let flat = FlatHvConfig::from_demo(&cfg);
        assert!(!flat.nic_is_inject());
    }
}
