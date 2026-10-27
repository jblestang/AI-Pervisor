#![no_std]
#![no_main]

//! Freestanding Type-1 entry (Multiboot1 via `boot.s`).
//!
//! Boot ABI:
//! - Multiboot1 modules: `[0]=FlatHvConfigBlob`, `[1]=guest-wan`, `[2]=guest-lan`
//! - Platform UART from `FlatHvConfig.uart_io_port`
//! - L2: place guests at layout GPAs, pack EPT + guest PT, VMLAUNCH when VMXON works
//! - Inject smoke (L1) still emits `SMOKE_OK` when resolved backend is Inject

extern crate alloc;

use core::panic::PanicInfo;

use guest_common::GuestRole;
use hv::boot_blob::fill_boot_info;
use hv::config::{FlatHvConfig, FlatHvConfigBlob};
use hv::ept::build_ept_intent;
use hv::ept_install::pack_identity_ept;
use hv::guest_mem::{build_layout, GuestId};
use hv::l2::{
    identity_tips, install_boot_info, load_into_private, prepare_guest, permissive_controls,
};
use hv::multiboot::{self, MULTIBOOT1_BOOT_MAGIC};
use hv::smoke_l1::run_inject_smoke;
use hv::uart::UartSink;
use hv::vmx::controls::{AdjustedControls, ControlCaps, ControlWants};
use hv::vmx::enter::enter_guest;
use hv::vmx::{
    handle_exit, last_guest_gprs, vmx_enable_and_on, ExitAction, GuestRunState, VcpuId,
    VmxExitReason, VmxonRegion,
};
use hv::PAGE_SIZE;

#[global_allocator]
static ALLOC: guest_common::BumpAlloc = guest_common::BumpAlloc::new();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

fn platform_uart_write(port: u16, byte: u8) {
    #[cfg(all(target_os = "none", target_arch = "x86_64"))]
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") byte,
            options(nostack, preserves_flags),
        );
    }
    let _ = (port, byte);
}

struct PlatformUart {
    port: u16,
}

impl UartSink for PlatformUart {
    fn write_byte(&mut self, byte: u8) {
        platform_uart_write(self.port, byte);
    }
}

fn write_str<S: UartSink>(sink: &mut S, s: &[u8]) {
    for &b in s {
        sink.write_byte(b);
    }
}

fn early_uart() -> PlatformUart {
    PlatformUart { port: 0x3F8 }
}

/// SAFETY: `gpa` must be identity-mapped L1 RAM owned by bring-up.
unsafe fn slice_at(gpa: u64, len: usize) -> &'static mut [u8] {
    core::slice::from_raw_parts_mut(gpa as *mut u8, len)
}

fn read_control_caps() -> Option<ControlCaps> {
    #[cfg(all(target_os = "none", target_arch = "x86_64"))]
    {
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
        // Prefer true/true MSRs when bit 55 of VMX_BASIC is set.
        unsafe {
            let basic = rdmsr(0x480);
            let true_ctls = (basic & (1u64 << 55)) != 0;
            let pin = rdmsr(if true_ctls { 0x48D } else { 0x481 });
            let primary = rdmsr(if true_ctls { 0x48E } else { 0x482 });
            let exit = rdmsr(if true_ctls { 0x48F } else { 0x483 });
            let entry = rdmsr(if true_ctls { 0x490 } else { 0x484 });
            let secondary = rdmsr(0x48B);
            Some(ControlCaps {
                pin,
                primary,
                secondary,
                exit,
                entry,
            })
        }
    }
    #[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
    {
        None
    }
}

fn adjusted_controls() -> AdjustedControls {
    match read_control_caps() {
        Some(caps) => AdjustedControls::from_wants(ControlWants::spike(), caps),
        None => permissive_controls(),
    }
}

