#![no_std]
#![no_main]

//! Freestanding Type-1 entry.
//!
//! Boot ABI (spike / Task 7): `rdi` = `*const FlatHvConfig` prepared by the
//! nested loader from `DemoConfig`. Guest image bytes are supplied via a
//! platform-specific module list (documented in `docs/HARNESS.md`); until the
//! harness wires modules, this entry validates config + builds EPT intent and
//! attempts VMXON when nested VT-x is present.

extern crate alloc;

use core::panic::PanicInfo;

use guest_common::GuestRole;
use hv::boot_blob::fill_boot_info;
use hv::config::FlatHvConfig;
use hv::ept::build_ept_intent;
use hv::ept_tables::identity_entries;
use hv::guest_mem::build_layout;
use hv::uart::UartSink;
use hv::vmx::{
    handle_exit, vmx_enable_and_on, ExitAction, GuestRunState, VcpuId, VmxExitReason, VmxonRegion,
};

/// Bump allocator for freestanding `alloc` (EPT entry vecs during bring-up).
#[global_allocator]
static ALLOC: guest_common::BumpAlloc = guest_common::BumpAlloc::new();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// Platform UART byte out — TCB when backed by real serial I/O.
fn platform_uart_write(port: u16, byte: u8) {
    // SAFETY: I/O port from FlatHvConfig (`hv.uart_io_port`); nested QEMU maps L1 serial.
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

/// Bring-up sequence: layout → EPT intent → VMXON (when available) → dual vCPU state.
fn bring_up(cfg: &FlatHvConfig) -> Result<(), ()> {
    let mut uart = PlatformUart {
        port: cfg.uart_io_port,
    };
    write_str(&mut uart, b"hv: bring-up\n");

    let layout = build_layout(cfg).map_err(|_| ())?;
    let intent = build_ept_intent(&layout).map_err(|_| ())?;

    // Prove identity EPT leaves can be derived for both guests (Task 7 loads HPA).
    let _wan_pt = identity_entries(&intent.wan).map_err(|_| ())?;
    let _lan_pt = identity_entries(&intent.lan).map_err(|_| ())?;

    let wan_boot = fill_boot_info(cfg, &layout, GuestRole::Wan);
    let lan_boot = fill_boot_info(cfg, &layout, GuestRole::Lan);
    let _ = (wan_boot, lan_boot);

    let mut vmxon = VmxonRegion::zeroed();
    match vmx_enable_and_on(&mut vmxon) {
        Ok(()) => write_str(&mut uart, b"hv: VMXON ok\n"),
        Err(_) => {
            // Nested VT-x may be absent in CI images — layout/EPT still constructed.
            write_str(&mut uart, b"hv: VMXON skipped/unavailable\n");
            return Ok(());
        }
    }

    let mut state = GuestRunState::new();
    state.wan.entry_gpa = layout.wan_private.base;
    state.wan.boot_info_gpa = layout.boot_info_wan.base;
    state.lan.entry_gpa = layout.lan_private.base;
    state.lan.boot_info_gpa = layout.boot_info_lan.base;

    // Full VMLAUNCH + module load is wired by Task 7 harness. Link the exit
    // policy once (cross-guest GPA → halt WAN only).
    let _action = handle_exit(
        cfg,
        &intent,
        &mut state,
        VcpuId::Wan,
        VmxExitReason::EptViolation,
        0,
        layout.lan_private.base,
        &mut uart,
    );
    debug_assert!(matches!(_action, ExitAction::Halted));
    let _ = ExitAction::Resume;

    write_str(&mut uart, b"hv: guests enterable (await harness modules)\n");
    Ok(())
}

#[no_mangle]
pub extern "C" fn _start(cfg: *const FlatHvConfig) -> ! {
    if cfg.is_null() {
        // Loader not wired yet (Task 7) — idle without fabricating lab config.
        loop {
            core::hint::spin_loop();
        }
    }

    // SAFETY: Task 7 loader passes a valid FlatHvConfig mapping for the L1 life.
    let cfg = unsafe { &*cfg };
    let _ = bring_up(cfg);

    loop {
        core::hint::spin_loop();
    }
}
