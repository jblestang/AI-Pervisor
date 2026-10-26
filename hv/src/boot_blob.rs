//! Fill [`guest_common::BootInfo`] from flat config + layout (ICD `IF-BOOT-*`).

use guest_common::{BootInfo, GuestRole, NicBackendKind, BOOT_INFO_MAGIC, BOOT_INFO_VERSION};

use crate::config::FlatHvConfig;
use crate::guest_mem::GuestLayout;

/// Construct a boot-info blob for `role` using only config + derived layout GPAs.
pub fn fill_boot_info(cfg: &FlatHvConfig, layout: &GuestLayout, role: GuestRole) -> BootInfo {
    let boot_info_gpa = match role {
        GuestRole::Wan => layout.boot_info_wan.base,
        GuestRole::Lan => layout.boot_info_lan.base,
    };
    BootInfo {
        magic: BOOT_INFO_MAGIC,
        version: BOOT_INFO_VERSION,
        guest_role: role,
        ring_wan_to_lan_gpa: layout.ring_wan_to_lan.base,
        ring_lan_to_wan_gpa: layout.ring_lan_to_wan.base,
        ring_capacity_pow2: cfg.ring_capacity_pow2,
        _ring_pad: 0,
        arena_gpa: layout.arena.base,
        arena_len: layout.arena.size,
        stats_gpa: layout.stats.base,
        boot_info_gpa,
        nic_backend: if cfg.nic_is_inject() {
            NicBackendKind::Inject
        } else {
            NicBackendKind::Sriov
        },
        nic_mmio_gpa: 0,
        nic_mmio_len: 0,
        inject_queue_capacity: cfg.inject_queue_capacity,
        _nic_pad: 0,
        uart_port: cfg.uart_io_port,
        _uart_pad: 0,
        uart_hypercall_id: 0,
    }
}
