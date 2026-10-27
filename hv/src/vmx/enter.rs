//! Host-testable L2 launch image + freestanding VMCS load / VMLAUNCH path.
//!
//! [`GuestLaunchImage`] captures every field the freestanding path will
//! `vmwrite` — unit tests assert completeness without executing VT-x.

use crate::vmx::controls::AdjustedControls;
use crate::vmx::vmcs::VmcsField;

/// Why preparing or attempting an L2 enter failed (software / policy).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchError {
    MissingEptp,
    MissingEntry,
    MissingCr3,
    VmcsHw,
    VmxNotOn,
}

/// Software snapshot of VMCS fields for one guest enter (host-testable).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestLaunchImage {
    pub entry_rip: u64,
    pub entry_rsp: u64,
    pub entry_rdi: u64,
    pub guest_cr0: u64,
    pub guest_cr3: u64,
    pub guest_cr4: u64,
    pub guest_efer: u64,
    pub guest_rflags: u64,
    pub eptp: u64,
    pub controls: AdjustedControls,
    pub guest_gdtr_base: u64,
    pub guest_gdtr_limit: u32,
    pub guest_cs: u16,
    pub guest_ss: u16,
    pub guest_tr: u16,
}

/// Inputs derived from layout + loaded guest (no lab literals).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaunchTips {
    pub entry_rip: u64,
    pub entry_rsp: u64,
    pub boot_info_gpa: u64,
    pub guest_cr3: u64,
    pub eptp: u64,
    pub guest_gdtr_base: u64,
}

/// Fixed long-mode CR0/CR4/EFER skeleton (fixed bits applied by freestanding path).
pub const GUEST_CR0_BASE: u64 = (1 << 0) | (1 << 1) | (1 << 5) | (1 << 31); // PE|MP|NE|PG
pub const GUEST_CR4_BASE: u64 = (1 << 4) | (1 << 5) | (1 << 7); // PSE|PAE|PGE
pub const GUEST_EFER_LMA_LME: u64 = (1 << 8) | (1 << 10); // LME|LMA
pub const GUEST_RFLAGS: u64 = 0x2;

/// Build a launch image from tips + already-adjusted controls.
pub fn prepare_launch_image(
    tips: LaunchTips,
    controls: AdjustedControls,
) -> Result<GuestLaunchImage, LaunchError> {
    if tips.eptp == 0 {
        return Err(LaunchError::MissingEptp);
    }
    if tips.entry_rip == 0 {
        return Err(LaunchError::MissingEntry);
    }
    if tips.guest_cr3 == 0 {
        return Err(LaunchError::MissingCr3);
    }
    Ok(GuestLaunchImage {
        entry_rip: tips.entry_rip,
        entry_rsp: tips.entry_rsp,
        entry_rdi: tips.boot_info_gpa,
        guest_cr0: GUEST_CR0_BASE,
        guest_cr3: tips.guest_cr3,
        guest_cr4: GUEST_CR4_BASE,
        guest_efer: GUEST_EFER_LMA_LME,
        guest_rflags: GUEST_RFLAGS,
        eptp: tips.eptp,
        controls,
        guest_gdtr_base: tips.guest_gdtr_base,
        guest_gdtr_limit: 0x17,
        guest_cs: 0x08,
        guest_ss: 0x10,
        guest_tr: 0x18,
    })
}

/// Encode a subset of [`GuestLaunchImage`] as (field, value) pairs for tests / tracers.
pub fn launch_field_pairs(img: &GuestLaunchImage) -> alloc::vec::Vec<(VmcsField, u64)> {
    alloc::vec![
        (VmcsField::GuestRip, img.entry_rip),
        (VmcsField::GuestRsp, img.entry_rsp),
        (VmcsField::GuestRflags, img.guest_rflags),
        (VmcsField::GuestCr0, img.guest_cr0),
        (VmcsField::GuestCr3, img.guest_cr3),
        (VmcsField::GuestCr4, img.guest_cr4),
        (VmcsField::EptPointer, img.eptp),
        (
            VmcsField::PinBasedVmExecControl,
            u64::from(img.controls.pin)
        ),
        (
            VmcsField::CpuBasedVmExecControl,
            u64::from(img.controls.primary)
        ),
        (
            VmcsField::SecondaryVmExecControl,
            u64::from(img.controls.secondary)
        ),
        (VmcsField::VmExitControls, u64::from(img.controls.exit)),
        (VmcsField::VmEntryControls, u64::from(img.controls.entry)),
    ]
}

