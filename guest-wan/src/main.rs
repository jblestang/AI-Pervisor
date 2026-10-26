#![no_std]
#![no_main]

//! WAN guest entry.
//!
//! Boot info GPA / UART port / ring tips come from the [`guest_common::BootInfo`]
//! blob prepared by `hv`. No lab addresses are hardcoded here.
//!
//! Entry ABI: `rdi` = `*const BootInfo`.

extern crate alloc;

use core::panic::PanicInfo;

use guest_common::{
    forward_wan_poll, guest_entry_banner, ArenaCursor, BootInfo, BumpAlloc, GuestRole, InjectNic,
    SpscRing, Stats,
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
    if !boot.is_valid() || boot.guest_role != GuestRole::Wan {
        loop {}
    }

    let _ = guest_entry_banner(boot);

    let _fwd: fn(
        &mut InjectNic,
        &mut SpscRing<'_>,
        &mut SpscRing<'_>,
        &mut [u8],
        &mut ArenaCursor,
        &mut [u8],
        &Stats,
    ) -> usize = forward_wan_poll;
    let _ = _fwd;

    loop {
        core::hint::spin_loop();
    }
}
