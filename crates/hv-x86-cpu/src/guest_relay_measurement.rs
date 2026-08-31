//! In-VM guest relay frame measurement via boot-info extension and EPT reads (Phase 29–31).

use hv_ept::{resolve_guest_phys_range_to_host, EptProgrammedTables};
use hv_guest_abi::{
    parse_guest_boot_info_relay_measurement, parse_relay_measurement_page_header,
    GuestBootInfoRelayMeasurement, GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES,
    GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION, GUEST_RELAY_MEASUREMENT_MAGIC,
};
use hv_types::VmId;

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::instructions::{hypervisor_elapsed_tsc, validate_hypervisor_tsc_bracket};
use crate::seams::{CpuInstructionDisposition, DatapathGuestExecutionCpuSeamOutcome};
use crate::vmexit_relay_counter::{
    read_relay_measurement_page_frames, validate_vmexit_relay_frame_count,
};

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
    /// Hypervisor physical base of the relay measurement page when installed.
    pub measurement_page_host_phys: Option<u64>,
    /// Total boot info blob size in bytes (includes relay measurement extension).
    pub boot_info_size: u32,
}

/// Complete in-VM relay measurement sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InVmRelayMeasurement {
    /// End-to-end relay frames (conservative cross-check).
    pub frames: u64,
    /// Elapsed TSC ticks from the hypervisor execution bracket.
    pub elapsed_tsc: u64,
    /// Frames derived from the out-partition IPC consumer tail.
    pub ipc_delivered_frames: u64,
    /// Frames published on the hypervisor-owned measurement page.
    pub extension_frames: u64,
    /// Frames counted by the hypervisor on VM-exits during out-partition execution.
    pub vmexit_relay_frames: u64,
    /// Frames reported by the guest boot-info tail (cross-check input).
    pub guest_boot_info_frames: u64,
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
            measurement_page_host_phys: None,
            boot_info_size: self.boot_info_size,
        }
    }
}

/// Reads the relay measurement extension from an installed guest boot info blob via EPT.
pub fn read_relay_measurement_extension_from_installed_boot_info(
    ept_tables: &EptProgrammedTables,
    boot_info_guest_phys: u64,
    boot_info_size: u32,
) -> Result<GuestBootInfoRelayMeasurement, CpuSeamError> {
    if boot_info_guest_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires installed boot info guest address",
        ));
    }
    let bytes =
        read_guest_bytes_via_ept(ept_tables, boot_info_guest_phys, boot_info_size as usize)?;
    parse_guest_boot_info_relay_measurement(&bytes).ok_or_else(|| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "installed guest boot info relay measurement extension invalid",
        )
    })
}

/// Reads the relay measurement extension from the hypervisor-owned page or boot info via EPT.
pub fn read_relay_measurement_extension_from_guest(
    context: &GuestRelayMeasurementContext,
) -> Result<GuestBootInfoRelayMeasurement, CpuSeamError> {
    if let Some(host_phys) = context.measurement_page_host_phys {
        let bytes = read_host_bytes(host_phys, GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES)?;
        return parse_relay_measurement_page_header(&bytes).ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "relay measurement page header invalid",
            )
        });
    }
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
    let header =
        read_guest_bytes_via_ept(&context.ept_tables, context.out_ipc_consumer_guest_phys, 8)?;
    let tail_bytes = header
        .get(IPC_QUEUE_TAIL_OFFSET..IPC_QUEUE_TAIL_OFFSET + 4)
        .ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "out IPC consumer header shorter than queue tail field",
            )
        })?;
    let tail_array: [u8; 4] = tail_bytes.try_into().map_err(|_| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "out IPC consumer queue tail field unreadable",
        )
    })?;
    let tail = u32::from_le_bytes(tail_array);
    Ok(u64::from(tail))
}