/// Install a minimal guest GDT (null / 64-bit code / data / TSS) at `dest`.
///
/// `dest` must be 32 bytes (4 descriptors). TSS base is a placeholder at
/// `tss_base` (usually immediately after the GDT).
pub fn write_guest_gdt(dest: &mut [u8], tss_base: u64) {
    assert!(dest.len() >= 32);
    dest[..32].fill(0);
    // CS: 0x00AF9A000000FFFF — 64-bit code
    dest[8..16].copy_from_slice(&0x00AF_9A00_0000_FFFFu64.to_le_bytes());
    // SS/DS: 0x00CF92000000FFFF — data
    dest[16..24].copy_from_slice(&0x00CF_9200_0000_FFFFu64.to_le_bytes());
    // TR: 64-bit busy TSS — limit 0x67, type 0x8B; base = tss_base low bits
    let base = tss_base;
    let lim = 0x67u64;
    let mut tss = 0u64;
    tss |= lim & 0xFFFF;
    tss |= (base & 0xFFFFFF) << 16;
    tss |= 0x8Bu64 << 40; // type busy TSS64 + P
    tss |= ((base >> 24) & 0xFF) << 56;
    dest[24..32].copy_from_slice(&tss.to_le_bytes());
    // High half of TSS descriptor (base 63:32) sits in next 8 bytes if available.
    if dest.len() >= 40 {
        dest[32..40].copy_from_slice(&(base >> 32).to_le_bytes());
    }
}

/// Freestanding: load VMCS from image + host state, then VMLAUNCH / VMRESUME.
///
/// Returns `Ok(exit_reason)` after a VM-exit, or `Err` if the instruction fails.
#[cfg(all(target_os = "none", target_arch = "x86_64"))]
pub unsafe fn enter_guest(
    vcpu: &mut crate::vmx::vmcs::Vcpu,
    img: &GuestLaunchImage,
    launched: bool,
) -> Result<u32, LaunchError> {
    use crate::vmx::vmcs::hw;

    let rev = (crate::vmx::vmxon::read_vmx_basic() & 0xffff_ffff) as u32;
    vcpu.vmcs.set_revision(rev);
    hw::vmclear(&mut vcpu.vmcs).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmptrld(&mut vcpu.vmcs).map_err(|_| LaunchError::VmcsHw)?;

    write_guest_state(img)?;
    write_host_state()?;
    write_controls(img)?;

    let fail = vmx_launch_with_rdi(img.entry_rdi, launched);
    if fail != 0 {
        return Err(LaunchError::VmcsHw);
    }
    let reason = hw::vmread(VmcsField::VmExitReason).map_err(|_| LaunchError::VmcsHw)?;
    Ok(reason as u32)
}

