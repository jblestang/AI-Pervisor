//! Live EPT pointer programming helpers.

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Attempts to load an EPT pointer into the current VMCS when live execution is permitted.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn execute_ept_pointer_load(ept_pointer: u64) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    validate_ept_pointer_operand(ept_pointer)?;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "live EPT pointer load requires HV_X86_LIVE_INSTRUCTIONS=1 in ring 0",
        ));
    }
    vmwrite_ept_pointer(ept_pointer)
}

/// Without live execution support, EPT pointer load is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn execute_ept_pointer_load(_ept_pointer: u64) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live EPT pointer load unavailable in this build",
    ))
}

fn validate_ept_pointer_operand(ept_pointer: u64) -> Result<(), CpuSeamError> {
    if ept_pointer & 0xFFF != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "EPT pointer low 12 bits must be zero",
        ));
    }
    Ok(())
}

/// VMCS EPT pointer encoding field number.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
const VMCS_EPT_POINTER: u32 = 0x0000_201A;

#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
fn vmwrite_ept_pointer(value: u64) -> Result<(), CpuSeamError> {
    let mut cf: u8;
    let mut zf: u8;
    // SAFETY: VMWRITE is defined when executing in VMX root operation with a valid VMCS.
    unsafe {
        core::arch::asm!(
            "vmwrite {field}, {value}",
            "setc {cf}",
            "setz {zf}",
            field = in(reg) u64::from(VMCS_EPT_POINTER),
            value = in(reg) value,
            cf = out(reg_byte) cf,
            zf = out(reg_byte) zf,
            options(nostack),
        );
    }
    if cf != 0 || zf != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMWRITE EPT pointer failed (CF/ZF set)",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ept_pointer_operand_rejects_misaligned_low_bits() {
        assert!(validate_ept_pointer_operand(0x1001).is_err());
    }

    #[test]
    fn execute_ept_pointer_load_unavailable_without_live_environment() {
        assert!(execute_ept_pointer_load(0x2000).is_err());
    }
}
