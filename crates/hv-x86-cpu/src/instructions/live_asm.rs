//! Privileged instruction encodings for REAL_HW Gate C (excluded from test coverage).

#![cfg(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
))]

use crate::constants::{CR4_VMXE_BIT, VMCS_EPT_POINTER_FIELD};
use crate::error::{CpuSeamError, CpuSeamErrorKind};

pub fn enable_vmx_in_cr4() -> Result<(), CpuSeamError> {
    let cr4 = read_cr4();
    let vmxe_mask = 1u64 << CR4_VMXE_BIT;
    if cr4 & vmxe_mask == 0 {
        write_cr4(cr4 | vmxe_mask);
    }
    Ok(())
}

pub fn vmxon(host_phys: u64) -> Result<(), CpuSeamError> {
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

pub fn vmclear(vmcs_phys: u64) -> Result<(), CpuSeamError> {
    let mut cf: u8;
    let mut zf: u8;
    // SAFETY: VMCLEAR is defined for a page-aligned VMCS region in VMX root operation.
    unsafe {
        core::arch::asm!(
            "vmclear [{region}]",
            "setc {cf}",
            "setz {zf}",
            region = in(reg) vmcs_phys,
            cf = out(reg_byte) cf,
            zf = out(reg_byte) zf,
            options(nostack),
        );
    }
    if cf != 0 || zf != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMCLEAR failed (CF/ZF set)",
        ));
    }
    Ok(())
}

pub fn vmptrld(vmcs_phys: u64) -> Result<(), CpuSeamError> {
    let mut cf: u8;
    let mut zf: u8;
    // SAFETY: VMPTRLD is defined for a valid VMCS region in VMX root operation.
    unsafe {
        core::arch::asm!(
            "vmptrld [{region}]",
            "setc {cf}",
            "setz {zf}",
            region = in(reg) vmcs_phys,
            cf = out(reg_byte) cf,
            zf = out(reg_byte) zf,
            options(nostack),
        );
    }
    if cf != 0 || zf != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMPTRLD failed (CF/ZF set)",
        ));
    }
    Ok(())
}

pub fn vmwrite(field: u32, value: u64) -> Result<(), CpuSeamError> {
    let mut cf: u8;
    let mut zf: u8;
    // SAFETY: VMWRITE is defined when executing in VMX root operation with a valid VMCS.
    unsafe {
        core::arch::asm!(
            "vmwrite {field}, {value}",
            "setc {cf}",
            "setz {zf}",
            field = in(reg) u64::from(field),
            value = in(reg) value,
            cf = out(reg_byte) cf,
            zf = out(reg_byte) zf,
            options(nostack),
        );
    }
    if cf != 0 || zf != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMWRITE failed (CF/ZF set)",
        ));
    }
    Ok(())
}

pub fn vmwrite_ept_pointer(value: u64) -> Result<(), CpuSeamError> {
    vmwrite(VMCS_EPT_POINTER_FIELD, value)
}

pub fn vmlaunch() -> Result<(), CpuSeamError> {
    let mut cf: u8;
    let mut zf: u8;
    // SAFETY: VMLAUNCH is defined when VMX root operation has a valid current VMCS.
    unsafe {
        core::arch::asm!(
            "vmlaunch",
            "setc {cf}",
            "setz {zf}",
            cf = out(reg_byte) cf,
            zf = out(reg_byte) zf,
            options(nostack),
        );
    }
    if cf != 0 || zf != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMLAUNCH failed (CF/ZF set)",
        ));
    }
    Ok(())
}

/// VMLAUNCH followed by a return from the VM-exit stub when the guest executes `HLT`.
pub fn vmlaunch_wait_for_hlt_exit() -> Result<(), CpuSeamError> {
    let mut cf: u8 = 0;
    let mut zf: u8 = 0;
    let mut guest_done: u64 = 0;
    // SAFETY: HOST_RIP points at the installed exit stub; stub `ret` resumes at `guest_hlt_done`.
    unsafe {
        core::arch::asm!(
            "call 2f",
            "2:",
            "pop {guest_done}",
            "add {guest_done}, {guest_done_offset}",
            "push rax",
            "push {guest_done}",
            "vmlaunch",
            "add rsp, 8",
            "pop rax",
            "setc {cf}",
            "setz {zf}",
            "jmp 4f",
            "3:",
            "pop rax",
            "4:",
            guest_done = out(reg) guest_done,
            guest_done_offset = const GUEST_HLT_DONE_OFFSET,
            cf = out(reg_byte) cf,
            zf = out(reg_byte) zf,
            options(nostack),
        );
    }
    if cf != 0 || zf != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "VMLAUNCH failed (CF/ZF set)",
        ));
    }
    Ok(())
}

/// Byte offset from the `pop` in the call/pop sequence to label `3` (`guest_hlt_done`).
const GUEST_HLT_DONE_OFFSET: u64 = 19;

pub fn rdmsr(msr: u32) -> Result<(u32, u32), CpuSeamError> {
    let mut low: u32;
    let mut high: u32;
    // SAFETY: RDMSR is defined for valid architectural MSRs in ring 0.
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            lateout("eax") low,
            lateout("edx") high,
            options(nostack, preserves_flags),
        );
    }
    Ok((low, high))
}

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
