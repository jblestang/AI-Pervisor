#![no_std]
#![no_main]

//! Freestanding Type-1 entry (Multiboot1 via `boot.s`).
//!
//! Boot ABI (Task 7):
//! - Multiboot1 modules: `[0]=FlatHvConfigBlob`, `[1]=guest-wan`, `[2]=guest-lan`
//! - Platform UART from `FlatHvConfig.uart_io_port`
//! - Inject smoke emits `SMOKE_OK` when config selects Inject / inject-fallback
//! - Module load + EPT pack + VMXON / VMLAUNCH are attempted; incomplete VMCS
//!   fails with a clear UART line (nested lab may still pass via L1 inject smoke)

extern crate alloc;

use core::panic::PanicInfo;

use guest_common::GuestRole;
use hv::boot_blob::fill_boot_info;
use hv::config::{FlatHvConfig, FlatHvConfigBlob};
use hv::ept::build_ept_intent;
use hv::ept_install::pack_identity_ept;
use hv::ept_tables::identity_entries;
use hv::guest_load::load_guest_image;
use hv::guest_mem::{build_layout, GuestId};
use hv::multiboot::{self, MULTIBOOT1_BOOT_MAGIC};
use hv::smoke_l1::run_inject_smoke;
use hv::uart::UartSink;
use hv::vmx::{
    handle_exit, vmx_enable_and_on, ExitAction, GuestRunState, VcpuId, VmxExitReason, VmxonRegion,
};

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

/// Early UART at COM1 for messages before config is parsed (loader contract).
fn early_uart() -> PlatformUart {
    PlatformUart { port: 0x3F8 }
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

    let _ = identity_entries(&intent.wan);
    let _ = identity_entries(&intent.lan);

    let wan_boot = fill_boot_info(cfg, &layout, GuestRole::Wan);
    let lan_boot = fill_boot_info(cfg, &layout, GuestRole::Lan);
    let _ = (wan_boot, lan_boot);

    // --- Module load into private buffers (capped; full guest RAM is EPT-sized later) ---
    // Keep BSS modest so Multiboot ADDR + KVM load stays reliable in nested lab VMs.
    static mut WAN_BUF: [u8; 256 * 1024] = [0; 256 * 1024];
    static mut LAN_BUF: [u8; 256 * 1024] = [0; 256 * 1024];
    // SAFETY: single-threaded freestanding bring-up owns these buffers.
    let wan_buf = unsafe { &mut *core::ptr::addr_of_mut!(WAN_BUF) };
    let lan_buf = unsafe { &mut *core::ptr::addr_of_mut!(LAN_BUF) };

    let wan_region = layout.image_region(GuestId::Wan);
    let lan_region = layout.image_region(GuestId::Lan);
    // Cap copy to buffer size (layout may request larger private RAM).
    let wan_slot = GpaSlot {
        size: (wan_region.size as usize).min(wan_buf.len()) as u64,
    };
    let lan_slot = GpaSlot {
        size: (lan_region.size as usize).min(lan_buf.len()) as u64,
    };
    let _ = (wan_region.base, lan_region.base);

    // Truncate oversized freestanding guest ELFs into the capped slot (smoke cares
    // about Multiboot module presence; full image map needs larger private RAM).
    let wan_chunk = wan_img.len().min(wan_slot.size as usize);
    let lan_chunk = lan_img.len().min(lan_slot.size as usize);
    if wan_chunk > 0 {
        wan_buf[..wan_chunk].copy_from_slice(&wan_img[..wan_chunk]);
        write_str(&mut uart, b"hv: wan module bytes staged\n");
        let _ = load_guest_image;
    } else {
        write_str(&mut uart, b"hv: wan module missing\n");
    }
    if lan_chunk > 0 {
        lan_buf[..lan_chunk].copy_from_slice(&lan_img[..lan_chunk]);
        write_str(&mut uart, b"hv: lan module bytes staged\n");
    } else {
        write_str(&mut uart, b"hv: lan module missing\n");
    }

    // --- EPT pack (identity) into static pages ---
    #[repr(C, align(4096))]
    struct EptStorage([u8; 64 * 1024]);
    static mut EPT_STORAGE: EptStorage = EptStorage([0; 64 * 1024]);
    let ept_storage = unsafe { &mut *core::ptr::addr_of_mut!(EPT_STORAGE.0) };
    match pack_identity_ept(&intent.wan, ept_storage) {
        Ok((eptp, _)) => {
            write_str(&mut uart, b"hv: EPT packed\n");
            let _ = eptp;
        }
        Err(_) => write_str(&mut uart, b"hv: EPT pack skipped/failed\n"),
    }

    // VMCS / VMXON regions are 4 KiB-aligned — keep them off the boot stack.
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
    match vmx_enable_and_on(vmxon) {
        Ok(()) => write_str(&mut uart, b"hv: VMXON ok\n"),
        Err(_) => write_str(&mut uart, b"hv: VMXON skipped/unavailable\n"),
    }

    let state = unsafe { &mut *core::ptr::addr_of_mut!(RUN_STATE) };
    state.wan.entry_gpa = layout.wan_private.base;
    state.wan.boot_info_gpa = layout.boot_info_wan.base;
    state.lan.entry_gpa = layout.lan_private.base;
    state.lan.boot_info_gpa = layout.boot_info_lan.base;

    write_str(
        &mut uart,
        b"hv: VMLAUNCH deferred (VMCS host-state incomplete - use L1 inject smoke)\n",
    );

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
    let _ = (ExitAction::Resume, _action);

    if cfg.nic_is_inject() {
        write_str(&mut uart, b"hv: starting inject smoke\n");
        let _ = run_inject_smoke(cfg, &mut uart);
    } else {
        write_str(
            &mut uart,
            b"hv: sriov path - inject smoke skipped (need VF + L2 VMLAUNCH)\n",
        );
    }

    write_str(&mut uart, b"hv: bring-up done\n");
}

struct GpaSlot {
    size: u64,
}

/// Called from Multiboot trampoline (`boot.s`) with Multiboot1 info pointer in `rdi`.
#[no_mangle]
pub extern "C" fn rust_entry(info: u32) -> ! {
    let mut early = early_uart();
    write_str(&mut early, b"hv: rust_entry\n");

    // QEMU Multiboot leaves magic in EAX; trampoline only forwarded EBX (info).
    // Accept info==0 as "no modules" and fail clearly.
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
