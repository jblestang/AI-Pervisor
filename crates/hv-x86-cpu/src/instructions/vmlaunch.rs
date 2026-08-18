//! Live VMLAUNCH instruction execution.

#![allow(clippy::needless_return)]

use hv_ept::EPT_PAGE_OFFSET_MASK;

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Attempts to execute VMLAUNCH when live execution is permitted.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn execute_vmlaunch(vmcs_phys: u64) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    validate_vmlaunch_operand(vmcs_phys)?;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            crate::constants::HV_X86_LIVE_VMLAUNCH_UNAVAILABLE,
        ));
    }
    #[cfg(any(test, coverage))]
    {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMLAUNCH skipped in test harness",
        ));
    }
    #[cfg(all(not(test), not(coverage)))]
    {
        super::live_asm::vmlaunch()
    }
}

/// Without live execution support, VMLAUNCH is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn execute_vmlaunch(_vmcs_phys: u64) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live VMLAUNCH unavailable in this build",
    ))
}

fn validate_vmlaunch_operand(vmcs_phys: u64) -> Result<(), CpuSeamError> {
    if vmcs_phys == 0 || vmcs_phys & EPT_PAGE_OFFSET_MASK != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMLAUNCH requires a page-aligned non-zero VMCS address",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_vmlaunch_operand_rejects_unaligned_address() {
        assert!(validate_vmlaunch_operand(0x1001).is_err());
        assert!(validate_vmlaunch_operand(0).is_err());
    }

    #[test]
    fn execute_vmlaunch_unavailable_without_live_environment() {
        assert!(execute_vmlaunch(0x3000).is_err());
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn execute_vmlaunch_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        test_force_live_environment_ready(true);
        let result = execute_vmlaunch(0x3000);
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }
}
