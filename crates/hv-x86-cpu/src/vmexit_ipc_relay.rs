//! VM-exit-driven IPC shared-memory relay (Phase 47).

use hv_datapath::{queue_storage_bytes, REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES};

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::vmexit_relay_counter::VM_EXIT_REASON_EPT_VIOLATION;

/// VMCS encoding for guest `RAX` (write value source on typical store exits).
pub const VMCS_GUEST_RAX: u32 = 0x6810;

/// Host-side IPC queue backing installed for VM-exit relay emulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmexitIpcRelayConfig {
    /// Guest physical base of the IPC shared region.
    pub ipc_guest_phys: u64,
    /// IPC region size in bytes.
    pub ipc_region_bytes: u64,
    /// Host physical base backing the IPC region (identity or dedicated state page).
    pub backing_host_phys: u64,
}

/// Initializes reference IPC queue header fields in hypervisor-owned backing memory.
pub fn initialize_ipc_queue_backing(backing_host_phys: u64) -> Result<(), CpuSeamError> {
    if backing_host_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "IPC relay backing host address must be non-zero",
        ));
    }
    let required = queue_storage_bytes(REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES)
        .map_err(map_datapath_error)?;
    if required > usize::MAX {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "IPC queue storage exceeds host addressable range",
        ));
    }
    write_u32_at(backing_host_phys, 8, REFERENCE_IPC_QUEUE_SLOTS)?;
    write_u32_at(backing_host_phys, 12, REFERENCE_IPC_SLOT_SIZE_BYTES)?;
    let slots = read_u32_at(backing_host_phys, 8)?;
    let slot_size = read_u32_at(backing_host_phys, 12)?;
    if slots != REFERENCE_IPC_QUEUE_SLOTS || slot_size != REFERENCE_IPC_SLOT_SIZE_BYTES {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "IPC queue backing header read-back mismatch",
        ));
    }
    Ok(())
}

/// Returns whether an EPT violation exit qualifies as an IPC region write trap.
pub fn is_ipc_region_write_violation(
    guest_phys: u64,
    exit_qualification: u64,
    config: &VmexitIpcRelayConfig,
) -> bool {
    if config.ipc_guest_phys == 0 || config.backing_host_phys == 0 || config.ipc_region_bytes == 0 {
        return false;
    }
    let region_end = config
        .ipc_guest_phys
        .saturating_add(config.ipc_region_bytes.saturating_sub(1));
    if guest_phys < config.ipc_guest_phys || guest_phys > region_end {
        return false;
    }
    exit_qualification & (1 << 1) != 0
}

/// Derives the trapped store width from the VM-exit instruction length.
pub fn ipc_write_size_from_instruction_len(instruction_len: u64) -> u8 {
    let len = instruction_len & 0xF;
    if len <= 2 {
        1
    } else if len <= 6 {
        4
    } else {
        8
    }
}

/// Handles one IPC shared-memory write VM-exit by relaying bytes into host backing.
pub fn handle_ipc_vmexit(
    exit_reason: u32,
    guest_phys: u64,
    exit_qualification: u64,
    guest_rax: u64,
    write_size: u8,
    config: &VmexitIpcRelayConfig,
) -> Result<bool, CpuSeamError> {
    if exit_reason != VM_EXIT_REASON_EPT_VIOLATION {
        return Ok(false);
    }
    if !is_ipc_region_write_violation(guest_phys, exit_qualification, config) {
        return Ok(false);
    }
    let offset = guest_phys.saturating_sub(config.ipc_guest_phys);
    relay_ipc_write(
        config.backing_host_phys,
        config.ipc_region_bytes,
        offset,
        guest_rax,
        write_size,
    )?;
    Ok(true)
}

