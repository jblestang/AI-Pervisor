//! VMLAUNCH plus VM-exit / VMRESUME loop until guest `HLT`.

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Runs the current VMCS guest until it executes `HLT` and the exit stub returns.
#[cfg(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
))]
pub fn run_vmx_guest_until_halt(vmcs_phys: u64, host_exit_phys: u64) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    use super::vmexit_stub::install_vmexit_stub;

    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "VMX guest run loop unavailable in this environment",
        ));
    }
    install_vmexit_stub(host_exit_phys)?;
    super::live_asm::vmptrld(vmcs_phys)?;
    super::live_asm::vmlaunch_wait_for_hlt_exit()
}

/// Without live execution support, the guest run loop is unavailable.
#[cfg(not(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
)))]
pub fn run_vmx_guest_until_halt(_vmcs_phys: u64, _host_exit_phys: u64) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "VMX guest run loop unavailable in this build",
    ))
}

#[cfg(all(test, feature = "execute-instructions"))]
mod tests {
    use super::*;
    use crate::instructions::environment::test_force_live_environment_ready;

    #[test]
    fn run_vmx_guest_until_halt_unavailable_in_test_harness() {
        test_force_live_environment_ready(true);
        let result = run_vmx_guest_until_halt(0x3000, 0x4000);
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }
}
