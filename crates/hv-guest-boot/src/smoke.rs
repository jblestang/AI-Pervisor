//! Minimal smoke guest image for single-partition VMX launch bring-up.

/// Serial marker emitted when the smoke guest runs (COM1 write in image).
pub const GUEST_SMOKE_RUNNING_MARKER: &str = "GUEST: smoke partition running";

/// Minimal freestanding guest code: write marker prefix to COM1 and halt.
///
/// The image is placed at the guest entry physical address from static layout planning.
pub const GUEST_SMOKE_IMAGE: &[u8] = &[
    0x48, 0xC7, 0xC0, 0x47, 0x00, 0x00, 0x00, // mov rax, 'G'
    0x48, 0xC7, 0xC2, 0xF8, 0x03, 0x00, 0x00, // mov rdx, 0x3F8
    0xEE, // out dx, al
    0x48, 0xC7, 0xC0, 0x0A, 0x00, 0x00, 0x00, // mov rax, '\n'
    0xEE, // out dx, al
    0xF4, // hlt
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_smoke_image_is_small() {
        assert!(GUEST_SMOKE_IMAGE.len() <= 64);
    }
}
