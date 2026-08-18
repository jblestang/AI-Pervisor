//! Live VMX instruction execution.

#![allow(clippy::needless_return)]

use hv_ept::EPT_PAGE_OFFSET_MASK;

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Attempts to execute VMXON against the given host-physical VMXON region address.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn execute_vmxon(host_phys: u64) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    validate_vmxon_operand(host_phys)?;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            crate::constants::HV_X86_LIVE_VMXON_UNAVAILABLE,
        ));
    }
    #[cfg(any(test, coverage))]
    {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMXON skipped in test harness",
        ));
    }
    #[cfg(all(not(test), not(coverage)))]
    {
        super::live_asm::enable_vmx_in_cr4()?;
        super::live_asm::vmxon(host_phys)
    }
}

/// Without live execution support, VMXON is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn execute_vmxon(_host_phys: u64) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live VMXON unavailable in this build",
    ))
}

fn validate_vmxon_operand(host_phys: u64) -> Result<(), CpuSeamError> {
    if host_phys == 0 || host_phys & EPT_PAGE_OFFSET_MASK != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMXON region address must be page aligned and non-zero",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_vmxon_operand_rejects_unaligned_address() {
        assert!(validate_vmxon_operand(0x1001).is_err());
        assert!(validate_vmxon_operand(0).is_err());
    }

    #[test]
    fn execute_vmxon_unavailable_without_live_environment() {
        assert!(execute_vmxon(0x1000).is_err());
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn execute_vmxon_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        test_force_live_environment_ready(true);
        let result = execute_vmxon(0x1000);
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }
}
