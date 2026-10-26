#![no_std]
#![no_main]

//! LAN guest entry.
//!
//! Boot info GPA / UART port / ring tips come from the [`guest_common::BootInfo`]
//! blob prepared by `hv`. No lab addresses are hardcoded here.
//!
//! Entry ABI: `rdi` = `*const BootInfo`. Emits `SMOKE_OK` (`IF-SER-001`) after the
//! LAN forward path advances TX/fwd counters (rings/NIC mapped by hv). Until L2
//! VMLAUNCH is complete, nested smoke uses the L1 inject path in `hv`.

extern crate alloc;

use core::panic::PanicInfo;

use guest_common::{
    emit_smoke_ok, forward_lan_poll, guest_entry_banner, ArenaCursor, BootInfo, BumpAlloc,
    GuestRole, InjectNic, SpscRing, Stats,
};

#[global_allocator]
static ALLOC: BumpAlloc = BumpAlloc::new();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
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

    // Keep LAN forward + smoke emit in the freestanding image (same symbols as
    // host tests / L1 smoke). Live GPA poll lands with complete VMLAUNCH.
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
    let _emit: fn(&BootInfo) = emit_smoke_ok;
    let _ = _emit;

    loop {
        core::hint::spin_loop();
    }
}
