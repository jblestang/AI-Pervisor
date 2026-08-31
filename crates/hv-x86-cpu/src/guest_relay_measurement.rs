//! In-VM guest relay frame measurement via boot-info extension and EPT reads (Phase 29–31).

use hv_ept::{resolve_guest_phys_to_host, EptProgrammedTables};
use hv_guest_abi::{
    guest_relay_measurement_elapsed_tsc, parse_guest_boot_info_relay_measurement,
    GuestBootInfoRelayMeasurement, GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES,
};
use hv_types::VmId;

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::seams::{CpuInstructionDisposition, DatapathGuestExecutionCpuSeamOutcome};

/// VM id for the reference `out` partition whose counter reflects delivered frames.
pub const GUEST_RELAY_MEASUREMENT_VM_ID: VmId = VmId::new(2);

/// IPC queue header tail offset (delivered frame count) in bytes.
const IPC_QUEUE_TAIL_OFFSET: usize = 4;

/// Context for EPT-aware in-VM relay measurement reads.
#[derive(Debug, Clone)]
pub struct GuestRelayMeasurementContext {
    /// Programmed EPT tables used to resolve guest physical addresses.
    pub ept_tables: EptProgrammedTables,
    /// Out-partition boot info guest physical base.
    pub out_boot_info_guest_phys: u64,
    /// Out-partition IPC consumer queue guest physical base.
    pub out_ipc_consumer_guest_phys: u64,
    /// Total boot info blob size in bytes (includes relay measurement extension).
    pub boot_info_size: u32,
}

/// Complete in-VM relay measurement sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InVmRelayMeasurement {
    /// End-to-end relay frames (conservative cross-check).
    pub frames: u64,
    /// Elapsed TSC ticks from the guest measurement extension.
    pub elapsed_tsc: u64,
    /// Frames derived from the out-partition IPC consumer tail.
    pub ipc_delivered_frames: u64,
    /// Frames reported by the guest boot-info extension.
    pub extension_frames: u64,
}

/// Host-mapped guest boot info site (legacy validate-only tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBootInfoMeasurementSite {
    /// Owning VM id for the boot info blob.
    pub vm_id: VmId,
    /// Guest physical address of the boot info blob.
    pub boot_info_guest_phys: u64,
    /// Total boot info blob size in bytes.
    pub boot_info_size: u32,
}

impl GuestBootInfoMeasurementSite {
    /// Converts a legacy site into an EPT measurement context when tables are available.
    pub fn into_context(
        self,
        ept_tables: EptProgrammedTables,
        out_ipc_consumer_guest_phys: u64,
    ) -> GuestRelayMeasurementContext {
        GuestRelayMeasurementContext {
            ept_tables,
            out_boot_info_guest_phys: self.boot_info_guest_phys,
            out_ipc_consumer_guest_phys,
            boot_info_size: self.boot_info_size,
        }
    }
}

/// Reads the relay measurement extension from guest boot info via EPT resolution.
pub fn read_relay_measurement_extension_from_guest(
    context: &GuestRelayMeasurementContext,
) -> Result<GuestBootInfoRelayMeasurement, CpuSeamError> {
    let bytes = read_guest_bytes_via_ept(
        &context.ept_tables,
        context.out_boot_info_guest_phys,
        context.boot_info_size as usize,
    )?;
    parse_guest_boot_info_relay_measurement(&bytes).ok_or_else(|| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info relay measurement extension invalid",
        )
    })
}

/// Reads delivered frame count from the out-partition IPC consumer queue tail via EPT.
pub fn read_ipc_delivered_frames_from_guest(
    context: &GuestRelayMeasurementContext,
) -> Result<u64, CpuSeamError> {
    if context.out_ipc_consumer_guest_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires out IPC consumer guest address",
        ));
    }
    let header = read_guest_bytes_via_ept(
        &context.ept_tables,
        context.out_ipc_consumer_guest_phys,
        8,
    )?;
    let tail = u32::from_le_bytes([
        header[IPC_QUEUE_TAIL_OFFSET],
        header[IPC_QUEUE_TAIL_OFFSET + 1],
        header[IPC_QUEUE_TAIL_OFFSET + 2],
        header[IPC_QUEUE_TAIL_OFFSET + 3],
    ]);
    Ok(u64::from(tail))
}

/// Measures in-VM relay frames using EPT-aware boot-info and IPC cross-checks.
pub fn measure_in_vm_relay_from_context(
    execution_seam: &DatapathGuestExecutionCpuSeamOutcome,
    context: &GuestRelayMeasurementContext,
    expected_frames: u64,
) -> Result<InVmRelayMeasurement, CpuSeamError> {
    if expected_frames == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires a non-zero expected frame count",
        ));
    }
    if execution_seam.disposition != CpuInstructionDisposition::Executed {
        return Ok(InVmRelayMeasurement {
            frames: 0,
            elapsed_tsc: 0,
            ipc_delivered_frames: 0,
            extension_frames: 0,
        });
    }
    if context.out_boot_info_guest_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires out-partition boot info guest address",
        ));
    }
    let extension = read_relay_measurement_extension_from_guest(context)?;
    let extension_frames = extension.frames_completed;
    let elapsed_tsc = guest_relay_measurement_elapsed_tsc(&extension);
    let ipc_delivered_frames = read_ipc_delivered_frames_from_guest(context).unwrap_or(0);
    let frames = end_to_end_relay_frames(extension_frames, ipc_delivered_frames, expected_frames);
    Ok(InVmRelayMeasurement {
        frames,
        elapsed_tsc,
        ipc_delivered_frames,
        extension_frames,
    })
}

