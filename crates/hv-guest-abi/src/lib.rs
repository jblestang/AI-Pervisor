//! Versioned guest boot ABI shared by the hypervisor and guest partitions.
//!
//! Guests discover their resources exclusively through the boot info blob.

#![no_std]

use hv_types::{GuestPhysAddr, VcpuId, VmId};

/// Current guest ABI version.
pub const GUEST_ABI_VERSION: u32 = 1;

/// Magic bytes for `GuestBootInfoHeader`.
pub const GUEST_BOOT_INFO_MAGIC: [u8; 8] = *b"HVGUEST\0";

/// Hypervisor to guest boot information header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBootInfoHeader {
    /// Magic identifier (`HVGUEST\0`).
    pub magic: [u8; 8],
    /// ABI version number.
    pub version: u32,
    /// Total guest boot info size in bytes.
    pub size: u32,
    /// Assigned VM identifier.
    pub vm_id: VmId,
    /// Initial vCPU identifier.
    pub vcpu_id: VcpuId,
    /// Offset to memory region table.
    pub memory_table_offset: u32,
    /// Number of memory regions.
    pub memory_region_count: u32,
    /// Offset to IPC region table.
    pub ipc_table_offset: u32,
    /// Number of IPC regions.
    pub ipc_region_count: u32,
    /// Offset to device region table.
    pub device_table_offset: u32,
    /// Number of device regions.
    pub device_region_count: u32,
}

/// Guest memory region kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestMemoryKind {
    /// Normal guest RAM.
    Ram = 1,
    /// MMIO region.
    Mmio = 2,
}

/// One guest memory region descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestMemoryRegion {
    /// Region kind.
    pub kind: GuestMemoryKind,
    /// Guest physical base address.
    pub guest_phys: GuestPhysAddr,
    /// Region size in bytes.
    pub size: u64,
}

/// IPC role for a guest mapping.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestIpcRole {
    /// Producer side of an IPC channel.
    Producer = 1,
    /// Consumer side of an IPC channel.
    Consumer = 2,
}

/// One IPC region descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestIpcRegion {
    /// IPC channel identifier.
    pub channel_id: u32,
    /// IPC role for this guest.
    pub role: GuestIpcRole,
    /// Guest physical base address.
    pub guest_phys: GuestPhysAddr,
    /// Mapping size in bytes.
    pub size: u64,
}

/// Device kind exposed to a guest.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestDeviceKind {
    /// Intel e1000 NIC MMIO and registers.
    NicE1000 = 1,
}

/// One device MMIO region descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestDeviceRegion {
    /// Device kind.
    pub kind: GuestDeviceKind,
    /// MMIO guest physical base.
    pub mmio_guest_phys: GuestPhysAddr,
    /// MMIO region size.
    pub mmio_size: u64,
}

/// Returns whether a guest boot info header matches the supported ABI.
pub fn guest_abi_is_compatible(header: &GuestBootInfoHeader) -> bool {
    header.magic == GUEST_BOOT_INFO_MAGIC && header.version == GUEST_ABI_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn guest_header_layout_is_stable() {
        assert_eq!(size_of::<GuestBootInfoHeader>(), 48);
        assert_eq!(align_of::<GuestBootInfoHeader>(), 4);
    }

    #[test]
    fn descriptor_sizes_are_stable() {
        assert_eq!(size_of::<GuestMemoryRegion>(), 24);
        assert_eq!(size_of::<GuestIpcRegion>(), 24);
        assert_eq!(size_of::<GuestDeviceRegion>(), 24);
    }
}
