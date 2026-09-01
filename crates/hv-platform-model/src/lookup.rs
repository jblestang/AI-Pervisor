//! Platform layout lookups derived from static IR (no hardcoded VM ids).

use hv_types::{PciBdf, VmId};

use crate::error::{PlatformError, PlatformErrorKind};
use crate::platform_ir::{PlannedPciDevice, StaticPlatformIR};

/// Datapath ingress NIC role string from platform configuration schema.
pub const DATAPATH_ROLE_IN: &str = "datapath_in";
/// Datapath egress NIC role string from platform configuration schema.
pub const DATAPATH_ROLE_OUT: &str = "datapath_out";

/// Returns the VM id assigned to a partition by stable partition id.
pub fn vm_id_for_partition_id(
    layout: &StaticPlatformIR,
    partition_id: &str,
) -> Result<VmId, PlatformError> {
    layout
        .guest_memory
        .iter()
        .find(|guest| guest.partition_id == partition_id)
        .map(|guest| guest.vm_id)
        .ok_or_else(|| {
            PlatformError::new(
                PlatformErrorKind::Planning,
                "partition id not found in platform layout",
            )
        })
}

/// Returns the PCI device assigned the given datapath role.
pub fn pci_device_for_datapath_role<'a>(
    layout: &'a StaticPlatformIR,
    role: &str,
) -> Result<&'a PlannedPciDevice, PlatformError> {
    layout
        .pci_devices
        .iter()
        .find(|device| device.role.as_deref() == Some(role))
        .ok_or_else(|| {
            PlatformError::new(
                PlatformErrorKind::Planning,
                "datapath role not found in platform PCI layout",
            )
        })
}

/// Returns the PCI BDF for a datapath role.
pub fn bdf_for_datapath_role(
    layout: &StaticPlatformIR,
    role: &str,
) -> Result<PciBdf, PlatformError> {
    Ok(pci_device_for_datapath_role(layout, role)?.bdf)
}

/// Returns the VM id that owns the datapath ingress NIC.
pub fn vm_id_for_datapath_in(layout: &StaticPlatformIR) -> Result<VmId, PlatformError> {
    Ok(pci_device_for_datapath_role(layout, DATAPATH_ROLE_IN)?.vm_id)
}

/// Returns the VM id that owns the datapath egress NIC.
pub fn vm_id_for_datapath_out(layout: &StaticPlatformIR) -> Result<VmId, PlatformError> {
    Ok(pci_device_for_datapath_role(layout, DATAPATH_ROLE_OUT)?.vm_id)
}

/// Returns the MMIO guest physical base for a datapath role.
pub fn mmio_guest_phys_for_datapath_role(
    layout: &StaticPlatformIR,
    role: &str,
) -> Result<u64, PlatformError> {
    Ok(pci_device_for_datapath_role(layout, role)?.mmio_guest_phys)
}

/// Returns the MMIO guest physical base for a VM-owned e1000 PCI device.
pub fn mmio_guest_phys_for_vm_id(
    layout: &StaticPlatformIR,
    vm_id: VmId,
) -> Result<u64, PlatformError> {
    layout
        .pci_devices
        .iter()
        .find(|device| device.vm_id == vm_id && device.kind == "nic_e1000")
        .map(|device| device.mmio_guest_phys)
        .ok_or_else(|| {
            PlatformError::new(
                PlatformErrorKind::Planning,
                "vm id has no e1000 device in platform PCI layout",
            )
        })
}

/// Returns the VM id for the relay partition between datapath IN and OUT.
pub fn vm_id_for_datapath_mid(
    layout: &StaticPlatformIR,
    in_vm: VmId,
    out_vm: VmId,
) -> Result<VmId, PlatformError> {
    layout
        .guest_memory
        .iter()
        .find(|guest| guest.vm_id != in_vm && guest.vm_id != out_vm)
        .map(|guest| guest.vm_id)
        .ok_or_else(|| {
            PlatformError::new(
                PlatformErrorKind::Planning,
                "datapath mid partition not found in platform layout",
            )
        })
}
