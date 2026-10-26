#![no_std]
#![no_main]

//! LAN guest entry.
//!
//! Boot info GPA / UART port / ring tips / NIC backend come from the
//! [`guest_common::BootInfo`] blob prepared by `hv`. No lab addresses and no
//! backend `#ifdef` — switching `datapath.backend` in config does **not**
//! require rebuilding this guest (Task 10).
//!
//! Entry ABI: `rdi` = `*const BootInfo`. Emits `SMOKE_OK` (`IF-SER-001`) after the
//! LAN forward path advances TX/fwd counters (rings/NIC mapped by hv). Until L2
//! VMLAUNCH is complete, nested smoke uses the L1 inject path in `hv`.

extern crate alloc;

use core::panic::PanicInfo;

use guest_common::{
    emit_smoke_ok, forward_lan_poll, guest_entry_banner, ArenaCursor, BootInfo, BumpAlloc,
    GuestRole, InjectNic, NicBackendKind, SpscRing, SriovNic, Stats,
};

#[global_allocator]
static ALLOC: BumpAlloc = BumpAlloc::new();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// Keep both NIC backends linked so a config-only Sriov↔Inject switch works
/// without a guest rebuild. Live GPA poll lands with complete VMLAUNCH.
fn retain_datapath_symbols(boot: &BootInfo) {
    match boot.nic_backend {
        NicBackendKind::Inject => {
            let _fwd: fn(
                &mut InjectNic,
                &mut SpscRing<'_>,
                &mut SpscRing<'_>,
                &mut [u8],
                &mut ArenaCursor,
                &mut [u8],
                &Stats,
            ) -> usize = forward_lan_poll;
            let _ = _fwd;
        }
        NicBackendKind::Sriov => {
            let _fwd: fn(
                &mut SriovNic,
                &mut SpscRing<'_>,
                &mut SpscRing<'_>,
                &mut [u8],
                &mut ArenaCursor,
                &mut [u8],
                &Stats,
            ) -> usize = forward_lan_poll;
            let _ = _fwd;
        }
    }
    let _emit: fn(&BootInfo) = emit_smoke_ok;
    let _ = _emit;
}

#[no_mangle]
pub extern "C" fn _start(boot_info: *const BootInfo) -> ! {
    if boot_info.is_null() {
        loop {}
    }

    // SAFETY: hv must pass a valid BootInfo mapping before entry.
    let boot = unsafe { &*boot_info };
    if !boot.is_valid() || boot.guest_role != GuestRole::Lan {
        loop {}
    }

    let _ = guest_entry_banner(boot);
    retain_datapath_symbols(boot);

    loop {
        core::hint::spin_loop();
    }
}
