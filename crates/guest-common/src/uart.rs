//! Guest UART output via I/O port from [`crate::BootInfo`] (`IF-UART-001`).
//!
//! Port value comes from the boot blob (filled by hv). No lab port constants.

use crate::boot_info::BootInfo;

/// Write one byte to the UART I/O port advertised in `boot`.
///
/// When `uart_port == 0` (placeholder before hv fill), this is a no-op so
/// freestanding guests can link before Task 6 wires exits.
#[inline]
pub fn uart_write_byte(boot: &BootInfo, byte: u8) {
    let port = boot.uart_port;
    if port == 0 {
        return;
    }
    // SAFETY: hv emulates this port exit; port comes from boot info, not a lab literal.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") byte,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Write a byte string to UART.
pub fn uart_write(boot: &BootInfo, bytes: &[u8]) {
    for &b in bytes {
        uart_write_byte(boot, b);
    }
}
