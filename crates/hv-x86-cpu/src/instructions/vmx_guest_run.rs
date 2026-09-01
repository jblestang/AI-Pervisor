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
    run_vmx_guest_until_halt_with_relay_dispatch(vmcs_phys, use_host_dispatch, None).map(|_| ())
}

/// Runs the guest until `HLT`, optionally dispatching relay measurement and MMIO VM-exits.
#[cfg(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
))]
pub fn run_vmx_guest_until_halt_with_relay_dispatch(
    vmcs_phys: u64,
    use_host_dispatch: bool,
    dispatch_config: Option<crate::vmexit_relay_dispatch::VmexitRelayDispatchConfig>,
) -> Result<crate::vmexit_relay_dispatch::VmexitRelayDispatchOutcome, CpuSeamError> {
    use crate::vmexit_relay_dispatch::VmexitRelayDispatchOutcome;

    if !super::environment::live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "VMX guest run loop unavailable in this environment",
        ));
    }
    super::live_asm::vmptrld(vmcs_phys)?;
    if use_host_dispatch {
        let config = dispatch_config.ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "host VM-exit dispatch requires relay dispatch config",
            )
        })?;
        if !config.requires_host_dispatch() {
            return Err(CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "host VM-exit dispatch requested without relay handlers",
            ));
        }
        run_vmx_guest_vmexit_dispatch_loop(config)
    } else {
        super::live_asm::vmlaunch_wait_for_hlt_exit()?;
        Ok(VmexitRelayDispatchOutcome::default())
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
    dispatch_config: crate::vmexit_relay_dispatch::VmexitRelayDispatchConfig,
) -> Result<crate::vmexit_relay_dispatch::VmexitRelayDispatchOutcome, CpuSeamError> {
    use super::live_asm::{vmlaunch_to_host, vmread, vmresume_to_host, vmwrite};
    use crate::vmexit_relay_counter::{
        VMCS_EXIT_QUALIFICATION, VMCS_GUEST_PHYSICAL_ADDRESS, VMCS_VM_EXIT_REASON,
        VM_EXIT_REASON_HLT,
    };
    use crate::vmexit_relay_dispatch::{
        finalize_measurement_relay_frames, handle_relay_dispatch_vmexit, VmexitRelayDispatchOutcome,
    };

    let mut outcome = VmexitRelayDispatchOutcome::default();
    vmlaunch_to_host()?;
    loop {
        let exit_reason = vmread(VMCS_VM_EXIT_REASON)? & 0xFFFF;
        if exit_reason == u64::from(VM_EXIT_REASON_HLT) {
            break;
        }
        let guest_phys = vmread(VMCS_GUEST_PHYSICAL_ADDRESS)?;
        let exit_qualification = vmread(VMCS_EXIT_QUALIFICATION)?;
        let step = handle_relay_dispatch_vmexit(
            exit_reason as u32,
            guest_phys,
            exit_qualification,
            &dispatch_config,
        )?;
        if step.relay_frames > 0 || step.mmio_relay_events > 0 {
            outcome.relay_frames = outcome.relay_frames.saturating_add(step.relay_frames);
            outcome.mmio_relay_events = outcome
                .mmio_relay_events
                .saturating_add(step.mmio_relay_events);
            advance_guest_rip(&vmread, &vmwrite)?;
        }
        vmresume_to_host()?;
    }
    if let Some(measurement) = dispatch_config.measurement {
        outcome.relay_frames =
            finalize_measurement_relay_frames(&measurement, outcome.relay_frames)?;
    }
    Ok(outcome)
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
pub fn run_vmx_guest_until_halt_with_relay_dispatch(
    _vmcs_phys: u64,
    _use_host_dispatch: bool,
    _dispatch_config: Option<crate::vmexit_relay_dispatch::VmexitRelayDispatchConfig>,
) -> Result<crate::vmexit_relay_dispatch::VmexitRelayDispatchOutcome, CpuSeamError> {
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
