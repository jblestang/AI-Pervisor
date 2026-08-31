//! Live EPT pointer programming helpers.

#![allow(clippy::needless_return)]

use hv_ept::{
    EPT_PAGE_OFFSET_MASK, EPT_POINTER_MEMORY_TYPE_SHIFT, EPT_POINTER_MEMORY_TYPE_WB,
    EPT_POINTER_PAGE_WALK_LENGTH, EPT_POINTER_PAGE_WALK_LENGTH_SHIFT,
};

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Attempts to load an EPT pointer into the current VMCS when live execution is permitted.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn execute_ept_pointer_load(ept_pointer: u64, vmcs_phys: u64) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    validate_ept_pointer_operand(ept_pointer)?;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "live EPT pointer load requires live execution opt-in in ring 0",
        ));
    }
    #[cfg(not(any(test, coverage)))]
    {
        super::vmcs::execute_vmcs_prepare(vmcs_phys)?;
    }
    #[cfg(any(test, coverage))]
    {
        let _ = vmcs_phys;
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "EPT pointer load skipped in test harness",
        ));
    }
    #[cfg(all(not(test), not(coverage)))]
    {
        super::live_asm::vmwrite_ept_pointer(ept_pointer)
    }
}

/// Without live execution support, EPT pointer load is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn execute_ept_pointer_load(_ept_pointer: u64, _vmcs_phys: u64) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live EPT pointer load unavailable in this build",
    ))
}

/// Attempts INVEPT single-context invalidation for the given encoded EPT pointer.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn execute_invept_single_context(ept_pointer: u64) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    validate_ept_pointer_operand(ept_pointer)?;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "live INVEPT requires live execution opt-in in ring 0",
        ));
    }
    #[cfg(not(any(test, coverage)))]
    {
        super::live_asm::invept_single_context(ept_pointer)
    }
    #[cfg(any(test, coverage))]
    {
        let _ = ept_pointer;
        Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "INVEPT skipped in test harness",
        ))
    }
}

/// Without live execution support, INVEPT is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn execute_invept_single_context(_ept_pointer: u64) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live INVEPT unavailable in this build",
    ))
}

const EPT_POINTER_LOW_CONTROL_MASK: u64 = (EPT_POINTER_MEMORY_TYPE_WB
    << EPT_POINTER_MEMORY_TYPE_SHIFT)
    | (EPT_POINTER_PAGE_WALK_LENGTH << EPT_POINTER_PAGE_WALK_LENGTH_SHIFT);

fn validate_ept_pointer_operand(ept_pointer: u64) -> Result<(), CpuSeamError> {
    if ept_pointer == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "EPT pointer must be non-zero",
        ));
    }
    let low_bits = ept_pointer & EPT_PAGE_OFFSET_MASK;
    if low_bits & !EPT_POINTER_LOW_CONTROL_MASK != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "EPT pointer low bits contain reserved or misaligned fields",
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
    fn validate_ept_pointer_operand_accepts_encoded_ept_pointer() {
        let encoded = 0x2000u64 | EPT_POINTER_LOW_CONTROL_MASK;
        assert!(validate_ept_pointer_operand(encoded).is_ok());
    }

    #[test]
    fn execute_ept_pointer_load_unavailable_without_live_environment() {
        assert!(execute_ept_pointer_load(0x2000, 0x3000).is_err());
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn execute_ept_pointer_load_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        test_force_live_environment_ready(true);
        let result = execute_ept_pointer_load(0x2000, 0x3000);
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }

    #[test]
    fn execute_invept_single_context_unavailable_without_live_environment() {
        assert!(execute_invept_single_context(0x2000).is_err());
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn execute_invept_single_context_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        test_force_live_environment_ready(true);
        let result = execute_invept_single_context(0x2000);
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }
}