/// Measures in-VM relay frames from legacy measurement sites (EPT tables required).
pub fn measure_in_vm_relay_frames_from_boot_infos(
    execution_seam: &DatapathGuestExecutionCpuSeamOutcome,
    ept_tables: &EptProgrammedTables,
    sites: &[GuestBootInfoMeasurementSite],
    out_ipc_consumer_guest_phys: u64,
    expected_frames: u64,
) -> Result<InVmRelayMeasurement, CpuSeamError> {
    let out_site = sites
        .iter()
        .find(|site| site.vm_id == GUEST_RELAY_MEASUREMENT_VM_ID)
        .ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "relay measurement requires out-partition boot info site",
            )
        })?;
    let context = GuestRelayMeasurementContext {
        ept_tables: ept_tables.clone(),
        out_boot_info_guest_phys: out_site.boot_info_guest_phys,
        out_ipc_consumer_guest_phys,
        boot_info_size: out_site.boot_info_size,
    };
    measure_in_vm_relay_from_context(execution_seam, &context, expected_frames)
}

/// Reads the relay-frame counter from a boot info blob buffer (host-side copy).
pub fn read_relay_frames_completed_from_boot_info_blob(blob: &[u8]) -> Result<u64, CpuSeamError> {
    let extension = parse_guest_boot_info_relay_measurement(blob).ok_or_else(|| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info relay measurement extension invalid",
        )
    })?;
    Ok(extension.frames_completed)
}

fn end_to_end_relay_frames(
    extension_frames: u64,
    ipc_delivered_frames: u64,
    expected_frames: u64,
) -> u64 {
    let mut frames = extension_frames;
    if ipc_delivered_frames > 0 {
        frames = frames.min(ipc_delivered_frames);
    }
    frames.min(expected_frames)
}

fn read_guest_bytes_via_ept(
    tables: &EptProgrammedTables,
    guest_phys: u64,
    len: usize,
) -> Result<alloc::vec::Vec<u8>, CpuSeamError> {
    let host_phys = resolve_guest_phys_to_host(tables, guest_phys).map_err(map_ept_error)?;
    let mut bytes = alloc::vec![0u8; len];
    // SAFETY: resolved host physical range is readable under Gate D EPT mappings.
    unsafe {
        let src = core::slice::from_raw_parts(host_phys as *const u8, len);
        bytes.copy_from_slice(src);
    }
    Ok(bytes)
}

fn map_ept_error(err: hv_ept::EptError) -> CpuSeamError {
    CpuSeamError::new(CpuSeamErrorKind::InvalidInput, err.message)
}

#[cfg(test)]
#[cfg(feature = "datapath-guest-execution")]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_ept::{encode_identity_ept_entry, EptProgrammedMapping, EptProgrammedTables};
    use hv_guest_abi::{
        GUEST_BOOT_INFO_MAGIC, GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION,
        GUEST_RELAY_MEASUREMENT_MAGIC,
    };

    fn sample_ept() -> EptProgrammedTables {
        EptProgrammedTables {
            root_table_phys: 0x2000,
            root_table: alloc::vec![0u8; 4096],
            mappings: vec![EptProgrammedMapping {
                guest_phys: 0,
                host_phys: 0,
                size_bytes: 0x10_0000,
                encoded_entry: encode_identity_ept_entry(0),
            }],
        }
    }

    fn sample_extension_blob(frames: u64) -> [u8; 80] {
        let mut blob = [0u8; 80];
        let total = blob.len() as u32;
        blob[0..8].copy_from_slice(&GUEST_BOOT_INFO_MAGIC);
        blob[8..12].copy_from_slice(&2u32.to_le_bytes());
        blob[12..16].copy_from_slice(&total.to_le_bytes());
        let offset = blob.len() - GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES;
        blob[offset..offset + 4].copy_from_slice(&GUEST_RELAY_MEASUREMENT_MAGIC.to_le_bytes());
        blob[offset + 4..offset + 8]
            .copy_from_slice(&GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION.to_le_bytes());
        blob[offset + 8..offset + 16].copy_from_slice(&frames.to_le_bytes());
        blob
    }

    #[test]
    fn read_relay_frames_completed_reads_extension_counter() {
        let blob = sample_extension_blob(42);
        let frames = read_relay_frames_completed_from_boot_info_blob(&blob).expect("read");
        assert_eq!(frames, 42);
    }

    #[test]
    fn measure_in_vm_relay_returns_zero_when_execution_not_executed() {
        let execution = DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SeamValidated,
            vmexit_stub_validated: true,
            partitions_validated: 3,
            vmlaunch_attempts: 3,
        };
        let context = GuestRelayMeasurementContext {
            ept_tables: sample_ept(),
            out_boot_info_guest_phys: 0x5000,
            out_ipc_consumer_guest_phys: 0x6000,
            boot_info_size: 80,
        };
        let sample = measure_in_vm_relay_from_context(&execution, &context, 64).expect("measure");
        assert_eq!(sample.frames, 0);
    }

    #[test]
    fn end_to_end_relay_frames_uses_ipc_cross_check() {
        assert_eq!(end_to_end_relay_frames(64, 40, 64), 40);
        assert_eq!(end_to_end_relay_frames(32, 0, 64), 32);
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
            boot_info_guest_phys: 0x1000,
            boot_info_size: 80,
        };
        assert!(measure_in_vm_relay_frames_from_boot_infos(
            &execution,
            &sample_ept(),
            &[site],
            0x2000,
            64
        )
        .is_err());
    }
}
