//! Model-specific register helpers for live VMX bring-up.

#![allow(clippy::needless_return)]

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// IA32_VMX_BASIC MSR number.
pub const IA32_VMX_BASIC: u32 = 0x480;

/// Reads IA32_VMX_BASIC when live execution is permitted in ring 0.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn read_vmx_basic_msr() -> Result<u64, CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "IA32_VMX_BASIC read requires ring 0 live execution",
        ));
    }
    #[cfg(any(test, coverage))]
    {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "IA32_VMX_BASIC read skipped in test harness",
        ));
    }
    #[cfg(all(not(test), not(coverage)))]
    {
        let (low, high) = super::live_asm::rdmsr(IA32_VMX_BASIC)?;
        Ok(u64::from(low) | (u64::from(high) << 32))
    }
}

/// Non-x86 or feature-disabled builds cannot read VMX MSRs.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn read_vmx_basic_msr() -> Result<u64, CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "IA32_VMX_BASIC read unavailable in this build",
    ))
}

/// Extracts the VMX revision identifier from IA32_VMX_BASIC bits 30:0.
pub fn vmx_revision_from_basic_msr(basic: u64) -> u32 {
    (basic & 0x7FFF_FFFF) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmx_revision_from_basic_msr_masks_low_bits() {
        let revision = vmx_revision_from_basic_msr(0x0000_0001_8000_0000);
        assert_eq!(revision, 0);
    }

    #[test]
    fn read_vmx_basic_msr_unavailable_without_live_environment() {
        assert!(read_vmx_basic_msr().is_err());
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn read_vmx_basic_msr_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        test_force_live_environment_ready(true);
        let result = read_vmx_basic_msr();
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }
}
