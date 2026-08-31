#![no_std]
#![no_main]
#![allow(unsafe_code)]

use guest_common::{self, Role};

#[unsafe(no_mangle)]
pub extern "C" fn _start(boot_info: *const u8) -> ! {
    guest_common::run(Role::Mid, boot_info);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}
