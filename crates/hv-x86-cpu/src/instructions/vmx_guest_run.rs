//! VMLAUNCH plus host-side VM-exit dispatch until guest `HLT`.

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Runs the current VMCS guest until it executes `HLT` and the exit stub returns.
#[cfg(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
))]
#[allow(dead_code)]
pub fn run_vmx_guest_until_halt(
    vmcs_phys: u64,
    use_host_dispatch: bool,
) -> Result<(), CpuSeamError> {
    run_vmx_guest_until_halt_with_relay_counter(vmcs_phys, use_host_dispatch, None).map(|_| ())
}

/// Runs the guest until `HLT`, optionally counting relay frames on VM-exits.
#[cfg(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
))]
pub fn run_vmx_guest_until_halt_with_relay_counter(
    vmcs_phys: u64,
    use_host_dispatch: bool,
    relay_config: Option<crate::vmexit_relay_counter::VmexitRelayCounterConfig>,
) -> Result<u64, CpuSeamError> {
    use super::environment::live_execution_environment_ready;

    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "VMX guest run loop unavailable in this environment",
        ));
    }
    super::live_asm::vmptrld(vmcs_phys)?;
    if use_host_dispatch {
        run_vmx_guest_vmexit_dispatch_loop(relay_config)
    } else {
        super::live_asm::vmlaunch_wait_for_hlt_exit()?;
        Ok(0)
    }
}

#[cfg(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    feature = "datapath-guest-relay-measurement",
    not(test),
    not(coverage)
))]
fn run_vmx_guest_vmexit_dispatch_loop(
    relay_config: Option<crate::vmexit_relay_counter::VmexitRelayCounterConfig>,
) -> Result<u64, CpuSeamError> {
    use super::live_asm::{vmlaunch_to_host, vmread, vmresume_to_host, vmwrite};
    use crate::vmexit_relay_counter::{
        handle_relay_frame_vmexit, VMCS_EXIT_QUALIFICATION, VMCS_GUEST_PHYSICAL_ADDRESS,
        VMCS_VM_EXIT_REASON, VM_EXIT_REASON_HLT,
    };

    let mut relay_frames = 0u64;
    vmlaunch_to_host()?;
    loop {
        let exit_reason = vmread(VMCS_VM_EXIT_REASON)? & 0xFFFF;
        if exit_reason == u64::from(VM_EXIT_REASON_HLT) {
            break;
        }
        if let Some(config) = relay_config {
            let guest_phys = vmread(VMCS_GUEST_PHYSICAL_ADDRESS)?;
            let exit_qualification = vmread(VMCS_EXIT_QUALIFICATION)?;
            if handle_relay_frame_vmexit(
                exit_reason as u32,
                guest_phys,
                exit_qualification,
                &config,
            )? {
                relay_frames = relay_frames.saturating_add(1);
                advance_guest_rip(&vmread, &vmwrite)?;
            }
        }
        vmresume_to_host()?;
    }
    Ok(relay_frames)
}

#[cfg(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    feature = "datapath-guest-relay-measurement",
    not(test),
    not(coverage)
))]
fn advance_guest_rip(
    vmread: &impl Fn(u32) -> Result<u64, CpuSeamError>,
    vmwrite: &impl Fn(u32, u64) -> Result<(), CpuSeamError>,
) -> Result<(), CpuSeamError> {
    use crate::vmexit_relay_counter::{VMCS_GUEST_RIP, VMCS_VM_EXIT_INSTRUCTION_LEN};

    let guest_rip = vmread(VMCS_GUEST_RIP)?;
    let instruction_len = vmread(VMCS_VM_EXIT_INSTRUCTION_LEN)? & 0xF;
    vmwrite(VMCS_GUEST_RIP, guest_rip.saturating_add(instruction_len))
}

/// Without live execution support, the guest run loop is unavailable.
#[cfg(not(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
)))]
pub fn run_vmx_guest_until_halt(
    _vmcs_phys: u64,
    _use_host_dispatch: bool,
) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "VMX guest run loop unavailable in this build",
    ))
}

#[cfg(not(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
)))]
#[cfg(feature = "datapath-guest-relay-measurement")]
pub fn run_vmx_guest_until_halt_with_relay_counter(
    _vmcs_phys: u64,
    _use_host_dispatch: bool,
    _relay_config: Option<crate::vmexit_relay_counter::VmexitRelayCounterConfig>,
) -> Result<u64, CpuSeamError> {
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
        let result = run_vmx_guest_until_halt(0x3000, false);
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }
}
