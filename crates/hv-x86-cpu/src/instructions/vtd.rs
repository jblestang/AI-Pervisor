//! Live VT-d enable intent recording for Gate C bring-up.

use crate::error::{CpuSeamError, CpuSeamErrorKind};

use super::environment::live_execution_environment_ready;

/// Records a live VT-d enable intent when the execution environment is ready.
///
/// Phase 14 does not perform DMAR MMIO; firmware/hardware integration supplies the base later.
pub fn execute_vtd_enable(interrupt_remapping: bool) -> Result<(), CpuSeamError> {
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            crate::constants::HV_X86_LIVE_VTD_UNAVAILABLE,
        ));
    }
    record_vtd_enable_intent(interrupt_remapping);
    Ok(())
}

static mut LAST_VTD_ENABLE_INTENT: VtdEnableIntent = VtdEnableIntent {
    recorded: false,
    interrupt_remapping: false,
};

/// Last recorded VT-d enable intent for host-side verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VtdEnableIntent {
    /// Whether an intent was recorded.
    pub recorded: bool,
    /// Interrupt remapping flag from the init plan.
    pub interrupt_remapping: bool,
}

/// Returns the last recorded VT-d enable intent.
pub fn last_vtd_enable_intent() -> VtdEnableIntent {
    // SAFETY: host-only Gate C tests read the intent after a single-threaded enable call.
    unsafe { LAST_VTD_ENABLE_INTENT }
}

fn record_vtd_enable_intent(interrupt_remapping: bool) {
    // SAFETY: host-only Gate C path records intent from the current thread.
    unsafe {
        LAST_VTD_ENABLE_INTENT = VtdEnableIntent {
            recorded: true,
            interrupt_remapping,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_vtd_enable_unavailable_without_live_environment() {
        assert!(execute_vtd_enable(true).is_err());
    }

    #[test]
    fn execute_vtd_enable_records_intent_when_test_environment_forced() {
    use crate::instructions::environment::test_force_live_environment_ready;

    test_force_live_environment_ready(true);
    execute_vtd_enable(true).expect("record intent");
    let intent = last_vtd_enable_intent();
    assert!(intent.recorded);
    assert!(intent.interrupt_remapping);
    test_force_live_environment_ready(false);
}
}
