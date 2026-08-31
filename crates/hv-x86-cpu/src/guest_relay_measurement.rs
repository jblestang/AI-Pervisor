//! In-VM guest relay frame measurement via boot-info counter tails (Phase 29).

use hv_guest_abi::{
    guest_boot_info_relay_frames_offset, GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES,
};
use hv_types::VmId;

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::seams::{CpuInstructionDisposition, DatapathGuestExecutionCpuSeamOutcome};

/// VM id for the reference `out` partition whose counter reflects delivered frames.
pub const GUEST_RELAY_MEASUREMENT_VM_ID: VmId = VmId::new(2);

/// Host-mapped guest boot info site used for relay-frame measurement reads.
///
/// `host_boot_info_phys` must equal guest RDI under the Gate D identity-mapping contract:
/// guest physical addresses for installed ELF/boot-info regions map 1:1 to host physical memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBootInfoMeasurementSite {
    /// Owning VM id for the boot info blob.
    pub vm_id: VmId,
    /// Host physical address of the boot info blob (identity-mapped in firmware).
    pub host_boot_info_phys: u64,
    /// Total boot info blob size in bytes (includes relay counter tail on ABI v2+).
    pub boot_info_size: u32,
}

/// Reads the relay-frame counter from a boot info blob buffer (host-side copy).
pub fn read_relay_frames_completed_from_boot_info_blob(blob: &[u8]) -> Result<u64, CpuSeamError> {
    if blob.len() < core::mem::size_of::<hv_guest_abi::GuestBootInfoHeader>() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info blob too small for header",
        ));
    }
    let size = u32::from_le_bytes([
        blob[12], blob[13], blob[14], blob[15],
    ]);
    read_relay_frames_completed_from_boot_info_host(blob.as_ptr(), size)
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
    if boot_info_size == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info size must not be zero",
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

/// Measures in-VM relay frames from the reference `out` partition boot-info counter.
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
    let out_site = sites
        .iter()
        .find(|site| site.vm_id == GUEST_RELAY_MEASUREMENT_VM_ID)
        .ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "relay measurement requires out-partition boot info site",
            )
        })?;
    if out_site.host_boot_info_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires identity-mapped out-partition boot info address",
        ));
    }
    let frames = read_relay_frames_completed_from_boot_info_host(
        out_site.host_boot_info_phys as *const u8,
        out_site.boot_info_size,
    )?;
    Ok(frames.min(expected_frames))
}

#[cfg(test)]
#[cfg(feature = "datapath-guest-execution")]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn read_relay_frames_completed_reads_tail_counter() {
        let mut blob = [0u8; 64];
        blob[8..12].copy_from_slice(&2u32.to_le_bytes());
        blob[12..16].copy_from_slice(&64u32.to_le_bytes());
        blob[56..64].copy_from_slice(&42u64.to_le_bytes());
        let frames = read_relay_frames_completed_from_boot_info_blob(&blob).expect("read");
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
            vm_id: GUEST_RELAY_MEASUREMENT_VM_ID,
            host_boot_info_phys: 0,
            boot_info_size: 64,
        };
        let frames = measure_in_vm_relay_frames_from_boot_infos(&execution, &[site], 64)
            .expect("measure");
        assert_eq!(frames, 0);
    }

    #[test]
    fn measure_in_vm_relay_frames_requires_out_partition_site() {
        let execution = DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::Executed,
            vmexit_stub_validated: true,
            partitions_validated: 3,
            vmlaunch_attempts: 3,
        };
        let site = GuestBootInfoMeasurementSite {
            vm_id: VmId::new(0),
            host_boot_info_phys: 0x1000,
            boot_info_size: 64,
        };
        assert!(measure_in_vm_relay_frames_from_boot_infos(&execution, &[site], 64).is_err());
    }
}