fn bring_up(cfg: &FlatHvConfig, wan_img: &[u8], lan_img: &[u8]) {
    let mut uart = PlatformUart {
        port: cfg.uart_io_port,
    };
    write_str(&mut uart, b"hv: bring-up\n");

    let layout = match build_layout(cfg) {
        Ok(l) => l,
        Err(_) => {
            write_str(&mut uart, b"hv: layout failed\n");
            return;
        }
    };
    let intent = match build_ept_intent(&layout) {
        Ok(i) => i,
        Err(_) => {
            write_str(&mut uart, b"hv: ept intent failed\n");
            return;
        }
    };

    let wan_boot = fill_boot_info(cfg, &layout, GuestRole::Wan);
    let lan_boot = fill_boot_info(cfg, &layout, GuestRole::Lan);

    // --- Place BootInfo + zero shared pages at layout GPAs (identity map) ---
    // SAFETY: gpa_window is inside the Multiboot trampoline's 1 GiB identity map.
    unsafe {
        let boot_w = slice_at(layout.boot_info_wan.base, PAGE_SIZE as usize);
        let boot_l = slice_at(layout.boot_info_lan.base, PAGE_SIZE as usize);
        let _ = install_boot_info(boot_w, &wan_boot);
        let _ = install_boot_info(boot_l, &lan_boot);
        slice_at(layout.ring_wan_to_lan.base, layout.ring_wan_to_lan.size as usize).fill(0);
        slice_at(layout.ring_lan_to_wan.base, layout.ring_lan_to_wan.size as usize).fill(0);
        slice_at(layout.stats.base, PAGE_SIZE as usize).fill(0);
        let arena_len = core::cmp::min(layout.arena.size as usize, 64 * 1024);
        slice_at(layout.arena.base, arena_len).fill(0);
    }
    write_str(&mut uart, b"hv: bootinfo installed\n");

    // --- Load guest images into private GPA windows ---
    let wan_region = layout.image_region(GuestId::Wan);
    let lan_region = layout.image_region(GuestId::Lan);
    let wan_elf = unsafe {
        let dest = slice_at(wan_region.base, wan_region.size as usize);
        dest.fill(0);
        load_into_private(dest, wan_region, wan_img)
    };
    let lan_elf = unsafe {
        let dest = slice_at(lan_region.base, lan_region.size as usize);
        dest.fill(0);
        load_into_private(dest, lan_region, lan_img)
    };
    match &wan_elf {
        Ok(_) => write_str(&mut uart, b"hv: wan image loaded\n"),
        Err(_) => write_str(&mut uart, b"hv: wan image load failed\n"),
    }
    match &lan_elf {
        Ok(_) => write_str(&mut uart, b"hv: lan image loaded\n"),
        Err(_) => write_str(&mut uart, b"hv: lan image load failed\n"),
    }

    // --- EPT pack per guest ---
    #[repr(C, align(4096))]
    struct EptStorage([u8; 64 * 1024]);
    static mut EPT_WAN: EptStorage = EptStorage([0; 64 * 1024]);
    static mut EPT_LAN: EptStorage = EptStorage([0; 64 * 1024]);
    let ept_wan = unsafe { &mut *core::ptr::addr_of_mut!(EPT_WAN.0) };
    let ept_lan = unsafe { &mut *core::ptr::addr_of_mut!(EPT_LAN.0) };
    let eptp_wan = match pack_identity_ept(&intent.wan, ept_wan) {
        Ok((eptp, _)) => {
            write_str(&mut uart, b"hv: EPT wan packed\n");
            Some(eptp)
        }
        Err(_) => {
            write_str(&mut uart, b"hv: EPT wan pack failed\n");
            None
        }
    };
    let eptp_lan = match pack_identity_ept(&intent.lan, ept_lan) {
        Ok((eptp, _)) => {
            write_str(&mut uart, b"hv: EPT lan packed\n");
            Some(eptp)
        }
        Err(_) => {
            write_str(&mut uart, b"hv: EPT lan pack failed\n");
            None
        }
    };

    // --- VMXON ---
    static mut VMXON: VmxonRegion = VmxonRegion { data: [0; 4096] };
    static mut RUN_STATE: GuestRunState = GuestRunState {
        wan: hv::vmx::Vcpu {
            id: VcpuId::Wan,
            vmcs: hv::vmx::vmcs::VmcsRegion { data: [0; 4096] },
            halted: false,
            entry_gpa: 0,
            boot_info_gpa: 0,
            ept_pointer: 0,
        },
        lan: hv::vmx::Vcpu {
            id: VcpuId::Lan,
            vmcs: hv::vmx::vmcs::VmcsRegion { data: [0; 4096] },
            halted: false,
            entry_gpa: 0,
            boot_info_gpa: 0,
            ept_pointer: 0,
        },
    };
    let vmxon = unsafe { &mut *core::ptr::addr_of_mut!(VMXON) };
    let vmx_on = match vmx_enable_and_on(vmxon) {
        Ok(()) => {
            write_str(&mut uart, b"hv: VMXON ok\n");
            true
        }
        Err(_) => {
            write_str(&mut uart, b"hv: VMXON skipped/unavailable\n");
            false
        }
    };

    let state = unsafe { &mut *core::ptr::addr_of_mut!(RUN_STATE) };
    state.wan.entry_gpa = layout.wan_private.base;
    state.wan.boot_info_gpa = layout.boot_info_wan.base;
    state.lan.entry_gpa = layout.lan_private.base;
    state.lan.boot_info_gpa = layout.boot_info_lan.base;
    if let Some(e) = eptp_wan {
        state.wan.ept_pointer = e;
    }
    if let Some(e) = eptp_lan {
        state.lan.ept_pointer = e;
    }

    let controls = adjusted_controls();
    let mut l2_ran = false;

    if vmx_on {
        l2_ran = run_l2_guests(
            cfg,
            &layout,
            &intent,
            state,
            &wan_elf,
            &lan_elf,
            eptp_wan,
            eptp_lan,
            controls,
            &mut uart,
        );
    } else {
        write_str(
            &mut uart,
            b"hv: VMLAUNCH skipped (no VMXON - need nested VT-x / HV_QEMU_ACCEL=kvm)\n",
        );
    }

    // Host-testable exit policy still exercised once for EPT fault demo on L1.
    let _action = handle_exit(
        cfg,
        &intent,
        state,
        VcpuId::Wan,
        VmxExitReason::EptViolation,
        0,
        layout.lan_private.base,
        &mut uart,
    );
    let _ = (ExitAction::Resume, _action, l2_ran, identity_tips);

    if cfg.nic_is_inject() {
        write_str(&mut uart, b"hv: starting inject smoke\n");
        let _ = run_inject_smoke(cfg, &mut uart);
    } else {
        write_str(
            &mut uart,
            b"hv: sriov path - L1 inject smoke skipped (need VF BAR + guest Sriov rings)\n",
        );
    }

    write_str(&mut uart, b"hv: bring-up done\n");
}