#[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
pub unsafe fn enter_guest(
    _vcpu: &mut crate::vmx::vmcs::Vcpu,
    _img: &GuestLaunchImage,
    _launched: bool,
) -> Result<u32, LaunchError> {
    Err(LaunchError::VmxNotOn)
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
unsafe fn write_controls(img: &GuestLaunchImage) -> Result<(), LaunchError> {
    use crate::vmx::vmcs::hw;
    hw::vmwrite(
        VmcsField::PinBasedVmExecControl,
        u64::from(img.controls.pin),
    )
    .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(
        VmcsField::CpuBasedVmExecControl,
        u64::from(img.controls.primary),
    )
    .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(
        VmcsField::SecondaryVmExecControl,
        u64::from(img.controls.secondary),
    )
    .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::VmExitControls, u64::from(img.controls.exit))
        .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::VmEntryControls, u64::from(img.controls.entry))
        .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::EptPointer, img.eptp).map_err(|_| LaunchError::VmcsHw)?;
    Ok(())
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
unsafe fn write_guest_state(img: &GuestLaunchImage) -> Result<(), LaunchError> {
    use crate::vmx::vmcs::hw;
    let fixed0_cr0 = rdmsr(0x486);
    let fixed1_cr0 = rdmsr(0x487);
    let fixed0_cr4 = rdmsr(0x488);
    let fixed1_cr4 = rdmsr(0x489);
    let cr0 = (img.guest_cr0 & fixed1_cr0) | fixed0_cr0;
    let cr4 = (img.guest_cr4 & fixed1_cr4) | fixed0_cr4;

    hw::vmwrite(VmcsField::GuestCr0, cr0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestCr3, img.guest_cr3).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestCr4, cr4).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestRip, img.entry_rip).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestRsp, img.entry_rsp).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestRflags, img.guest_rflags).map_err(|_| LaunchError::VmcsHw)?;

    hw::vmwrite(VmcsField::GuestCsSelector, u64::from(img.guest_cs))
        .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestSsSelector, u64::from(img.guest_ss))
        .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestDsSelector, u64::from(img.guest_ss))
        .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestEsSelector, u64::from(img.guest_ss))
        .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestFsSelector, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestGsSelector, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestLdtrSelector, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestTrSelector, u64::from(img.guest_tr))
        .map_err(|_| LaunchError::VmcsHw)?;

    hw::vmwrite(VmcsField::GuestCsAccess, 0xA09B).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestSsAccess, 0xC093).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestDsAccess, 0xC093).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestEsAccess, 0xC093).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestFsAccess, 0xC093).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestGsAccess, 0xC093).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestLdtrAccess, 0x10000).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestTrAccess, 0x8B).map_err(|_| LaunchError::VmcsHw)?;

    for f in [
        VmcsField::GuestCsBase,
        VmcsField::GuestSsBase,
        VmcsField::GuestDsBase,
        VmcsField::GuestEsBase,
        VmcsField::GuestFsBase,
        VmcsField::GuestGsBase,
        VmcsField::GuestLdtrBase,
    ] {
        hw::vmwrite(f, 0).map_err(|_| LaunchError::VmcsHw)?;
    }
    hw::vmwrite(VmcsField::GuestTrBase, img.guest_gdtr_base + 0x1000)
        .map_err(|_| LaunchError::VmcsHw)?;

    for f in [
        VmcsField::GuestCsLimit,
        VmcsField::GuestSsLimit,
        VmcsField::GuestDsLimit,
        VmcsField::GuestEsLimit,
        VmcsField::GuestFsLimit,
        VmcsField::GuestGsLimit,
        VmcsField::GuestLdtrLimit,
    ] {
        hw::vmwrite(f, 0).map_err(|_| LaunchError::VmcsHw)?;
    }
    hw::vmwrite(VmcsField::GuestTrLimit, 0x67).map_err(|_| LaunchError::VmcsHw)?;

    hw::vmwrite(VmcsField::GuestGdtrBase, img.guest_gdtr_base).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestGdtrLimit, u64::from(img.guest_gdtr_limit))
        .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestIdtrBase, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestIdtrLimit, 0).map_err(|_| LaunchError::VmcsHw)?;

    hw::vmwrite(VmcsField::GuestActivityState, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestInterruptibility, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestPendingDbg, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::VmcsLinkPointer, !0u64).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestIa32Efer, img.guest_efer).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestIa32Pat, read_pat()).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestSysenterCs, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestSysenterEsp, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestSysenterEip, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::GuestDr7, 0x400).map_err(|_| LaunchError::VmcsHw)?;
    Ok(())
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
unsafe fn write_host_state() -> Result<(), LaunchError> {
    use crate::vmx::vmcs::hw;

    let mut cr0: u64;
    let mut cr3: u64;
    let mut cr4: u64;
    core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nostack, preserves_flags));
    core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
    core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));

    hw::vmwrite(VmcsField::HostCr0, cr0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostCr3, cr3).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostCr4, cr4).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(
        VmcsField::HostRip,
        vmx_host_resume as *const () as usize as u64,
    )
    .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostRsp, host_stack_top()).map_err(|_| LaunchError::VmcsHw)?;

    hw::vmwrite(VmcsField::HostCsSelector, 0x08).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostSsSelector, 0x10).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostDsSelector, 0x10).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostEsSelector, 0x10).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostFsSelector, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostGsSelector, 0).map_err(|_| LaunchError::VmcsHw)?;
    // Host TR must be a valid TSS selector on the host GDT. The Multiboot
    // trampoline GDT has no TSS — load a temporary one before enter.
    ensure_host_tss();
    hw::vmwrite(VmcsField::HostTrSelector, 0x18).map_err(|_| LaunchError::VmcsHw)?;

    let (gdtr_base, idtr_base) = read_gdtr_idtr();
    hw::vmwrite(VmcsField::HostGdtrBase, gdtr_base).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostIdtrBase, idtr_base).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostTrBase, core::ptr::addr_of!(HOST_TSS) as u64)
        .map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostFsBase, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostGsBase, 0).map_err(|_| LaunchError::VmcsHw)?;

    hw::vmwrite(VmcsField::HostIa32Efer, rdmsr(0xC000_0080)).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostIa32Pat, read_pat()).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostSysenterCs, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostSysenterEsp, 0).map_err(|_| LaunchError::VmcsHw)?;
    hw::vmwrite(VmcsField::HostSysenterEip, 0).map_err(|_| LaunchError::VmcsHw)?;
    Ok(())
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
#[repr(C, align(16))]
struct Tss64 {
    data: [u8; 0x68],
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
static mut HOST_TSS: Tss64 = Tss64 { data: [0; 0x68] };

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
static mut HOST_GDT: [u64; 6] = [0; 6];

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
unsafe fn ensure_host_tss() {
    // Rebuild a host GDT with TSS at selector 0x18 so VMCS host TR is valid.
    let tss = core::ptr::addr_of!(HOST_TSS) as u64;
    HOST_GDT[0] = 0;
    HOST_GDT[1] = 0x00AF_9A00_0000_FFFF; // CS
    HOST_GDT[2] = 0x00CF_9200_0000_FFFF; // DS
    let lim = 0x67u64;
    let mut lo = 0u64;
    lo |= lim & 0xFFFF;
    lo |= (tss & 0xFFFFFF) << 16;
    lo |= 0x89u64 << 40; // available TSS64 + P
    lo |= ((tss >> 24) & 0xFF) << 56;
    HOST_GDT[3] = lo;
    HOST_GDT[4] = tss >> 32;
    HOST_GDT[5] = 0;

    let mut desc = [0u8; 10];
    let limit = (core::mem::size_of::<[u64; 6]>() - 1) as u16;
    desc[0..2].copy_from_slice(&limit.to_le_bytes());
    desc[2..10].copy_from_slice(&(core::ptr::addr_of!(HOST_GDT) as u64).to_le_bytes());
    core::arch::asm!("lgdt [{}]", in(reg) desc.as_ptr(), options(nostack, preserves_flags));
    core::arch::asm!("ltr {0:x}", in(reg) 0x18u16, options(nostack, preserves_flags));
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
fn host_stack_top() -> u64 {
    static mut HOST_STACK: [u8; 65536] = [0; 65536];
    unsafe {
        let base = core::ptr::addr_of_mut!(HOST_STACK) as u64;
        base + 65536
    }
}

/// Continuation address written by the launch trampoline; host RIP jumps here.
#[cfg(all(target_os = "none", target_arch = "x86_64"))]
static mut VMX_CONTINUE: u64 = 0;

/// Guest GPRs captured at VM-exit (VMCS does not save them).
#[cfg(all(target_os = "none", target_arch = "x86_64"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct GuestGprs {
    pub rax: u64,
    pub rdi: u64,
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
static mut LAST_GUEST_GPRS: GuestGprs = GuestGprs { rax: 0, rdi: 0 };

/// Read guest GPRs captured by the last VM-exit trampoline.
#[cfg(all(target_os = "none", target_arch = "x86_64"))]
pub fn last_guest_gprs() -> GuestGprs {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(LAST_GUEST_GPRS)) }
}

