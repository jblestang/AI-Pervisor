//! VMXON region + CR/MSR fixed-bit bring-up.

/// VMX capability / bring-up errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmxError {
    NotSupported,
    FeatureControlLockedOff,
    InvalidRegion,
    VmxonFailed,
    AlreadyOn,
}

/// 4 KiB VMXON region (revision ID in first 4 bytes per SDM).
#[repr(C, align(4096))]
pub struct VmxonRegion {
    pub data: [u8; 4096],
}

impl VmxonRegion {
    pub const fn zeroed() -> Self {
        Self { data: [0; 4096] }
    }

    /// Write IA32_VMX_BASIC revision id into the region header.
    pub fn set_revision(&mut self, revision: u32) {
        let bytes = revision.to_le_bytes();
        self.data[0..4].copy_from_slice(&bytes);
    }
}

/// Read IA32_VMX_BASIC (0x480) — TCB.
#[cfg(target_arch = "x86_64")]
pub fn read_vmx_basic() -> u64 {
    // SAFETY: RDMSR of architectural VMX capability MSR.
    unsafe { rdmsr(0x480) }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn read_vmx_basic() -> u64 {
    0
}

/// Enable CR4.VMXE and execute VMXON on `region`.
///
/// On host `cfg(test)` / non-freestanding builds this returns [`VmxError::NotSupported`]
/// without executing privileged instructions when nested VT-x is unavailable.
#[cfg(target_os = "none")]
pub fn vmx_enable_and_on(region: &mut VmxonRegion) -> Result<(), VmxError> {
    // SAFETY: Freestanding Type-1 path; caller owns exclusive bring-up.
    unsafe { vmx_enable_and_on_unchecked(region) }
}

#[cfg(not(target_os = "none"))]
pub fn vmx_enable_and_on(_region: &mut VmxonRegion) -> Result<(), VmxError> {
    // Host unit-test / std builds never touch real VMX.
    Err(VmxError::NotSupported)
}

/// Unchecked VMXON sequence (Intel SDM Vol. 3C).
///
/// # Safety
/// Caller must run at CPL0 with a valid VMXON region and exclusive ownership of
/// VMX state on this logical processor.
#[cfg(target_arch = "x86_64")]
pub unsafe fn vmx_enable_and_on_unchecked(region: &mut VmxonRegion) -> Result<(), VmxError> {
    if !cpuid_vmx_supported() {
        return Err(VmxError::NotSupported);
    }

    // IA32_FEATURE_CONTROL (0x3A): lock bit 0, VMX outside SMX bit 2.
    let feat = unsafe { rdmsr(0x3A) };
    let locked = (feat & 1) != 0;
    let vmx_outside = (feat & (1 << 2)) != 0;
    if locked && !vmx_outside {
        return Err(VmxError::FeatureControlLockedOff);
    }
    if !locked {
        unsafe { wrmsr(0x3A, feat | (1 << 2) | 1) };
    }

    let basic = unsafe { rdmsr(0x480) };
    let revision = (basic & 0xffff_ffff) as u32;
    region.set_revision(revision);

    unsafe {
        adjust_cr0_cr4_for_vmx()?;
        // CR4.VMXE = bit 13
        let mut cr4: u64;
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
        cr4 |= 1 << 13;
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));

        let phys = region as *mut VmxonRegion as u64;
        // VMXON m64
        let mut fail: u8;
        core::arch::asm!(
            "vmxon [{}]",
            "setbe {}",
            in(reg) &phys,
            out(reg_byte) fail,
            options(nostack),
        );
        if fail != 0 {
            return Err(VmxError::VmxonFailed);
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn vmx_enable_and_on_unchecked(_region: &mut VmxonRegion) -> Result<(), VmxError> {
    Err(VmxError::NotSupported)
}

#[cfg(target_arch = "x86_64")]
unsafe fn adjust_cr0_cr4_for_vmx() -> Result<(), VmxError> {
    let fixed0_cr0 = unsafe { rdmsr(0x486) };
    let fixed1_cr0 = unsafe { rdmsr(0x487) };
    let fixed0_cr4 = unsafe { rdmsr(0x488) };
    let fixed1_cr4 = unsafe { rdmsr(0x489) };

    let mut cr0: u64;
    let mut cr4: u64;
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nostack, preserves_flags));
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
    }
    cr0 = (cr0 & fixed1_cr0) | fixed0_cr0;
    cr4 = (cr4 & fixed1_cr4) | fixed0_cr4;
    unsafe {
        core::arch::asm!("mov cr0, {}", in(reg) cr0, options(nostack, preserves_flags));
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));
    }
    Ok(())
}

#[cfg(target_arch = "x86_64")]
fn cpuid_vmx_supported() -> bool {
    // SAFETY: CPUID leaf 1 is architectural.
    unsafe {
        let mut ecx: u32;
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "pop rbx",
            inout("eax") 1u32 => _,
            lateout("ecx") ecx,
            lateout("edx") _,
            options(nostack),
        );
        (ecx & (1 << 5)) != 0
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn rdmsr(index: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") index,
            out("eax") lo,
            out("edx") hi,
            options(nostack, preserves_flags),
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

#[cfg(target_arch = "x86_64")]
unsafe fn wrmsr(index: u32, value: u64) {
    let lo = value as u32;
    let hi = (value >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") index,
            in("eax") lo,
            in("edx") hi,
            options(nostack, preserves_flags),
        );
    }
}