fn run_l2_guests<S: UartSink>(
    cfg: &FlatHvConfig,
    layout: &hv::guest_mem::GuestLayout,
    intent: &hv::ept::EptIntent,
    state: &mut GuestRunState,
    wan_elf: &Result<hv::guest_load::ElfLoad, hv::l2::PrepareError>,
    lan_elf: &Result<hv::guest_load::ElfLoad, hv::l2::PrepareError>,
    eptp_wan: Option<u64>,
    eptp_lan: Option<u64>,
    controls: AdjustedControls,
    uart: &mut S,
) -> bool {
    let mut any = false;
    for (role, elf, eptp, vcpu_id) in [
        (GuestRole::Wan, wan_elf, eptp_wan, VcpuId::Wan),
        (GuestRole::Lan, lan_elf, eptp_lan, VcpuId::Lan),
    ] {
        let (Ok(elf), Some(eptp)) = (elf, eptp) else {
            write_str(uart, b"hv: L2 skip guest (load/EPT)\n");
            continue;
        };
        let private = match role {
            GuestRole::Wan => layout.wan_private,
            GuestRole::Lan => layout.lan_private,
        };
        // PT + GDT in the last 4 pages of private RAM (identity GPA).
        let pt_gpa = private.base + private.size - 4 * PAGE_SIZE;
        let gdt_gpa = private.base + private.size - PAGE_SIZE;
        let prep = unsafe {
            let pt = slice_at(pt_gpa, (3 * PAGE_SIZE) as usize);
            let gdt = slice_at(gdt_gpa, PAGE_SIZE as usize);
            gdt.fill(0);
            prepare_guest(cfg, layout, role, elf, eptp, pt, gdt, gdt_gpa, controls)
        };
        let Ok(prep) = prep else {
            write_str(uart, b"hv: L2 prepare failed\n");
            continue;
        };

        write_str(uart, b"hv: VMLAUNCH ");
        write_str(
            uart,
            match role {
                GuestRole::Wan => b"wan",
                GuestRole::Lan => b"lan",
            },
        );
        write_str(uart, b"\n");

        state.vcpu_mut(vcpu_id).ept_pointer = eptp;
        let mut launched = false;
        // Bounded exit loop — enough for guest banners + a few UART outs.
        for _ in 0..64 {
            if state.vcpu_mut(vcpu_id).halted {
                break;
            }
            let enter_result = {
                let vcpu = state.vcpu_mut(vcpu_id);
                unsafe { enter_guest(vcpu, &prep.launch, launched) }
            };
            match enter_result {
                Ok(reason_raw) => {
                    launched = true;
                    any = true;
                    let reason = VmxExitReason::from_raw(reason_raw);
                    let qual = unsafe { vmread_field(hv::vmx::VmcsField::VmExitQualification) };
                    let gpa = unsafe { vmread_field(hv::vmx::VmcsField::GuestPhysicalAddress) };
                    let gprs = last_guest_gprs();
                    let data_or_gpa = match reason {
                        VmxExitReason::IoInstruction => gprs.rax & 0xff,
                        _ => gpa,
                    };
                    let action = handle_exit(
                        cfg,
                        intent,
                        state,
                        vcpu_id,
                        reason,
                        qual,
                        data_or_gpa,
                        uart,
                    );
                    if action != ExitAction::Resume {
                        break;
                    }
                    // Advance guest RIP after I/O by VM-exit instruction length.
                    if matches!(reason, VmxExitReason::IoInstruction) {
                        let len = unsafe { vmread_raw(0x440C) };
                        let rip = unsafe { vmread_field(hv::vmx::VmcsField::GuestRip) };
                        let _ = unsafe {
                            vmwrite_field(hv::vmx::VmcsField::GuestRip, rip.wrapping_add(len))
                        };
                    }
                }
                Err(_) => {
                    write_str(uart, b"hv: VMLAUNCH/VMRESUME failed\n");
                    let err = unsafe { vmread_field(hv::vmx::VmcsField::VmInstructionError) };
                    write_str(uart, b"hv: VM instr err=");
                    write_hex_u64(uart, err);
                    write_str(uart, b"\n");
                    break;
                }
            }
        }
    }
    if any {
        write_str(uart, b"hv: L2 enter attempted (see guest UART above)\n");
    }
    any
}

