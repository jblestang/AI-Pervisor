//! In-VM guest relay frame measurement via boot-info counter tails (Phase 29).

use hv_guest_abi::{
    guest_boot_info_relay_frames_offset, GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES,
};
use hv_types::VmId;

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::seams::{CpuInstructionDisposition, DatapathGuestExecutionCpuSeamOutcome};

/// Host-mapped guest boot info site used for relay-frame measurement reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBootInfoMeasurementSite {
    /// Owning VM id for the boot info blob.
    pub vm_id: VmId,
    /// Host physical address of the boot info blob (identity-mapped in firmware).
    pub host_boot_info_phys: u64,
    /// Total boot info blob size in bytes (includes relay counter tail).
    pub boot_info_size: u32,
}

/// Reads the relay-frame counter from a host-mapped boot info blob.
pub fn read_relay_frames_completed_from_boot_info_host(
    host_boot_info: *const u8,
    boot_info_size: u32,
) -> Result<u64, CpuSeamError> {
    if host_boot_info.is_null() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info pointer must not be null",
        ));
    }
    let offset = guest_boot_info_relay_frames_offset(boot_info_size).ok_or_else(|| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info blob too small for relay measurement tail",
        )
    })?;
    if offset + GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES > boot_info_size as usize {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info relay counter out of bounds",
        ));
    }
    // SAFETY: caller guarantees `host_boot_info` points at `boot_info_size` readable bytes.
    let counter = unsafe {
        core::ptr::read_unaligned(host_boot_info.add(offset) as *const u64)
    };
    Ok(counter)
}

/// Measures in-VM relay frames from guest boot-info counters after guest execution.
pub fn measure_in_vm_relay_frames_from_boot_infos(
    execution_seam: &DatapathGuestExecutionCpuSeamOutcome,
    sites: &[GuestBootInfoMeasurementSite],
    expected_frames: u64,
) -> Result<u64, CpuSeamError> {
    if expected_frames == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires a non-zero expected frame count",
        ));
    }
    if execution_seam.disposition != CpuInstructionDisposition::Executed {
        return Ok(0);
    }
    if sites.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires at least one boot info site",
        ));
    }
    let mut min_frames = u64::MAX;
    for site in sites {
        let host_ptr = site.host_boot_info_phys as *const u8;
        let frames = read_relay_frames_completed_from_boot_info_host(host_ptr, site.boot_info_size)?;
        min_frames = min_frames.min(frames);
        let _ = site.vm_id;
    }
    if min_frames == u64::MAX {
        return Ok(0);
    }
    Ok(min_frames.min(expected_frames))
}

#[cfg(test)]
#[cfg(feature = "datapath-guest-execution")]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn read_relay_frames_completed_reads_tail_counter() {
        let mut blob = [0u8; 64];
        blob[56..64].copy_from_slice(&42u64.to_le_bytes());
        let frames =
            read_relay_frames_completed_from_boot_info_host(blob.as_ptr(), blob.len() as u32)
                .expect("read");
        assert_eq!(frames, 42);
    }

    #[test]
    fn measure_in_vm_relay_frames_returns_zero_when_execution_not_executed() {
        let execution = DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SeamValidated,
            vmexit_stub_validated: true,
            partitions_validated: 3,
            vmlaunch_attempts: 3,
        };
        let site = GuestBootInfoMeasurementSite {
            vm_id: VmId::new(0),
            host_boot_info_phys: 0,
            boot_info_size: 64,
        };
        let frames = measure_in_vm_relay_frames_from_boot_infos(&execution, &[site], 64)
            .expect("measure");
        assert_eq!(frames, 0);
    }
}
