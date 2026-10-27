#![no_std]
#![no_main]

//! WAN guest entry.
//!
//! Boot info GPA / UART port / ring tips / NIC backend come from the
//! [`guest_common::BootInfo`] blob prepared by `hv`. No lab addresses and no
//! backend `#ifdef` — switching `datapath.backend` in config does **not**
//! require rebuilding this guest.
//!
//! Entry ABI: `rdi` = `*const BootInfo` (VA == GPA under hv guest page tables).

extern crate alloc;

use core::panic::PanicInfo;

use guest_common::{
    forward_wan_poll, guest_entry_banner, uart_write, ArenaCursor, BootInfo, BumpAlloc, GuestRole,
    InjectNic, NicBackendKind, SpscRing, SriovNic, Stats,
};

#[global_allocator]
static ALLOC: BumpAlloc = BumpAlloc::new();

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// SAFETY: `gpa` must be mapped identity into this guest (hv guest PT).
unsafe fn slice_gpa<'a>(gpa: u64, len: usize) -> &'a mut [u8] {
    core::slice::from_raw_parts_mut(gpa as *mut u8, len)
}

fn open_nic(boot: &BootInfo) -> Result<NicKind, ()> {
    match boot.nic_backend {
        NicBackendKind::Inject => {
            let cap = boot.inject_queue_capacity.max(8) as usize;
            Ok(NicKind::Inject(InjectNic::with_capacity(cap)))
        }
        NicBackendKind::Sriov => {
            let ptr = if boot.nic_mmio_len == 0 {
                core::ptr::null_mut()
            } else {
                boot.nic_mmio_gpa as *mut u8
            };
            SriovNic::from_mmio_tips(boot.nic_mmio_gpa, boot.nic_mmio_len, ptr)
                .map(NicKind::Sriov)
                .map_err(|_| ())
        }
    }
}

enum NicKind {
    Inject(InjectNic),
    Sriov(SriovNic),
}

fn run_poll_loop(boot: &BootInfo) {
    let Ok(mut nic) = open_nic(boot) else {
        uart_write(boot, b"guest-wan: nic open failed (Sriov MMIO absent?)\n");
        return;
    };

    let cap = boot.ring_capacity_pow2.max(8);
    // SAFETY: hv mapped shared rings / arena / stats identity for this guest.
    let (wan_to_lan, lan_to_wan, arena, stats) = unsafe {
        let ring_span = 4096usize;
        let w2l = slice_gpa(boot.ring_wan_to_lan_gpa, ring_span);
        let l2w = slice_gpa(boot.ring_lan_to_wan_gpa, ring_span);
        let arena_len = core::cmp::min(boot.arena_len as usize, 65536);
        let arena = slice_gpa(boot.arena_gpa, arena_len);
        let stats_page = slice_gpa(boot.stats_gpa, core::mem::size_of::<Stats>());
        let stats = &*(stats_page.as_ptr() as *const Stats);
        let wan_to_lan = SpscRing::new_in(w2l, cap.min(128)).ok();
        let lan_to_wan = SpscRing::new_in(l2w, cap.min(128)).ok();
        (wan_to_lan, lan_to_wan, arena, stats)
    };
    let (Some(mut wan_to_lan), Some(mut lan_to_wan)) = (wan_to_lan, lan_to_wan) else {
        uart_write(boot, b"guest-wan: ring init failed\n");
        return;
    };

    let mut cursor = ArenaCursor::new();
    let mut scratch = [0u8; 2048];
    uart_write(boot, b"guest-wan: poll live\n");

    // Bounded polls so a nested smoke can observe UART then HLT.
    for _ in 0..32 {
        match &mut nic {
            NicKind::Inject(n) => {
                let _ = forward_wan_poll(
                    n,
                    &mut wan_to_lan,
                    &mut lan_to_wan,
                    arena,
                    &mut cursor,
                    &mut scratch,
                    stats,
                );
            }
            NicKind::Sriov(n) => {
                let _ = forward_wan_poll(
                    n,
                    &mut wan_to_lan,
                    &mut lan_to_wan,
                    arena,
                    &mut cursor,
                    &mut scratch,
                    stats,
                );
            }
        }
    }
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
    run_poll_loop(boot);

    // HLT → exit to hv (halted).
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
