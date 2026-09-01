//! Validates independent outer host e1000 bindings (IN and OUT not linked).
//!
//! Outer QEMU attaches `net_in` and `net_out` to separate host tap interfaces.
//! Nested guests own in→mid→out relay over IPC; the hypervisor does not forward packets.

use hv_platform_model::{PlannedPciDevice, StaticPlatformIR};
use hv_types::{PciBdf, VmId};

use crate::error::{DatapathError, DatapathErrorKind};

/// Planned independent host NIC bindings from platform layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E1000HostAttachPlan {
    /// PCI BDF for the IN partition outer e1000.
    pub host_in_bdf: PciBdf,
    /// PCI BDF for the OUT partition outer e1000.
    pub host_out_bdf: PciBdf,
}

/// Builds host attach bindings from static platform PCI intent.
pub fn plan_e1000_host_attach(layout: &StaticPlatformIR) -> Result<E1000HostAttachPlan, DatapathError> {
    let host_in_bdf = bdf_for_vm(&layout.pci_devices, VmId::new(0))?;
    let host_out_bdf = bdf_for_vm(&layout.pci_devices, VmId::new(2))?;
    if host_in_bdf == host_out_bdf {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "IN and OUT host NICs must use independent BDF bindings",
        ));
    }
    Ok(E1000HostAttachPlan {
        host_in_bdf,
        host_out_bdf,
    })
}

fn bdf_for_vm(devices: &[PlannedPciDevice], vm_id: VmId) -> Result<PciBdf, DatapathError> {
    devices
        .iter()
        .find(|device| device.vm_id == vm_id && device.kind == "nic_e1000")
        .map(|device| device.bdf)
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "missing host NIC device in platform layout",
            )
        })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn plan_finds_independent_host_in_and_host_out_nics() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_e1000_host_attach(&layout).expect("plan");
        assert_ne!(plan.host_in_bdf, plan.host_out_bdf);
    }
}
