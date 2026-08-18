//! VMCS lifecycle helpers for live EPT pointer programming.

#![allow(clippy::needless_return)]

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Prepares a VMCS region for field programming (VMCLEAR + VMPTRLD).
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn execute_vmcs_prepare(vmcs_phys: u64) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    validate_vmcs_operand(vmcs_phys)?;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "live VMCS prepare requires firmware or env live execution in ring 0",
        ));
    }
    #[cfg(any(test, coverage))]
    {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMCS prepare skipped in test harness",
        ));
    }
    #[cfg(all(not(test), not(coverage)))]
    {
        super::live_asm::vmclear(vmcs_phys)?;
        super::live_asm::vmptrld(vmcs_phys)
    }
}

/// Without live execution support, VMCS prepare is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn execute_vmcs_prepare(_vmcs_phys: u64) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live VMCS prepare unavailable in this build",
    ))
}

fn validate_vmcs_operand(vmcs_phys: u64) -> Result<(), CpuSeamError> {
    if vmcs_phys == 0 || vmcs_phys & 0xFFF != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMCS region address must be page aligned and non-zero",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_vmcs_operand_rejects_unaligned_address() {
        assert!(validate_vmcs_operand(0x1001).is_err());
        assert!(validate_vmcs_operand(0).is_err());
    }

    #[test]
    fn execute_vmcs_prepare_unavailable_without_live_environment() {
        assert!(execute_vmcs_prepare(0x3000).is_err());
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn execute_vmcs_prepare_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        test_force_live_environment_ready(true);
        let result = execute_vmcs_prepare(0x3000);
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }
}
