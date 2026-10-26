#![no_std]
#![no_main]

//! WAN guest entry (Task 5).
//!
//! Boot info GPA / UART port / ring tips come from the [`guest_common::BootInfo`]
//! blob prepared by `hv` (placeholders until Task 6). No lab addresses are
//! hardcoded here.
//!
//! Entry ABI (spike): first argument is a pointer to [`BootInfo`] (SysV: `rdi`).
//! When null or invalid, the guest halts.
//!
//! Forward datapath ([`guest_common::forward_wan_poll`]) is the same code
//! covered by host tests with real ring + Inject; hv maps rings/NIC in Task 6
//! before the poll loop can run against live GPAs.

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
        // hv not yet wired (Task 6) — idle.
        loop {}
    }

    // SAFETY: hv must pass a valid BootInfo mapping before entry.
    let boot = unsafe { &*boot_info };
    if !boot.is_valid() || boot.guest_role != GuestRole::Wan {
        loop {}
    }

    let _ = guest_entry_banner(boot);

    // Keep the WAN forward monomorphization in the freestanding image (same
    // function as host TEST-UNIT-FWD). Live poll waits for hv-filled GPAs.
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
