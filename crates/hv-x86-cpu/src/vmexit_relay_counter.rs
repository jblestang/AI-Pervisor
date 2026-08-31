//! VM-exit-driven relay frame counting on read-only measurement page writes (Phase 43).

use hv_datapath::RELAY_MEASUREMENT_PAGE_BYTES;
use hv_guest_abi::GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES;

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Basic VM-exit reason for guest `HLT` (Intel SDM).
pub const VM_EXIT_REASON_HLT: u32 = 12;
/// Basic VM-exit reason for EPT violation (Intel SDM).
pub const VM_EXIT_REASON_EPT_VIOLATION: u32 = 48;

/// VMCS encoding for `VM_EXIT_REASON`.
#[allow(dead_code)]
pub const VMCS_VM_EXIT_REASON: u32 = 0x4402;
/// VMCS encoding for `VM_EXIT_INSTRUCTION_LEN`.
#[allow(dead_code)]
pub const VMCS_VM_EXIT_INSTRUCTION_LEN: u32 = 0x440C;
/// VMCS encoding for `GUEST_RIP`.
#[allow(dead_code)]
pub const VMCS_GUEST_RIP: u32 = 0x681E;
/// VMCS encoding for `GUEST_PHYSICAL_ADDRESS`.
#[allow(dead_code)]
pub const VMCS_GUEST_PHYSICAL_ADDRESS: u32 = 0x2400;
/// VMCS encoding for `EXIT_QUALIFICATION`.
#[allow(dead_code)]
pub const VMCS_EXIT_QUALIFICATION: u32 = 0x6400;

/// Byte offset of `frames_completed` in the relay measurement page header.
const MEASUREMENT_PAGE_FRAMES_OFFSET: usize = 8;

/// Configuration for hypervisor-side VM-exit relay frame counting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmexitRelayCounterConfig {
    /// Host physical base of the hypervisor-owned measurement page.
    pub measurement_page_host_phys: u64,
    /// Guest physical base mapped read-only for the measurement page.
    pub measurement_page_gpa: u64,
}

/// Returns whether an EPT violation exit qualifies as a relay frame increment.
pub fn is_measurement_page_write_violation(
    guest_phys: u64,
    exit_qualification: u64,
    config: &VmexitRelayCounterConfig,
) -> bool {
    if config.measurement_page_gpa == 0 || config.measurement_page_host_phys == 0 {
        return false;
    }
    let page_end = config
        .measurement_page_gpa
        .saturating_add(RELAY_MEASUREMENT_PAGE_BYTES.saturating_sub(1));
    if guest_phys < config.measurement_page_gpa || guest_phys > page_end {
        return false;
    }
    // EPT violation qualification bit 1: access was a data write.
    exit_qualification & (1 << 1) != 0
}

/// Reads the current relay frame count from the hypervisor measurement page.
pub fn read_relay_measurement_page_frames(host_phys: u64) -> Result<u64, CpuSeamError> {
    if host_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement page host address must be non-zero",
        ));
    }
    let mut bytes = [0u8; 8];
    // SAFETY: caller guarantees the measurement page host range is readable.
    unsafe {
        core::ptr::copy_nonoverlapping(
            host_phys.saturating_add(MEASUREMENT_PAGE_FRAMES_OFFSET as u64) as *const u8,
            bytes.as_mut_ptr(),
            8,
        );
    }
    Ok(u64::from_le_bytes(bytes))
}

/// Increments the relay frame counter on the hypervisor-owned measurement page.
pub fn increment_relay_measurement_page_frames(host_phys: u64) -> Result<u64, CpuSeamError> {
    let current = read_relay_measurement_page_frames(host_phys)?;
    let next = current.saturating_add(1);
    write_relay_measurement_page_frames(host_phys, next)?;
    let read_back = read_relay_measurement_page_frames(host_phys)?;
    if read_back != next {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement page frame increment read-back mismatch",
        ));
    }
    Ok(next)
}