/// Publishes authoritative relay measurement counters to the hypervisor-owned page.
pub fn publish_relay_measurement_page_authoritative(
    context: &GuestRelayMeasurementContext,
    expected_frames: u64,
    hypervisor_tsc_start: u64,
    hypervisor_tsc_end: u64,
    vmexit_relay_frames: u64,
) -> Result<(), CpuSeamError> {
    let host_phys = context.measurement_page_host_phys.ok_or_else(|| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires hypervisor-owned measurement page",
        )
    })?;
    let boot_extension = read_relay_measurement_extension_from_installed_boot_info(
        &context.ept_tables,
        context.out_boot_info_guest_phys,
        context.boot_info_size,
    )?;
    validate_boot_relay_measurement_extension(&boot_extension)?;
    validate_hypervisor_tsc_bracket(hypervisor_tsc_start, hypervisor_tsc_end)?;
    let ipc_delivered_frames = read_ipc_delivered_frames_from_guest(context)?;
    validate_vmexit_relay_frame_count(vmexit_relay_frames, ipc_delivered_frames, expected_frames)?;
    let page_frames = read_relay_measurement_page_frames(host_phys)?;
    if page_frames != vmexit_relay_frames {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement page frame count mismatch with VM-exit counter",
        ));
    }
    let frames_completed = vmexit_relay_frames.min(expected_frames);
    let published = GuestBootInfoRelayMeasurement {
        magic: boot_extension.magic,
        version: boot_extension.version,
        frames_completed,
        tsc_start: hypervisor_tsc_start,
        tsc_end: hypervisor_tsc_end,
        measurement_page_gpa: boot_extension.measurement_page_gpa,
    };
    write_host_measurement_extension(host_phys, &published)?;
    let host_extension = read_host_measurement_extension(host_phys)?;
    if host_extension.frames_completed != frames_completed {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "published relay measurement page frame count mismatch",
        ));
    }
    if host_extension.tsc_start != hypervisor_tsc_start
        || host_extension.tsc_end != hypervisor_tsc_end
    {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "published relay measurement page TSC bracket mismatch",
        ));
    }
    Ok(())
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
            vmexit_relay_frames: 0,
            guest_boot_info_frames: 0,
        });
    }
    if execution_seam.vmexit_relay_frames == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires non-zero VM-exit relay frame count",
        ));
    }
    if context.measurement_page_host_phys.is_none() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires hypervisor-owned measurement page",
        ));
    }
    if context.out_boot_info_guest_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires out-partition boot info guest address",
        ));
    }
    let guest_boot_extension = read_relay_measurement_extension_from_installed_boot_info(
        &context.ept_tables,
        context.out_boot_info_guest_phys,
        context.boot_info_size,
    )?;
    publish_relay_measurement_page_authoritative(
        context,
        expected_frames,
        execution_seam.hypervisor_tsc_start,
        execution_seam.hypervisor_tsc_end,
        execution_seam.vmexit_relay_frames,
    )?;
    let host_extension = read_relay_measurement_extension_from_guest(context)?;
    let guest_boot_info_frames = guest_boot_extension.frames_completed;
    let extension_frames = host_extension.frames_completed;
    let vmexit_relay_frames = execution_seam.vmexit_relay_frames;
    let elapsed_tsc = hypervisor_elapsed_tsc(
        execution_seam.hypervisor_tsc_start,
        execution_seam.hypervisor_tsc_end,
    );
    if elapsed_tsc == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires non-zero hypervisor TSC elapsed time",
        ));
    }
    if host_extension.tsc_start != execution_seam.hypervisor_tsc_start
        || host_extension.tsc_end != execution_seam.hypervisor_tsc_end
    {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement page TSC bracket mismatch with execution seam",
        ));
    }
    let ipc_delivered_frames = read_ipc_delivered_frames_from_guest(context)?;
    if ipc_delivered_frames == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement requires non-zero IPC delivered frame count",
        ));
    }
    let frames =
        end_to_end_relay_frames(vmexit_relay_frames, ipc_delivered_frames, expected_frames);
    if guest_boot_info_frames > ipc_delivered_frames && guest_boot_info_frames > 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot-info frame count exceeds IPC delivered frames",
        ));
    }
    if extension_frames != vmexit_relay_frames.min(expected_frames) {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement authoritative frame count mismatch with VM-exit counter",
        ));
    }
    Ok(InVmRelayMeasurement {
        frames,
        elapsed_tsc,
        ipc_delivered_frames,
        extension_frames,
        vmexit_relay_frames,
        guest_boot_info_frames,
    })
}

/// Measures in-VM relay frames from legacy measurement sites (EPT tables required).
pub fn measure_in_vm_relay_frames_from_boot_infos(
    execution_seam: &DatapathGuestExecutionCpuSeamOutcome,
    ept_tables: &EptProgrammedTables,
    sites: &[GuestBootInfoMeasurementSite],
    out_ipc_consumer_guest_phys: u64,
    measurement_page_host_phys: Option<u64>,
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
        measurement_page_host_phys,
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
    vmexit_frames: u64,
    ipc_delivered_frames: u64,
    expected_frames: u64,
) -> u64 {
    vmexit_frames.min(ipc_delivered_frames).min(expected_frames)
}

fn read_guest_bytes_via_ept(
    tables: &EptProgrammedTables,
    guest_phys: u64,
    len: usize,
) -> Result<alloc::vec::Vec<u8>, CpuSeamError> {
    let host_phys =
        resolve_guest_phys_range_to_host(tables, guest_phys, len).map_err(map_ept_error)?;
    read_host_bytes(host_phys, len)
}

