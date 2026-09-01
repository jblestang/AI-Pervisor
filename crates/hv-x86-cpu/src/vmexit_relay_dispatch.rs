//! Combined VM-exit relay dispatch for measurement page and MMIO datapath.

use hv_types::VmId;

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::vmexit_mmio_relay::{handle_e1000_mmio_vmexit, VmexitE1000MmioConfig};
use crate::vmexit_relay_counter::{
    handle_relay_frame_vmexit, read_relay_measurement_page_frames, VmexitRelayCounterConfig,
    VM_EXIT_REASON_EPT_VIOLATION,
};

/// Per-partition VM-exit relay dispatch inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmexitRelayDispatchConfig {
    /// Optional measurement-page frame counter config (out partition).
    pub measurement: Option<VmexitRelayCounterConfig>,
    /// Optional trapped e1000 MMIO relay config.
    pub e1000_mmio: Option<VmexitE1000MmioConfig>,
}

/// Gate D VM-exit relay dispatch plan across all partitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmexitRelayDispatchPlan {
    /// Hypervisor measurement page counter config for the out partition.
    pub measurement: VmexitRelayCounterConfig,
    /// Trapped e1000 MMIO configs keyed by VM id.
    pub e1000_by_vm: alloc::vec::Vec<(VmId, VmexitE1000MmioConfig)>,
}

impl VmexitRelayDispatchPlan {
    /// Builds the per-partition dispatch config for one launch.
    pub fn config_for_vm(&self, vm_id: VmId) -> VmexitRelayDispatchConfig {
        let measurement = if vm_id == crate::guest_relay_measurement::GUEST_RELAY_MEASUREMENT_VM_ID
        {
            Some(self.measurement)
        } else {
            None
        };
        let e1000_mmio = self
            .e1000_by_vm
            .iter()
            .find(|(id, _)| *id == vm_id)
            .map(|(_, config)| *config);
        VmexitRelayDispatchConfig {
            measurement,
            e1000_mmio,
        }
    }
}

impl VmexitRelayDispatchConfig {
    /// Returns whether the host-side VM-exit dispatch loop is required.
    pub fn requires_host_dispatch(self) -> bool {
        self.measurement.is_some() || self.e1000_mmio.is_some()
    }

    /// Returns whether unhandled EPT violations must fail closed.
    pub fn strict_ept_violations(self) -> bool {
        self.requires_host_dispatch()
    }
}

/// Outcome counters collected by the VM-exit relay dispatch loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VmexitRelayDispatchOutcome {
    /// Relay frames counted via measurement-page EPT traps.
    pub relay_frames: u64,
    /// e1000 MMIO writes emulated on VM-exits.
    pub mmio_relay_events: u64,
}

/// Handles one VM-exit for relay measurement and/or MMIO datapath relay.
pub fn handle_relay_dispatch_vmexit(
    exit_reason: u32,
    guest_phys: u64,
    exit_qualification: u64,
    config: &VmexitRelayDispatchConfig,
) -> Result<VmexitRelayDispatchOutcome, CpuSeamError> {
    let mut outcome = VmexitRelayDispatchOutcome::default();
    if exit_reason != VM_EXIT_REASON_EPT_VIOLATION {
        return Ok(outcome);
    }
    if let Some(mmio) = config.e1000_mmio {
        if handle_e1000_mmio_vmexit(exit_reason, guest_phys, exit_qualification, &mmio)? {
            outcome.mmio_relay_events = 1;
            return Ok(outcome);
        }
    }
    if let Some(measurement) = config.measurement {
        if handle_relay_frame_vmexit(exit_reason, guest_phys, exit_qualification, &measurement)? {
            outcome.relay_frames = 1;
            return Ok(outcome);
        }
    }
    if config.strict_ept_violations() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "unexpected EPT violation during VM-exit relay dispatch",
        ));
    }
    Ok(outcome)
}

/// Validates MMIO relay event count against the reference sustained benchmark.
pub fn validate_vmexit_mmio_relay_events(
    mmio_relay_events: u64,
    expected_in_tx_events: u64,
) -> Result<(), CpuSeamError> {
    if expected_in_tx_events == 0 {
        return Ok(());
    }
    if mmio_relay_events < expected_in_tx_events {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VM-exit MMIO relay event count below expected in-partition TX doorbells",
        ));
    }
    Ok(())
}

/// Validates dispatch loop counters against the hypervisor measurement page.
pub fn finalize_measurement_relay_frames(
    config: &VmexitRelayCounterConfig,
    loop_frames: u64,
) -> Result<u64, CpuSeamError> {
    let page_frames = read_relay_measurement_page_frames(config.measurement_page_host_phys)?;
    if page_frames != loop_frames {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement page frame count mismatch with VM-exit dispatch loop",
        ));
    }
    Ok(page_frames)
}

#[cfg(test)]
#[cfg(feature = "datapath-guest-relay-measurement")]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_datapath::RELAY_MEASUREMENT_PAGE_GUEST_PHYS;

    #[test]
    fn dispatch_config_requires_host_dispatch_when_mmio_present() {
        let config = VmexitRelayDispatchConfig {
            measurement: None,
            e1000_mmio: Some(VmexitE1000MmioConfig {
                mmio_guest_phys: 0xFEB0_0000,
                state_host_phys: 0x8000,
            }),
        };
        assert!(config.requires_host_dispatch());
    }

    #[test]
    fn handle_relay_dispatch_vmexit_rejects_unhandled_ept_when_strict() {
        let config = VmexitRelayDispatchConfig {
            measurement: Some(VmexitRelayCounterConfig {
                measurement_page_host_phys: 0x7000,
                measurement_page_gpa: RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
            }),
            e1000_mmio: None,
        };
        assert!(handle_relay_dispatch_vmexit(
            VM_EXIT_REASON_EPT_VIOLATION,
            0x1000,
            1 << 1,
            &config,
        )
        .is_err());
    }

    #[test]
    fn handle_relay_dispatch_vmexit_rejects_unhandled_ept_for_mmio_only_config() {
        let config = VmexitRelayDispatchConfig {
            measurement: None,
            e1000_mmio: Some(VmexitE1000MmioConfig {
                mmio_guest_phys: 0xFEB0_0000,
                state_host_phys: 0x8000,
            }),
        };
        assert!(handle_relay_dispatch_vmexit(
            VM_EXIT_REASON_EPT_VIOLATION,
            0x1000,
            1 << 1,
            &config,
        )
        .is_err());
    }

    #[test]
    fn validate_vmexit_mmio_relay_events_requires_in_tx_doorbells() {
        assert!(validate_vmexit_mmio_relay_events(63, 64).is_err());
        assert!(validate_vmexit_mmio_relay_events(64, 64).is_ok());
    }
}
