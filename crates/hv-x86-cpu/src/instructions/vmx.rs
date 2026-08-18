//! Live VMX instruction execution.

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Attempts to execute VMXON against the given host-physical VMXON region address.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn execute_vmxon(host_phys: u64) -> Result<(), CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    validate_vmxon_operand(host_phys)?;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "live VMXON requires HV_X86_LIVE_INSTRUCTIONS=1 in ring 0",
        ));
    }
    enable_vmx_in_cr4()?;
    vmxon(host_phys)
}

/// Without live execution support, VMXON is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn execute_vmxon(_host_phys: u64) -> Result<(), CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live VMXON unavailable in this build",
    ))
}

fn validate_vmxon_operand(host_phys: u64) -> Result<(), CpuSeamError> {
    if host_phys == 0 || host_phys & 0xFFF != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMXON region address must be page aligned and non-zero",
        ));
    }
    Ok(())
}

#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
fn enable_vmx_in_cr4() -> Result<(), CpuSeamError> {
    let cr4 = read_cr4();
    let vmxe_mask = 1u64 << 13;
    if cr4 & vmxe_mask == 0 {
        write_cr4(cr4 | vmxe_mask);
    }
    Ok(())
}

#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
fn vmxon(host_phys: u64) -> Result<(), CpuSeamError> {
    let mut cf: u8;
    let mut zf: u8;
    // SAFETY: VMXON is defined when VMX is enabled in CR4 and the region is valid.
    unsafe {
        core::arch::asm!(
            "vmxon [{region}]",
            "setc {cf}",
            "setz {zf}",
            region = in(reg) host_phys,
            cf = out(reg_byte) cf,
            zf = out(reg_byte) zf,
            options(nostack),
        );
    }
    if cf != 0 || zf != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMXON failed (CF/ZF set)",
        ));
    }
    Ok(())
}

#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
fn read_cr4() -> u64 {
    let value: u64;
    // SAFETY: reading CR4 is safe in ring 0; callers gate on environment readiness.
    unsafe {
        core::arch::asm!(
            "mov {0}, cr4",
            out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
fn write_cr4(value: u64) {
    // SAFETY: writing CR4 is required to enable VMX before VMXON.
    unsafe {
        core::arch::asm!(
            "mov cr4, {0}",
            in(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_vmxon_operand_rejects_unaligned_address() {
        assert!(validate_vmxon_operand(0x1001).is_err());
        assert!(validate_vmxon_operand(0).is_err());
    }

    #[test]
    fn execute_vmxon_unavailable_without_live_environment() {
        assert!(execute_vmxon(0x1000).is_err());
    }
}
