//! VMXON region programming for hardware backend bring-up.

use alloc::vec;
use alloc::vec::Vec;

use crate::backend::VmxBackend;
use crate::constants::VMXON_REGION_MIN_BYTES;
use crate::error::{VmxError, VmxErrorKind};
use crate::plan::VmxInitPlan;

/// Reference VMX revision identifier used for MODEL hardware programming tests.
pub const REFERENCE_VMXON_REVISION: u32 = 0x0000_0001;

/// Encoded VMXON region bytes ready for host physical installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmxonProgrammedRegion {
    /// Host physical base where the region must be installed.
    pub host_phys: u64,
    /// VMXON region contents (page-aligned minimum size).
    pub bytes: Vec<u8>,
}

/// Programs a VMXON region from an init plan and revision identifier.
pub fn program_vmxon_region(
    plan: &VmxInitPlan,
    revision: u32,
) -> Result<VmxonProgrammedRegion, VmxError> {
    let size = plan.vmxon_region_bytes.bytes();
    if size < VMXON_REGION_MIN_BYTES {
        return Err(VmxError::new(
            VmxErrorKind::Planning,
            "VMXON region smaller than minimum page",
        ));
    }
    let mut bytes = vec![0u8; size as usize];
    let revision_bytes = revision.to_le_bytes();
    if let Some(prefix) = bytes.get_mut(0..4) {
        prefix.copy_from_slice(&revision_bytes);
    } else {
        return Err(VmxError::new(
            VmxErrorKind::Planning,
            "VMXON region too small for revision prefix",
        ));
    }
    Ok(VmxonProgrammedRegion {
        host_phys: plan.vmxon_region_phys.raw(),
        bytes,
    })
}

/// Backend that encodes VMXON region contents without executing VMX instructions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgrammingVmxBackend {
    /// Number of successful programming calls.
    pub program_calls: u32,
    /// Last programmed VMXON region.
    pub last_region: Option<VmxonProgrammedRegion>,
}

impl VmxBackend for ProgrammingVmxBackend {
    fn enable_vmx(&mut self, plan: &VmxInitPlan) -> Result<(), VmxError> {
        let region = program_vmxon_region(plan, REFERENCE_VMXON_REVISION)?;
        self.program_calls = self.program_calls.saturating_add(1);
        self.last_region = Some(region);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use crate::plan::plan_vmx_init;

    #[test]
    fn program_vmxon_region_writes_revision_prefix() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx plan");
        let region = program_vmxon_region(&plan, REFERENCE_VMXON_REVISION).expect("program");
        assert_eq!(region.host_phys, plan.vmxon_region_phys.raw());
        assert!(region.bytes.len() >= VMXON_REGION_MIN_BYTES as usize);
        let revision = region.bytes.get(0..4).expect("revision prefix");
        assert_eq!(
            u32::from_le_bytes(revision.try_into().expect("revision")),
            REFERENCE_VMXON_REVISION
        );
    }

    #[test]
    fn programming_vmx_backend_records_programmed_region() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx plan");
        let mut backend = ProgrammingVmxBackend::default();
        backend.enable_vmx(&plan).expect("enable");
        assert_eq!(backend.program_calls, 1);
        assert!(backend.last_region.is_some());
    }
}