unsafe fn vmread_field(field: hv::vmx::VmcsField) -> u64 {
    #[cfg(all(target_os = "none", target_arch = "x86_64"))]
    {
        hv::vmx::vmcs::hw::vmread(field).unwrap_or(0)
    }
    #[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
    {
        let _ = field;
        0
    }
}

unsafe fn vmwrite_field(field: hv::vmx::VmcsField, value: u64) -> Result<(), ()> {
    #[cfg(all(target_os = "none", target_arch = "x86_64"))]
    {
        hv::vmx::vmcs::hw::vmwrite(field, value)
    }
    #[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
    {
        let _ = (field, value);
        Err(())
    }
}

unsafe fn vmread_raw(field: u64) -> u64 {
    #[cfg(all(target_os = "none", target_arch = "x86_64"))]
    {
        let mut value: u64;
        let mut fail: u8;
        core::arch::asm!(
            "vmread {val}, {field}",
            "setbe {fail}",
            val = out(reg) value,
            field = in(reg) field,
            fail = out(reg_byte) fail,
            options(nostack),
        );
        if fail == 0 {
            value
        } else {
            0
        }
    }
    #[cfg(not(all(target_os = "none", target_arch = "x86_64")))]
    {
        let _ = field;
        0
    }
}

fn write_hex_u64<S: UartSink>(sink: &mut S, mut v: u64) {
    sink.write_byte(b'0');
    sink.write_byte(b'x');
    let mut buf = [0u8; 16];
    for i in (0..16).rev() {
        let n = (v & 0xf) as u8;
        buf[i] = if n < 10 { b'0' + n } else { b'a' + (n - 10) };
        v >>= 4;
    }
    for &b in &buf {
        sink.write_byte(b);
    }
}

/// Called from Multiboot trampoline (`boot.s`) with Multiboot1 info pointer in `rdi`.
#[no_mangle]
pub extern "C" fn rust_entry(info: u32) -> ! {
    let mut early = early_uart();
    write_str(&mut early, b"hv: rust_entry\n");

    let mods: alloc::vec::Vec<_> = multiboot::modules(info).collect();
    if mods.is_empty() {
        write_str(
            &mut early,
            b"hv: ERROR no Multiboot modules (need FlatHvConfig + guest images)\n",
        );
        write_str(
            &mut early,
            b"hv: see docs/HARNESS.md - xtask run-nested passes -initrd blob,wan,lan\n",
        );
        loop {
            core::hint::spin_loop();
        }
    }

    let cfg_bytes = mods[0].bytes();
    let Some(cfg) = FlatHvConfigBlob::parse(cfg_bytes) else {
        write_str(&mut early, b"hv: ERROR bad FlatHvConfigBlob (module 0)\n");
        loop {
            core::hint::spin_loop();
        }
    };
    let _ = MULTIBOOT1_BOOT_MAGIC;

    let wan_img = if mods.len() > 1 {
        mods[1].bytes()
    } else {
        &[]
    };
    let lan_img = if mods.len() > 2 {
        mods[2].bytes()
    } else {
        &[]
    };

    bring_up(&cfg, wan_img, lan_img);

    loop {
        core::hint::spin_loop();
    }
}
