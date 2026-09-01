//! VM-exit-driven e1000 MMIO relay emulation (Phase 45).

use core::mem::size_of;

use hv_datapath::{
    handle_e1000_mmio_read, handle_e1000_mmio_write, E1000MmioState, E1000_REG_RDH, E1000_REG_RDT,
    E1000_REG_TDH, E1000_REG_TDT, E1000_MMIO_SIZE_BYTES,
};

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::vmexit_relay_counter::VM_EXIT_REASON_EPT_VIOLATION;

/// Host-side e1000 MMIO relay state installed for VM-exit emulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmexitE1000MmioConfig {
    /// Guest physical base of the e1000 MMIO window.
    pub mmio_guest_phys: u64,
    /// Host physical base of the emulated `E1000MmioState` page.
    pub state_host_phys: u64,
}

/// Returns whether an EPT violation exit qualifies as an e1000 MMIO write trap.
pub fn is_e1000_mmio_write_violation(
    guest_phys: u64,
    exit_qualification: u64,
    config: &VmexitE1000MmioConfig,
) -> bool {
    if config.mmio_guest_phys == 0 || config.state_host_phys == 0 {
        return false;
    }
    let page_end = config
        .mmio_guest_phys
        .saturating_add(E1000_MMIO_SIZE_BYTES.saturating_sub(1));
    if guest_phys < config.mmio_guest_phys || guest_phys > page_end {
        return false;
    }
    exit_qualification & (1 << 1) != 0
}

/// Initializes the identity-mapped guest MMIO view from hypervisor-side state.
pub fn initialize_e1000_mmio_guest_view(
    state: &E1000MmioState,
    mmio_guest_phys: u64,
) -> Result<(), CpuSeamError> {
    mirror_e1000_mmio_guest_view(state, mmio_guest_phys)
}

/// Handles one e1000 MMIO write VM-exit by updating hypervisor-side MMIO state.
pub fn handle_e1000_mmio_vmexit(
    exit_reason: u32,
    guest_phys: u64,
    exit_qualification: u64,
    guest_rax: u64,
    config: &VmexitE1000MmioConfig,
) -> Result<bool, CpuSeamError> {
    if exit_reason != VM_EXIT_REASON_EPT_VIOLATION {
        return Ok(false);
    }
    if !is_e1000_mmio_write_violation(guest_phys, exit_qualification, config) {
        return Ok(false);
    }
    let offset = guest_phys.saturating_sub(config.mmio_guest_phys);
    let state = read_mmio_state(config.state_host_phys)?;
    let mut updated = state;
    let write_value = e1000_write_value_for_offset(offset, guest_rax);
    handle_e1000_mmio_write(&mut updated, offset, write_value).map_err(map_datapath_error)?;
    write_mmio_state(config.state_host_phys, &updated)?;
    mirror_e1000_mmio_guest_view(&updated, config.mmio_guest_phys)?;
    Ok(true)
}

/// Reads one emulated e1000 register from hypervisor-side state.
pub fn read_e1000_mmio_register(
    config: &VmexitE1000MmioConfig,
    offset: u64,
) -> Result<u64, CpuSeamError> {
    let state = read_mmio_state(config.state_host_phys)?;
    handle_e1000_mmio_read(&state, offset).map_err(map_datapath_error)
}

fn e1000_write_value_for_offset(offset: u64, guest_rax: u64) -> u64 {
    match offset {
        E1000_REG_TDT | E1000_REG_RDT => guest_rax & 0xFFFF_FFFF,
        _ => guest_rax & 0xFFFF_FFFF,
    }
}

fn mirror_e1000_mmio_guest_view(
    state: &E1000MmioState,
    mmio_guest_phys: u64,
) -> Result<(), CpuSeamError> {
    if mmio_guest_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "e1000 MMIO guest view base must be non-zero",
        ));
    }
    for (offset, value) in [
        (E1000_REG_TDH, u64::from(state.tdh)),
        (E1000_REG_TDT, u64::from(state.tdt)),
        (E1000_REG_RDH, u64::from(state.rdh)),
        (E1000_REG_RDT, u64::from(state.rdt)),
    ] {
        mirror_u32_at(mmio_guest_phys, offset, value)?;
    }
    Ok(())
}

fn mirror_u32_at(base: u64, offset: u64, value: u64) -> Result<(), CpuSeamError> {
    let address = base
        .checked_add(offset)
        .ok_or_else(|| {
            CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "e1000 MMIO mirror overflow")
        })? as *mut u32;
    // SAFETY: caller guarantees the identity-mapped MMIO GPA is writable host memory.
    unsafe {
        core::ptr::write_volatile(address, u32::try_from(value).map_err(|_| {
            CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "e1000 MMIO mirror value overflow")
        })?);
    }
    Ok(())
}

