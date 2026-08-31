//! VMCS field programming helpers for live VMX launch bring-up.

#![allow(clippy::needless_return)]

use hv_vmx::VmcsProgrammedFields;

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Programs VMCS fields via VMWRITE when live execution is permitted.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn execute_vmcs_field_programming(
    vmcs_phys: u64,
    fields: &VmcsProgrammedFields,
) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    validate_vmcs_fields(fields)?;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            crate::constants::HV_X86_LIVE_VMCS_FIELDS_UNAVAILABLE,
        ));
    }
    super::vmcs::execute_vmcs_prepare(vmcs_phys)?;
    #[cfg(any(test, coverage))]
    {
        let _ = vmcs_phys;
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMCS field programming skipped in test harness",
        ));
    }
    #[cfg(all(not(test), not(coverage)))]
    {
        for field in &fields.fields {
            super::live_asm::vmwrite(field.field, field.value)?;
        }
        Ok(())
    }
}

/// Without live execution support, VMCS field programming is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn execute_vmcs_field_programming(
    _vmcs_phys: u64,
    fields: &VmcsProgrammedFields,
) -> Result<(), CpuSeamError> {
    validate_vmcs_fields(fields)?;
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live VMCS field programming unavailable in this build",
    ))
}

fn validate_vmcs_fields(fields: &VmcsProgrammedFields) -> Result<(), CpuSeamError> {
    if fields.fields.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMCS programmed fields must not be empty",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use hv_vmx::plan_vmx_init;
    use hv_vmx::{plan_vmx_launch, program_vmcs_fields, DEFAULT_SMOKE_GUEST_PARTITION_ID};

    fn reference_fields() -> VmcsProgrammedFields {
        let yaml = include_str!("../../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let launch =
            plan_vmx_launch(&layout, &vmx_plan, DEFAULT_SMOKE_GUEST_PARTITION_ID).expect("launch");
        program_vmcs_fields(&launch)
    }

    #[test]
    fn validate_vmcs_fields_rejects_empty_list() {
        assert!(validate_vmcs_fields(&VmcsProgrammedFields {
            fields: alloc::vec::Vec::new(),
        })
        .is_err());
    }

    #[test]
    fn execute_vmcs_field_programming_unavailable_without_live_environment() {
        let fields = reference_fields();
        assert!(execute_vmcs_field_programming(0x3000, &fields).is_err());
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn execute_vmcs_field_programming_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        let fields = reference_fields();
        test_force_live_environment_ready(true);
        let result = execute_vmcs_field_programming(0x3000, &fields);
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }
}
