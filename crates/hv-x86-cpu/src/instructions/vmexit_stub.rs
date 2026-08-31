//! VM-exit stub installed at the planned `HOST_RIP` for REAL_HW guest execution.

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Basic VM-exit reason for guest `HLT` (Intel SDM).
#[allow(dead_code)]
pub(crate) const VM_EXIT_REASON_HLT: u32 = 12;

/// VMCS encoding for `VM_EXIT_REASON`.
#[allow(dead_code)]
pub(crate) const VMCS_VM_EXIT_REASON: u32 = 0x4402;

/// Hand-assembled x86_64 stub:
/// ```text
/// mov rdx, 0x4402
/// vmread rax, rdx
/// jc done
/// and eax, 0xffff
/// cmp eax, 12
/// je done
/// vmresume
/// done: ret
/// ```
pub const VMEXIT_STUB_BYTES: [u8; 27] = [
    0x48, 0xC7, 0xC2, 0x02, 0x44, 0x00, 0x00, // mov rdx, 0x4402
    0x48, 0x0F, 0x78, 0xC2, // vmread rax, rdx
    0x72, 0x0E, // jc done (+14)
    0x25, 0xFF, 0xFF, 0x00, 0x00, // and eax, 0xffff
    0x83, 0xF8, 0x0C, // cmp eax, 12
    0x74, 0x02, // je done (+2)
    0x0F, 0x01, 0xC3, // vmresume
    0xC3, // done: ret
];

/// Minimal stub that returns to the host on every VM-exit for Rust dispatch.
pub const VMEXIT_STUB_RET_ONLY_BYTES: [u8; 1] = [0xC3];

/// Installs the VM-exit stub at an identity-mapped host physical address.
pub fn install_vmexit_stub(host_exit_phys: u64) -> Result<(), CpuSeamError> {
    install_vmexit_stub_bytes(host_exit_phys, &VMEXIT_STUB_BYTES)
}

/// Installs the ret-only VM-exit stub used for host-side VM-exit dispatch.
pub fn install_vmexit_stub_ret_only(host_exit_phys: u64) -> Result<(), CpuSeamError> {
    install_vmexit_stub_bytes(host_exit_phys, &VMEXIT_STUB_RET_ONLY_BYTES)
}

fn install_vmexit_stub_bytes(host_exit_phys: u64, bytes: &[u8]) -> Result<(), CpuSeamError> {
    if host_exit_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VM-exit stub address must not be zero",
        ));
    }
    // SAFETY: caller guarantees `host_exit_phys` is identity-mapped writable firmware memory.
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), host_exit_phys as *mut u8, bytes.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::vmexit_stub::install_vmexit_stub_ret_only;

    #[test]
    fn vmexit_stub_bytes_end_with_ret() {
        assert_eq!(VMEXIT_STUB_BYTES.last().copied(), Some(0xC3));
        assert_eq!(VMEXIT_STUB_BYTES.len(), 27);
    }

    #[test]
    fn install_vmexit_stub_rejects_zero_address() {
        assert!(install_vmexit_stub(0).is_err());
    }

    #[test]
    fn install_vmexit_stub_ret_only_writes_ret_byte() {
        let mut buffer = [0u8; 8];
        let host_exit = buffer.as_mut_ptr() as u64;
        install_vmexit_stub_ret_only(host_exit).expect("install");
        assert_eq!(buffer.first().copied(), Some(0xC3));
    }

    #[test]
    fn install_vmexit_stub_writes_bytes_to_buffer() {
        let mut buffer = [0u8; 32];
        let host_exit = buffer.as_mut_ptr() as u64;
        install_vmexit_stub(host_exit).expect("install");
        assert_eq!(
            buffer.get(..VMEXIT_STUB_BYTES.len()),
            Some(VMEXIT_STUB_BYTES.as_slice())
        );
    }
}
