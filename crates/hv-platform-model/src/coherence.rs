//! Cross-checks between host network, PCI devices, and MMIO BAR layout.

use crate::error::{PlatformError, PlatformErrorKind};
use crate::lookup::{DATAPATH_ROLE_IN, DATAPATH_ROLE_OUT};
use crate::platform_ir::StaticPlatformIR;

/// Validates that outer host network interfaces match nested PCI/MMIO layout.
pub fn validate_layout_host_network_coherence(
    layout: &StaticPlatformIR,
) -> Result<(), PlatformError> {
    if !layout.host_network.enabled {
        return Ok(());
    }

    for interface in &layout.host_network.interfaces {
        let pci = layout
            .pci_devices
            .iter()
            .find(|device| device.bdf == interface.bdf)
            .ok_or_else(|| {
                PlatformError::new(
                    PlatformErrorKind::Planning,
                    "host network interface BDF missing from PCI layout",
                )
            })?;
        if pci.vm_id != interface.vm_id {
            return Err(PlatformError::new(
                PlatformErrorKind::Planning,
                "host network interface vm id mismatch with PCI layout",
            ));
        }
        if pci.mmio_guest_phys != interface.mmio_guest_phys {
            return Err(PlatformError::new(
                PlatformErrorKind::Planning,
                "host network interface MMIO mismatch with PCI layout",
            ));
        }
        if pci.kind != "nic_e1000" {
            return Err(PlatformError::new(
                PlatformErrorKind::Planning,
                "host network interface BDF must reference nic_e1000 PCI layout entry",
            ));
        }
        let guest = layout
            .guest_memory
            .iter()
            .find(|guest| guest.vm_id == interface.vm_id)
            .ok_or_else(|| {
                PlatformError::new(
                    PlatformErrorKind::Planning,
                    "host network interface vm id missing from guest layout",
                )
            })?;
        if guest.partition_id != interface.partition_id {
            return Err(PlatformError::new(
                PlatformErrorKind::Planning,
                "host network interface partition id mismatch with guest layout",
            ));
        }
    }

    for role in [DATAPATH_ROLE_IN, DATAPATH_ROLE_OUT] {
        let pci = layout
            .pci_devices
            .iter()
            .find(|device| device.role.as_deref() == Some(role))
            .ok_or_else(|| {
                PlatformError::new(
                    PlatformErrorKind::Planning,
                    "datapath PCI role missing from layout",
                )
            })?;
        if !layout
            .host_network
            .interfaces
            .iter()
            .any(|interface| interface.bdf == pci.bdf)
        {
            return Err(PlatformError::new(
                PlatformErrorKind::Planning,
                "datapath PCI role missing from host network layout",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;

    use crate::planner::plan_static_platform_ir;

    #[test]
    fn reference_config_passes_host_network_coherence() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        validate_layout_host_network_coherence(&layout).expect("coherent");
    }

    #[test]
    fn rejects_host_network_mmio_mismatch_with_pci() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let mut layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        assert!(!layout.host_network.interfaces.is_empty());
        if let Some(interface) = layout.host_network.interfaces.first_mut() {
            interface.mmio_guest_phys ^= 1;
        }
        assert!(validate_layout_host_network_coherence(&layout).is_err());
    }
}