#[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
#[derive(Clone, Copy, Debug, Default)]
pub struct GuestGprs {
    pub rax: u64,
    pub rdi: u64,
}

#[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
pub fn last_guest_gprs() -> GuestGprs {
    GuestGprs::default()
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
#[no_mangle]
pub unsafe extern "C" fn vmx_host_resume() {
    // Guest GPRs are still live in the CPU registers at this point.
    let mut rax: u64;
    let mut rdi: u64;
    core::arch::asm!(
        "mov {a}, rax",
        "mov {d}, rdi",
        a = out(reg) rax,
        d = out(reg) rdi,
        options(nostack, preserves_flags),
    );
    core::ptr::write_volatile(
        core::ptr::addr_of_mut!(LAST_GUEST_GPRS),
        GuestGprs { rax, rdi },
    );
    let cont = core::ptr::read_volatile(core::ptr::addr_of!(VMX_CONTINUE));
    if cont != 0 {
        core::arch::asm!("jmp {}", in(reg) cont, options(noreturn));
    }
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
#[inline(never)]
unsafe fn vmx_launch_with_rdi(guest_rdi: u64, launched: bool) -> u8 {
    let mut fail: u8 = 1;
    let launched_u64 = u64::from(launched);
    core::arch::asm!(
        "lea {tmp}, [rip + 4f]",
        "mov qword ptr [rip + {slot}], {tmp}",
        "mov rdi, {gdi}",
        // Zero caller-saved GPRs we are allowed to clobber; leave rbx/rbp alone
        // (LLVM frame pointer / reserved).
        "xor rax, rax",
        "xor rcx, rcx",
        "xor rdx, rdx",
        "xor rsi, rsi",
        "xor r8, r8",
        "xor r9, r9",
        "xor r10, r10",
        "xor r11, r11",
        "test {launched}, {launched}",
        "jnz 2f",
        "vmlaunch",
        "jmp 3f",
        "2:",
        "vmresume",
        "jmp 3f",
        "4:",
        // VM-exit → host RIP → here
        "xor {fail}, {fail}",
        "jmp 5f",
        "3:",
        "setbe {fail}",
        "5:",
        gdi = in(reg) guest_rdi,
        launched = in(reg) launched_u64,
        tmp = out(reg) _,
        slot = sym VMX_CONTINUE,
        fail = inout(reg_byte) fail,
        out("rax") _,
        out("rcx") _,
        out("rdx") _,
        out("rsi") _,
        out("rdi") _,
        out("r8") _,
        out("r9") _,
        out("r10") _,
        out("r11") _,
        options(nostack),
    );
    fail
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
unsafe fn rdmsr(index: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    core::arch::asm!(
        "rdmsr",
        in("ecx") index,
        out("eax") lo,
        out("edx") hi,
        options(nostack, preserves_flags),
    );
    ((hi as u64) << 32) | (lo as u64)
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
unsafe fn read_pat() -> u64 {
    rdmsr(0x277)
}

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
unsafe fn read_gdtr_idtr() -> (u64, u64) {
    let mut gdtr = [0u8; 10];
    let mut idtr = [0u8; 10];
    core::arch::asm!("sgdt [{}]", in(reg) gdtr.as_mut_ptr(), options(nostack, preserves_flags));
    core::arch::asm!("sidt [{}]", in(reg) idtr.as_mut_ptr(), options(nostack, preserves_flags));
    let gbase = u64::from_le_bytes(gdtr[2..10].try_into().unwrap());
    let ibase = u64::from_le_bytes(idtr[2..10].try_into().unwrap());
    (gbase, ibase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmx::controls::{AdjustedControls, ControlCaps, ControlWants};

    fn permissive_controls() -> AdjustedControls {
        let wants = ControlWants::spike();
        let caps = ControlCaps {
            pin: u64::from(u32::MAX) << 32,
            primary: u64::from(u32::MAX) << 32,
            secondary: u64::from(u32::MAX) << 32,
            exit: u64::from(u32::MAX) << 32,
            entry: u64::from(u32::MAX) << 32,
        };
        AdjustedControls::from_wants(wants, caps)
    }

    #[test]
    fn prepare_rejects_zero_eptp() {
        let tips = LaunchTips {
            entry_rip: 0x203390,
            entry_rsp: 0x1010_0000,
            boot_info_gpa: 0x1200_0000,
            guest_cr3: 0x1000_0000,
            eptp: 0,
            guest_gdtr_base: 0x1000_1000,
        };
        assert_eq!(
            prepare_launch_image(tips, permissive_controls()),
            Err(LaunchError::MissingEptp)
        );
    }

    #[test]
    fn prepare_fills_rdi_with_boot_info() {
        let tips = LaunchTips {
            entry_rip: 0x203390,
            entry_rsp: 0x1010_0000,
            boot_info_gpa: 0x1200_0040,
            guest_cr3: 0x1000_0000,
            eptp: 0xABCD_001E,
            guest_gdtr_base: 0x1000_1000,
        };
        let img = prepare_launch_image(tips, permissive_controls()).unwrap();
        assert_eq!(img.entry_rdi, 0x1200_0040);
        assert_eq!(img.entry_rip, 0x203390);
        assert_ne!(img.controls.secondary & (1 << 1), 0);
        let pairs = launch_field_pairs(&img);
        assert!(pairs
            .iter()
            .any(|(f, v)| *f == VmcsField::EptPointer && *v == 0xABCD_001E));
        assert!(pairs
            .iter()
            .any(|(f, v)| *f == VmcsField::GuestRip && *v == 0x203390));
    }

    #[test]
    fn guest_gdt_has_code_data_tss() {
        let mut gdt = [0u8; 40];
        write_guest_gdt(&mut gdt, 0x1000_2000);
        assert_ne!(u64::from_le_bytes(gdt[8..16].try_into().unwrap()), 0);
        assert_ne!(u64::from_le_bytes(gdt[16..24].try_into().unwrap()), 0);
        assert_ne!(u64::from_le_bytes(gdt[24..32].try_into().unwrap()), 0);
    }
}
