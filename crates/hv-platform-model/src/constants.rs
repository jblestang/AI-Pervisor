//! Platform planning and layout constants.

use hv_types::HostPhysAddr;

/// Host physical base address for statically planned guest and IPC regions.
pub const PLATFORM_PHYS_BASE: u64 = 0x0000_0001_0000_0000;

/// Minimum alignment applied between planned memory regions.
pub const REGION_ALIGNMENT_BYTES: u64 = 4096;

/// Returns the platform physical base as a typed host address.
pub const fn platform_phys_base() -> HostPhysAddr {
    HostPhysAddr::new(PLATFORM_PHYS_BASE)
}
