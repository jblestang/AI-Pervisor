//! Embedded static platform layout snapshot for UEFI Gate C boot.

/// Maximum guest private memory regions stored in a layout snapshot.
pub const MAX_LAYOUT_GUEST_REGIONS: usize = 8;

/// Maximum IPC shared memory regions stored in a layout snapshot.
pub const MAX_LAYOUT_IPC_REGIONS: usize = 8;

/// Maximum PCI devices stored in a layout snapshot.
pub const MAX_LAYOUT_PCI_DEVICES: usize = 8;

/// Layout snapshot device kind for Intel e1000 NIC assignments.
pub const LAYOUT_DEVICE_KIND_NIC_E1000: u32 = 1;

/// PCI device has no datapath role marker.
pub const LAYOUT_DEVICE_ROLE_NONE: u8 = 0;
/// PCI device is the datapath ingress NIC.
pub const LAYOUT_DEVICE_ROLE_DATAPATH_IN: u8 = 1;
/// PCI device is the datapath egress NIC.
pub const LAYOUT_DEVICE_ROLE_DATAPATH_OUT: u8 = 2;

/// Maximum partition id bytes stored in a layout snapshot guest region.
pub const MAX_LAYOUT_PARTITION_ID_LEN: usize = 8;

/// Maximum host network interfaces stored in a layout snapshot.
pub const MAX_LAYOUT_HOST_NETWORK_INTERFACES: usize = 4;

/// Maximum tap interface name length stored in a layout snapshot.
pub const LAYOUT_HOST_NETWORK_IFNAME_LEN: usize = 16;

/// Maximum netdev id length stored in a layout snapshot.
pub const LAYOUT_HOST_NETWORK_NETDEV_LEN: usize = 16;

/// Host network backend is user-mode networking.
pub const LAYOUT_HOST_NETWORK_BACKEND_USER: u8 = 0;
/// Host network backend is tap.
pub const LAYOUT_HOST_NETWORK_BACKEND_TAP: u8 = 1;

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
    /// Byte length of [`Self::partition_id`].
    pub partition_id_len: u8,
    /// Stable partition id bytes (UTF-8, zero padded).
    pub partition_id: [u8; MAX_LAYOUT_PARTITION_ID_LEN],
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
    /// Datapath role marker (`LAYOUT_DEVICE_ROLE_*`).
    pub device_role: u8,
    /// Reserved for alignment.
    pub reserved: u8,
    /// Compact device kind discriminator.
    pub device_kind: u32,
}

/// One outer host network interface stored in a layout snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutHostNetworkSnapshot {
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
    /// Host network backend (`LAYOUT_HOST_NETWORK_BACKEND_*`).
    pub backend: u8,
    /// Byte length of [`Self::tap_ifname`].
    pub tap_ifname_len: u8,
    /// Tap interface name bytes (UTF-8, zero padded).
    pub tap_ifname: [u8; LAYOUT_HOST_NETWORK_IFNAME_LEN],
    /// Byte length of [`Self::netdev_id`].
    pub netdev_id_len: u8,
    /// Reserved for alignment.
    pub reserved: u8,
    /// QEMU netdev identifier bytes (UTF-8, zero padded).
    pub netdev_id: [u8; LAYOUT_HOST_NETWORK_NETDEV_LEN],
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
    /// Whether host networking is enabled.
    pub host_network_enabled: u8,
    /// Host network backend (`LAYOUT_HOST_NETWORK_BACKEND_*`).
    pub host_network_backend: u8,
    /// Reserved for alignment.
    pub host_network_reserved: [u8; 2],
    /// Number of valid entries in [`Self::host_network_interfaces`].
    pub host_network_interface_count: u32,
    /// Independent outer host network interfaces sorted by BDF.
    pub host_network_interfaces: [LayoutHostNetworkSnapshot; MAX_LAYOUT_HOST_NETWORK_INTERFACES],
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn layout_snapshot_layout_is_stable() {
        assert_eq!(size_of::<PlannedRegionSnapshot>(), 16);
        assert_eq!(size_of::<LayoutGuestRegionSnapshot>(), 40);
        assert_eq!(size_of::<LayoutIpcRegionSnapshot>(), 32);
        assert_eq!(size_of::<LayoutPciSnapshot>(), 16);
        assert_eq!(size_of::<LayoutHostNetworkSnapshot>(), 48);
        assert_eq!(align_of::<LayoutSnapshot>(), 8);
        assert!(size_of::<LayoutSnapshot>() > size_of::<LayoutGuestRegionSnapshot>());
    }
}
