//! COM1 serial output for guest markers.

const COM1: u16 = 0x03F8;

/// Writes one byte to COM1.
pub fn write_byte(byte: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") COM1,
            in("al") byte,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Writes an ASCII string to COM1.
pub fn write_str(text: &str) {
    for byte in text.as_bytes() {
        write_byte(*byte);
    }
}

/// Writes a string followed by newline.
pub fn write_line(text: &str) {
    write_str(text);
    write_byte(b'\n');
}
