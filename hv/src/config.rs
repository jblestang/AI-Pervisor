//! Flattened hypervisor config — values originate from `DemoConfig`, never lab literals in call sites.

/// Flat bring-up parameters consumed by layout / EPT / boot-info builders.
///
/// Built by the host (`FlatHvConfig::from_demo`) or passed in a boot blob by the
/// nested loader (Task 7). Freestanding `hv` never parses TOML itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlatHvConfig {
    pub wan_memory_mb: u64,
    pub lan_memory_mb: u64,
    pub wan_vcpus: u32,
    pub lan_vcpus: u32,
    pub ring_capacity_pow2: u32,
    pub frame_capacity: u32,
    pub gpa_window_base: u64,
    pub uart_io_port: u16,
    /// Inject queue tip capacity mirrored into boot info (0 = unspecified).
    pub inject_queue_capacity: u32,
    pub nic_backend_inject: bool,
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
            inject_queue_capacity: cfg.ring.capacity_pow2,
            nic_backend_inject: matches!(cfg.datapath.backend, VfBackend::Inject)
                || cfg.datapath.allow_inject_fallback,
        }
    }
}
