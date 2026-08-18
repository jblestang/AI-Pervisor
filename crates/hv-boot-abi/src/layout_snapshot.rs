//! Embedded static platform layout snapshot for UEFI Gate C boot.

/// Maximum guest private memory regions stored in a layout snapshot.
pub const MAX_LAYOUT_GUEST_REGIONS: usize = 8;

/// Maximum IPC shared memory regions stored in a layout snapshot.
pub const MAX_LAYOUT_IPC_REGIONS: usize = 8;

/// Maximum PCI devices stored in a layout snapshot.
pub const MAX_LAYOUT_PCI_DEVICES: usize = 8;

/// Layout snapshot device kind for Intel e1000 NIC assignments.
pub const LAYOUT_DEVICE_KIND_NIC_E1000: u32 = 1;

/// One planned host memory region for Gate C EPT planning.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedRegionSnapshot {
    /// Host physical base address.
    pub host_phys: u64,
    /// Region size in bytes.
    pub size_bytes: u64,
}

/// Guest private memory region stored in a layout snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutGuestRegionSnapshot {
    /// Owning VM id.
    pub vm_id: u32,
    /// Host physical base address.
    pub host_phys: u64,
    /// Region size in bytes.
    pub size_bytes: u64,
}

/// IPC shared memory region stored in a layout snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutIpcRegionSnapshot {
    /// Assigned IPC channel id.
    pub channel_id: u32,
    /// Producer VM id for the channel.
    pub producer_vm_id: u32,
    /// Consumer VM id for the channel.
    pub consumer_vm_id: u32,
    /// Host physical base address.
    pub host_phys: u64,
    /// Region size in bytes.
    pub size_bytes: u64,
}

/// PCI device assignment stored in a layout snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutPciSnapshot {
    /// Owning VM id.
    pub vm_id: u32,
    /// PCI segment number.
    pub segment: u16,
    /// PCI bus number.
    pub bus: u8,
    /// PCI device number.
    pub device: u8,
    /// PCI function number.
    pub function: u8,
    /// Compact device kind discriminator.
    pub device_kind: u32,
}

/// Fixed-size static layout snapshot embedded in the hypervisor image.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutSnapshot {
    /// Number of valid entries in [`Self::guest_regions`].
    pub guest_region_count: u32,
    /// Guest private memory regions sorted by host address.
    pub guest_regions: [LayoutGuestRegionSnapshot; MAX_LAYOUT_GUEST_REGIONS],
    /// Number of valid entries in [`Self::ipc_regions`].
    pub ipc_region_count: u32,
    /// IPC shared memory regions sorted by host address.
    pub ipc_regions: [LayoutIpcRegionSnapshot; MAX_LAYOUT_IPC_REGIONS],
    /// Number of valid entries in [`Self::pci_devices`].
    pub pci_device_count: u32,
    /// PCI devices assigned to guest partitions.
    pub pci_devices: [LayoutPciSnapshot; MAX_LAYOUT_PCI_DEVICES],
    /// Planned host physical base for the hypervisor private reserve.
    pub hypervisor_reserve_phys: u64,
    /// Planned hypervisor reserve size in bytes.
    pub hypervisor_reserve_bytes: u64,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn layout_snapshot_layout_is_stable() {
        assert_eq!(size_of::<PlannedRegionSnapshot>(), 16);
        assert_eq!(size_of::<LayoutGuestRegionSnapshot>(), 24);
        assert_eq!(size_of::<LayoutIpcRegionSnapshot>(), 32);
        assert_eq!(size_of::<LayoutPciSnapshot>(), 16);
        assert_eq!(align_of::<LayoutSnapshot>(), 8);
        assert!(size_of::<LayoutSnapshot>() > size_of::<LayoutGuestRegionSnapshot>());
    }
}