fn read_host_bytes(host_phys: u64, len: usize) -> Result<alloc::vec::Vec<u8>, CpuSeamError> {
    let mut bytes = alloc::vec![0u8; len];
    // SAFETY: host physical range is readable under Gate D resident installs.
    unsafe {
        let src = core::slice::from_raw_parts(host_phys as *const u8, len);
        bytes.copy_from_slice(src);
    }
    Ok(bytes)
}

fn write_host_measurement_extension(
    host_phys: u64,
    extension: &GuestBootInfoRelayMeasurement,
) -> Result<(), CpuSeamError> {
    let mut bytes = [0u8; GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES];
    bytes[0..4].copy_from_slice(&extension.magic.to_le_bytes());
    bytes[4..8].copy_from_slice(&extension.version.to_le_bytes());
    bytes[8..16].copy_from_slice(&extension.frames_completed.to_le_bytes());
    bytes[16..24].copy_from_slice(&extension.tsc_start.to_le_bytes());
    bytes[24..32].copy_from_slice(&extension.tsc_end.to_le_bytes());
    bytes[32..40].copy_from_slice(&extension.measurement_page_gpa.to_le_bytes());
    // SAFETY: host physical measurement page is writable by the hypervisor.
    unsafe {
        let dest = core::slice::from_raw_parts_mut(host_phys as *mut u8, bytes.len());
        dest.copy_from_slice(&bytes);
    }
    Ok(())
}

fn read_host_measurement_extension(
    host_phys: u64,
) -> Result<GuestBootInfoRelayMeasurement, CpuSeamError> {
    let bytes = read_host_bytes(host_phys, GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES)?;
    parse_relay_measurement_page_header(&bytes).ok_or_else(|| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement page header invalid",
        )
    })
}

fn validate_boot_relay_measurement_extension(
    extension: &GuestBootInfoRelayMeasurement,
) -> Result<(), CpuSeamError> {
    if extension.magic != GUEST_RELAY_MEASUREMENT_MAGIC {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info relay measurement magic invalid",
        ));
    }
    if extension.version == 0 || extension.version > GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info relay measurement version invalid",
        ));
    }
    if extension.measurement_page_gpa == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info relay measurement page GPA missing",
        ));
    }
    Ok(())
}

fn map_ept_error(err: hv_ept::EptError) -> CpuSeamError {
    CpuSeamError::new(CpuSeamErrorKind::InvalidInput, err.message)
}

#[cfg(test)]
#[cfg(feature = "datapath-guest-execution")]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
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
                guest_writable: true,
                encoded_entry: encode_identity_ept_entry(0),
            }],
            paging_tables: alloc::vec::Vec::new(),
        }
    }

    fn sample_extension_blob(frames: u64) -> [u8; 88] {
        let mut blob = [0u8; 88];
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
            hypervisor_tsc_start: 0,
            hypervisor_tsc_end: 0,
            vmexit_relay_frames: 0,
        };
        let context = GuestRelayMeasurementContext {
            ept_tables: sample_ept(),
            out_boot_info_guest_phys: 0x5000,
            out_ipc_consumer_guest_phys: 0x6000,
            measurement_page_host_phys: None,
            boot_info_size: 88,
        };
        let sample = measure_in_vm_relay_from_context(&execution, &context, 64).expect("measure");
        assert_eq!(sample.frames, 0);
    }

    #[test]
    fn end_to_end_relay_frames_uses_ipc_cross_check() {
        assert_eq!(end_to_end_relay_frames(64, 40, 64), 40);
        assert_eq!(end_to_end_relay_frames(32, 48, 64), 32);
    }

    #[test]
    fn validate_boot_relay_measurement_extension_requires_page_gpa() {
        let extension = GuestBootInfoRelayMeasurement {
            magic: GUEST_RELAY_MEASUREMENT_MAGIC,
            version: GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION,
            frames_completed: 0,
            tsc_start: 0,
            tsc_end: 0,
            measurement_page_gpa: 0,
        };
        assert!(validate_boot_relay_measurement_extension(&extension).is_err());
    }

    #[test]
    fn measure_in_vm_relay_frames_requires_out_partition_site() {
        let execution = DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::Executed,
            vmexit_stub_validated: true,
            partitions_validated: 3,
            vmlaunch_attempts: 3,
            hypervisor_tsc_start: 0,
            hypervisor_tsc_end: 0,
            vmexit_relay_frames: 0,
        };
        let site = GuestBootInfoMeasurementSite {
            vm_id: VmId::new(0),
            boot_info_guest_phys: 0x1000,
            boot_info_size: 88,
        };
        assert!(measure_in_vm_relay_frames_from_boot_infos(
            &execution,
            &sample_ept(),
            &[site],
            0x2000,
            None,
            64
        )
        .is_err());
    }
}