fn relay_ipc_write(
    backing_host_phys: u64,
    region_bytes: u64,
    offset: u64,
    guest_rax: u64,
    write_size: u8,
) -> Result<(), CpuSeamError> {
    if offset >= region_bytes {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "IPC relay write offset out of region bounds",
        ));
    }
    let max_write = region_bytes.saturating_sub(offset) as usize;
    let size = usize::from(write_size).min(8).min(max_write);
    if size == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "IPC relay write size must be non-zero",
        ));
    }
    let mut bytes = [0u8; 8];
    bytes[..size].copy_from_slice(&guest_rax.to_le_bytes()[..size]);
    let dest = backing_host_phys.saturating_add(offset);
    // SAFETY: caller guarantees the IPC backing range is hypervisor-writable.
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), dest as *mut u8, size);
    }
    let mut read_back = [0u8; 8];
    // SAFETY: caller guarantees the IPC backing range is hypervisor-readable.
    unsafe {
        core::ptr::copy_nonoverlapping(dest as *const u8, read_back.as_mut_ptr(), size);
    }
    if read_back[..size] != bytes[..size] {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "IPC relay backing write read-back mismatch",
        ));
    }
    Ok(())
}

fn write_u32_at(host_phys: u64, offset: u64, value: u32) -> Result<(), CpuSeamError> {
    let bytes = value.to_le_bytes();
    let dest = host_phys.saturating_add(offset);
    // SAFETY: caller guarantees the IPC backing header is hypervisor-writable.
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), dest as *mut u8, 4);
    }
    Ok(())
}

fn read_u32_at(host_phys: u64, offset: u64) -> Result<u32, CpuSeamError> {
    let mut bytes = [0u8; 4];
    let src = host_phys.saturating_add(offset);
    // SAFETY: caller guarantees the IPC backing header is hypervisor-readable.
    unsafe {
        core::ptr::copy_nonoverlapping(src as *const u8, bytes.as_mut_ptr(), 4);
    }
    Ok(u32::from_le_bytes(bytes))
}

fn map_datapath_error(err: hv_datapath::DatapathError) -> CpuSeamError {
    CpuSeamError::new(CpuSeamErrorKind::InvalidInput, err.message)
}

#[cfg(test)]
#[cfg(feature = "datapath-guest-relay-measurement")]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_datapath::REFERENCE_IPC_CHAN_A_GUEST_PHYS;

    fn sample_config(backing: u64) -> VmexitIpcRelayConfig {
        VmexitIpcRelayConfig {
            ipc_guest_phys: REFERENCE_IPC_CHAN_A_GUEST_PHYS,
            ipc_region_bytes: hv_datapath::REFERENCE_IPC_SHARED_BYTES,
            backing_host_phys: backing,
        }
    }

    #[test]
    fn is_ipc_region_write_violation_requires_write_qualification() {
        let config = sample_config(0xA000);
        assert!(is_ipc_region_write_violation(
            REFERENCE_IPC_CHAN_A_GUEST_PHYS,
            1 << 1,
            &config,
        ));
        assert!(!is_ipc_region_write_violation(
            REFERENCE_IPC_CHAN_A_GUEST_PHYS,
            1 << 0,
            &config,
        ));
    }

    #[test]
    fn initialize_ipc_queue_backing_writes_reference_header() {
        let mut page = [0u8; 64];
        let backing = page.as_mut_ptr() as u64;
        initialize_ipc_queue_backing(backing).expect("init");
        assert_eq!(
            read_u32_at(backing, 8).expect("slots"),
            REFERENCE_IPC_QUEUE_SLOTS
        );
        assert_eq!(
            read_u32_at(backing, 12).expect("slot size"),
            REFERENCE_IPC_SLOT_SIZE_BYTES
        );
    }

    #[test]
    fn handle_ipc_vmexit_relays_head_write() {
        let mut page = [0u8; 64];
        let backing = page.as_mut_ptr() as u64;
        initialize_ipc_queue_backing(backing).expect("init");
        let config = sample_config(backing);
        assert!(handle_ipc_vmexit(
            VM_EXIT_REASON_EPT_VIOLATION,
            REFERENCE_IPC_CHAN_A_GUEST_PHYS,
            1 << 1,
            1,
            4,
            &config,
        )
        .expect("handle"));
        assert_eq!(read_u32_at(backing, 0).expect("head"), 1);
    }

    #[test]
    fn ipc_write_size_from_instruction_len_maps_common_store_sizes() {
        assert_eq!(ipc_write_size_from_instruction_len(2), 1);
        assert_eq!(ipc_write_size_from_instruction_len(5), 4);
        assert_eq!(ipc_write_size_from_instruction_len(10), 8);
    }
}
