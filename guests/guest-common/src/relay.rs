//! Relay-frame counter tail in guest boot info (Phase 29).

use hv_guest_abi::{guest_boot_info_relay_frames_offset, GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES};

/// Increments the relay-frame counter stored in the boot info measurement tail.
pub fn record_relay_frame_completed(boot_info: *const u8) {
    if boot_info.is_null() {
        return;
    }
    // SAFETY: boot info pointer is valid for the guest lifetime when non-null.
    let header_size = unsafe { read_boot_info_size(boot_info) };
    let Some(offset) = guest_boot_info_relay_frames_offset(header_size) else {
        return;
    };
    // SAFETY: offset validated against header size and tail length.
    unsafe {
        let counter_ptr = boot_info.add(offset) as *mut u64;
        let current = core::ptr::read_unaligned(counter_ptr);
        core::ptr::write_unaligned(counter_ptr, current.saturating_add(1));
    }
    let _ = GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES;
}

unsafe fn read_boot_info_size(boot_info: *const u8) -> u32 {
    let size_bytes = core::slice::from_raw_parts(boot_info.add(12), 4);
    u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]])
}