/// Resets the relay frame counter on the hypervisor-owned measurement page.
pub fn reset_relay_measurement_page_frames(host_phys: u64) -> Result<(), CpuSeamError> {
    write_relay_measurement_page_frames(host_phys, 0)
}

fn write_relay_measurement_page_frames(host_phys: u64, frames: u64) -> Result<(), CpuSeamError> {
    if host_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement page host address must be non-zero",
        ));
    }
    let bytes = frames.to_le_bytes();
    // SAFETY: caller guarantees the measurement page header is hypervisor-writable.
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            host_phys.saturating_add(MEASUREMENT_PAGE_FRAMES_OFFSET as u64) as *mut u8,
            8,
        );
    }
    Ok(())
}

/// Handles one VM-exit that may increment the relay frame counter.
pub fn handle_relay_frame_vmexit(
    exit_reason: u32,
    guest_phys: u64,
    exit_qualification: u64,
    config: &VmexitRelayCounterConfig,
) -> Result<bool, CpuSeamError> {
    if exit_reason != VM_EXIT_REASON_EPT_VIOLATION {
        return Ok(false);
    }
    if !is_measurement_page_write_violation(guest_phys, exit_qualification, config) {
        return Ok(false);
    }
    increment_relay_measurement_page_frames(config.measurement_page_host_phys)?;
    Ok(true)
}

/// Validates VM-exit-driven frame count against IPC and expected bounds.
pub fn validate_vmexit_relay_frame_count(
    vmexit_frames: u64,
    ipc_delivered_frames: u64,
    expected_frames: u64,
) -> Result<(), CpuSeamError> {
    if vmexit_frames == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires non-zero VM-exit frame count",
        ));
    }
    let capped = ipc_delivered_frames.min(expected_frames);
    if vmexit_frames > capped {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VM-exit relay frame count exceeds IPC delivered frames",
        ));
    }
    Ok(())
}

const _: () = assert!(GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES >= 16);

#[cfg(test)]
#[cfg(feature = "datapath-guest-relay-measurement")]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_datapath::RELAY_MEASUREMENT_PAGE_GUEST_PHYS;

    fn sample_config(host_phys: u64) -> VmexitRelayCounterConfig {
        VmexitRelayCounterConfig {
            measurement_page_host_phys: host_phys,
            measurement_page_gpa: RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
        }
    }

    #[test]
    fn is_measurement_page_write_violation_requires_write_qualification() {
        let config = sample_config(0x8000);
        assert!(is_measurement_page_write_violation(
            RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
            1 << 1,
            &config,
        ));
        assert!(!is_measurement_page_write_violation(
            RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
            1 << 0,
            &config,
        ));
    }

    #[test]
    fn is_measurement_page_write_violation_rejects_out_of_range_gpa() {
        let config = sample_config(0x8000);
        assert!(!is_measurement_page_write_violation(
            RELAY_MEASUREMENT_PAGE_GUEST_PHYS + RELAY_MEASUREMENT_PAGE_BYTES,
            1 << 1,
            &config,
        ));
    }

    #[test]
    fn increment_relay_measurement_page_frames_updates_host_buffer() {
        let mut page = [0u8; 64];
        let host_phys = page.as_mut_ptr() as u64;
        let config = sample_config(host_phys);
        assert!(handle_relay_frame_vmexit(
            VM_EXIT_REASON_EPT_VIOLATION,
            config.measurement_page_gpa,
            1 << 1,
            &config,
        )
        .expect("handle"));
        assert_eq!(
            read_relay_measurement_page_frames(host_phys).expect("read"),
            1
        );
    }

    #[test]
    fn validate_vmexit_relay_frame_count_rejects_over_ipc() {
        assert!(validate_vmexit_relay_frame_count(50, 40, 64).is_err());
        assert!(validate_vmexit_relay_frame_count(40, 40, 64).is_ok());
    }
}