fn read_mmio_state(host_phys: u64) -> Result<E1000MmioState, CpuSeamError> {
    if host_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "e1000 MMIO relay state host address must be non-zero",
        ));
    }
    let mut bytes = [0u8; size_of::<E1000MmioState>()];
    // SAFETY: caller guarantees the host state page is readable.
    unsafe {
        core::ptr::copy_nonoverlapping(host_phys as *const u8, bytes.as_mut_ptr(), bytes.len());
    }
    Ok(E1000MmioState {
        tdh: u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| {
            CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "e1000 tdh unreadable")
        })?),
        tdt: u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| {
            CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "e1000 tdt unreadable")
        })?),
        rdh: u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| {
            CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "e1000 rdh unreadable")
        })?),
        rdt: u32::from_le_bytes(bytes[12..16].try_into().map_err(|_| {
            CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "e1000 rdt unreadable")
        })?),
        tx_doorbell: bytes[16] != 0,
        rx_doorbell: bytes[17] != 0,
    })
}

fn write_mmio_state(host_phys: u64, state: &E1000MmioState) -> Result<(), CpuSeamError> {
    let mut bytes = [0u8; size_of::<E1000MmioState>()];
    bytes[0..4].copy_from_slice(&state.tdh.to_le_bytes());
    bytes[4..8].copy_from_slice(&state.tdt.to_le_bytes());
    bytes[8..12].copy_from_slice(&state.rdh.to_le_bytes());
    bytes[12..16].copy_from_slice(&state.rdt.to_le_bytes());
    bytes[16] = u8::from(state.tx_doorbell);
    bytes[17] = u8::from(state.rx_doorbell);
    // SAFETY: caller guarantees the host state page is writable.
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), host_phys as *mut u8, bytes.len());
    }
    let read_back = read_mmio_state(host_phys)?;
    if read_back != *state {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "e1000 MMIO relay state write read-back mismatch",
        ));
    }
    Ok(())
}

fn map_datapath_error(err: hv_datapath::DatapathError) -> CpuSeamError {
    CpuSeamError::new(CpuSeamErrorKind::InvalidInput, err.message)
}

#[cfg(test)]
#[cfg(feature = "datapath-guest-relay-measurement")]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_datapath::E1000_REG_TDT;

    fn sample_config(host_phys: u64, mmio_guest_phys: u64) -> VmexitE1000MmioConfig {
        VmexitE1000MmioConfig {
            mmio_guest_phys,
            state_host_phys: host_phys,
        }
    }

    #[test]
    fn is_e1000_mmio_write_violation_requires_write_qualification() {
        let config = sample_config(0x9000, 0xFEB0_0000);
        assert!(is_e1000_mmio_write_violation(
            0xFEB0_0000 + E1000_REG_TDT,
            1 << 1,
            &config,
        ));
        assert!(!is_e1000_mmio_write_violation(
            0xFEB0_0000 + E1000_REG_TDT,
            1 << 0,
            &config,
        ));
    }

    #[test]
    fn handle_e1000_mmio_vmexit_updates_host_state_and_guest_view() {
        let mut page = [0u8; 64];
        let state_host = page.as_mut_ptr() as u64;
        let mut guest_view = [0u8; 64];
        let mmio_guest_phys = guest_view.as_mut_ptr() as u64;
        let config = VmexitE1000MmioConfig {
            mmio_guest_phys,
            state_host_phys: state_host,
        };
        assert!(handle_e1000_mmio_vmexit(
            VM_EXIT_REASON_EPT_VIOLATION,
            mmio_guest_phys + E1000_REG_TDT,
            1 << 1,
            7,
            &config,
        )
        .expect("handle"));
        let state = read_mmio_state(state_host).expect("read");
        assert!(state.tx_doorbell);
        assert_eq!(state.tdt, 7);
        let mirrored = unsafe { core::ptr::read_volatile((mmio_guest_phys + E1000_REG_TDT) as *const u32) };
        assert_eq!(mirrored, 7);
    }

    #[test]
    fn read_e1000_mmio_register_returns_emulated_value() {
        let mut page = [0u8; 64];
        let state_host = page.as_mut_ptr() as u64;
        let config = sample_config(state_host, 0xFEB0_0000);
        let mut state = E1000MmioState::default();
        state.rdt = 9;
        write_mmio_state(state_host, &state).expect("write");
        assert_eq!(
            read_e1000_mmio_register(&config, E1000_REG_RDT).expect("read"),
            9
        );
    }
}
