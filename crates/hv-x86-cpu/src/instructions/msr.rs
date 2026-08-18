//! Model-specific register helpers for live VMX bring-up.

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// IA32_VMX_BASIC MSR number.
pub const IA32_VMX_BASIC: u32 = 0x480;

/// Reads IA32_VMX_BASIC when live execution is permitted in ring 0.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn read_vmx_basic_msr() -> Result<u64, CpuSeamError> {
    use super::environment::{current_privilege_level, live_execution_runtime_enabled};
    if !live_execution_runtime_enabled() || current_privilege_level() != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "IA32_VMX_BASIC read requires ring 0 live execution",
        ));
    }
    let (low, high) = rdmsr(IA32_VMX_BASIC)?;
    Ok(u64::from(low) | (u64::from(high) << 32))
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

#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
fn rdmsr(msr: u32) -> Result<(u32, u32), CpuSeamError> {
    let mut low: u32;
    let mut high: u32;
    // SAFETY: RDMSR is defined for valid architectural MSRs in ring 0.
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            lateout("eax") low,
            lateout("edx") high,
            options(nostack, preserves_flags),
        );
    }
    Ok((low, high))
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
}
