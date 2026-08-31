//! VT-d context programming for hardware backend bring-up.

use alloc::vec::Vec;

use crate::backend::VtdBackend;
use crate::error::VtdError;
use crate::plan::{VtdDeviceAssignment, VtdInitPlan};

/// Encoded VT-d device assignment ready for context table programming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtdProgrammedAssignment {
    /// PCI segment number.
    pub segment: u16,
    /// PCI bus number.
    pub bus: u8,
    /// PCI device number.
    pub device: u8,
    /// PCI function number.
    pub function: u8,
    /// Owning VM identifier.
    pub vm_id: u32,
    /// Whether interrupt remapping is required for this plan.
    pub interrupt_remapping: bool,
    /// Encoded context-entry flags (MODEL placeholder).
    pub context_flags: u64,
}

/// Encoded VT-d programming output for an init plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtdProgrammedTables {
    /// Whether interrupt remapping must be enabled.
    pub interrupt_remapping: bool,
    /// Device assignments with encoded context metadata.
    pub assignments: Vec<VtdProgrammedAssignment>,
}

/// Encodes VT-d context metadata for one device assignment.
pub fn encode_vtd_context_entry(
    assignment: &VtdDeviceAssignment,
    interrupt_remapping: bool,
) -> VtdProgrammedAssignment {
    let mut context_flags = 1u64;
    if interrupt_remapping {
        context_flags |= 1 << 1;
    }
    context_flags |= (assignment.vm_id as u64) << 8;
    VtdProgrammedAssignment {
        segment: assignment.bdf.segment.raw(),
        bus: assignment.bdf.bus.raw(),
        device: assignment.bdf.device.raw(),
        function: assignment.bdf.function.raw(),
        vm_id: assignment.vm_id,
        interrupt_remapping,
        context_flags,
    }
}

/// Programs VT-d assignment records from an init plan.
pub fn program_vtd_tables(plan: &VtdInitPlan) -> Result<VtdProgrammedTables, VtdError> {
    let mut assignments = Vec::with_capacity(plan.device_assignments.len());
    for device in &plan.device_assignments {
        assignments.push(encode_vtd_context_entry(device, plan.interrupt_remapping));
    }
    Ok(VtdProgrammedTables {
        interrupt_remapping: plan.interrupt_remapping,
        assignments,
    })
}

/// Backend that encodes VT-d context metadata without programming IOMMU registers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgrammingVtdBackend {
    /// Number of successful programming calls.
    pub program_calls: u32,
    /// Last programmed VT-d tables.
    pub last_tables: Option<VtdProgrammedTables>,
}

impl VtdBackend for ProgrammingVtdBackend {
    fn enable_vtd(&mut self, plan: &VtdInitPlan) -> Result<(), VtdError> {
        let tables = program_vtd_tables(plan)?;
        self.program_calls = self.program_calls.saturating_add(1);
        self.last_tables = Some(tables);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::plan::plan_vtd_init;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn program_vtd_tables_preserves_device_assignments() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd");
        let tables = program_vtd_tables(&plan).expect("program");
        assert_eq!(tables.assignments.len(), plan.device_assignments.len());
        assert!(tables.interrupt_remapping);
    }
}
