//! VT-d init plan derived from static platform layout.

use alloc::vec::Vec;

use hv_platform_model::StaticPlatformIR;
use hv_types::PciBdf;

use crate::error::VtdError;

/// Planned PCI device assignment for VT-d programming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtdDeviceAssignment {
    /// PCI BDF owned by a partition.
    pub bdf: PciBdf,
    /// Owning VM identifier.
    pub vm_id: u32,
}

/// Planned VT-d enablement metadata for backend initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtdInitPlan {
    /// PCI devices assigned to guest partitions.
    pub device_assignments: Vec<VtdDeviceAssignment>,
    /// Whether interrupt remapping must be enabled.
    pub interrupt_remapping: bool,
}

/// Builds a VT-d init plan from static platform layout.
pub fn plan_vtd_init(
    layout: &StaticPlatformIR,
    interrupt_remapping: bool,
) -> Result<VtdInitPlan, VtdError> {
    let mut device_assignments = Vec::with_capacity(layout.pci_devices.len());
    for device in &layout.pci_devices {
        device_assignments.push(VtdDeviceAssignment {
            bdf: device.bdf,
            vm_id: device.vm_id.raw(),
        });
    }
    Ok(VtdInitPlan {
        device_assignments,
        interrupt_remapping,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn plan_vtd_init_assigns_reference_pci_devices() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd plan");
        assert_eq!(plan.device_assignments.len(), layout.pci_devices.len());
        assert!(plan.interrupt_remapping);
    }

    #[test]
    fn plan_vtd_init_accepts_empty_pci_topology() {
        let yaml = include_str!("../../../configs/ovmf-smoke.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, false).expect("vtd plan");
        assert!(plan.device_assignments.is_empty());
        assert!(!plan.interrupt_remapping);
    }
}
