//! VMX init plan derived from static platform layout.

use hv_platform_model::PlannedHypervisorReserve;
use hv_types::{ByteSize, HostPhysAddr};

use crate::constants::{VMXON_REGION_ALIGNMENT_BYTES, VMXON_REGION_MIN_BYTES};
use crate::error::{VmxError, VmxErrorKind};

/// Planned VMXON region placement for backend initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmxInitPlan {
    /// Host physical base of the VMXON region.
    pub vmxon_region_phys: HostPhysAddr,
    /// VMXON region size in bytes.
    pub vmxon_region_bytes: ByteSize,
}

/// Builds a VMX init plan from the hypervisor reserve described by static platform IR.
pub fn plan_vmx_init(reserve: &PlannedHypervisorReserve) -> Result<VmxInitPlan, VmxError> {
    let bytes = reserve.size.bytes();
    if bytes < VMXON_REGION_MIN_BYTES {
        return Err(VmxError::new(
            VmxErrorKind::Planning,
            "hypervisor reserve below VMXON minimum",
        ));
    }
    if reserve.host_phys.raw() % VMXON_REGION_ALIGNMENT_BYTES != 0 {
        return Err(VmxError::new(
            VmxErrorKind::Planning,
            "hypervisor reserve base is not aligned for VMXON",
        ));
    }
    if bytes % VMXON_REGION_ALIGNMENT_BYTES != 0 {
        return Err(VmxError::new(
            VmxErrorKind::Planning,
            "hypervisor reserve size is not page aligned",
        ));
    }
    Ok(VmxInitPlan {
        vmxon_region_phys: reserve.host_phys,
        vmxon_region_bytes: reserve.size,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn plan_vmx_init_accepts_reference_layout() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("plan vmx");
        assert!(plan.vmxon_region_bytes.bytes() >= VMXON_REGION_MIN_BYTES);
        assert_eq!(plan.vmxon_region_phys, layout.hypervisor_reserve.host_phys);
    }
}
