//! Relay measurement extension in guest boot info (ABI v2).

use core::mem::{offset_of, size_of};

use hv_guest_abi::{
    guest_boot_info_relay_measurement_offset, parse_guest_boot_info_relay_measurement,
    GuestBootInfoRelayMeasurement, GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION,
    GUEST_RELAY_MEASUREMENT_MAGIC,
};

/// Initializes the relay measurement extension in guest boot info.
pub fn init_relay_measurement(boot_info: *const u8) {
    if boot_info.is_null() {
        return;
    }
    let Some(offset) = extension_offset(boot_info) else {
        return;
    };
    let extension = GuestBootInfoRelayMeasurement {
        magic: GUEST_RELAY_MEASUREMENT_MAGIC,
        version: GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION,
        frames_completed: 0,
        tsc_start: 0,
        tsc_end: 0,
    };
    // SAFETY: offset validated against boot info size and extension length.
    unsafe {
        core::ptr::write_unaligned(
            boot_info.add(offset) as *mut GuestBootInfoRelayMeasurement,
            extension,
        );
    }
}

/// Records TSC at the start of the sustained relay benchmark.
pub fn set_relay_measurement_tsc_start(boot_info: *const u8) {
    write_tsc_field(boot_info, offset_of!(GuestBootInfoRelayMeasurement, tsc_start));
}

/// Records TSC at the end of the sustained relay benchmark.
pub fn set_relay_measurement_tsc_end(boot_info: *const u8) {
    write_tsc_field(boot_info, offset_of!(GuestBootInfoRelayMeasurement, tsc_end));
}

/// Increments the out-partition end-to-end relay frame counter.
pub fn record_relay_frame_completed(boot_info: *const u8) {
    if boot_info.is_null() {
        return;
    }
    let Some(offset) = extension_offset(boot_info) else {
        return;
    };
    // SAFETY: offset validated against boot info size and extension length.
    unsafe {
        let frames_ptr = boot_info.add(offset + offset_of!(GuestBootInfoRelayMeasurement, frames_completed))
            as *mut u64;
        let current = core::ptr::read_unaligned(frames_ptr);
        core::ptr::write_unaligned(frames_ptr, current.saturating_add(1));
    }
}

fn write_tsc_field(boot_info: *const u8, field_offset: usize) {
    if boot_info.is_null() {
        return;
    }
    let Some(offset) = extension_offset(boot_info) else {
        return;
    };
    let tsc = read_tsc();
    // SAFETY: field offset lies inside the validated extension tail.
    unsafe {
        core::ptr::write_unaligned(
            boot_info.add(offset + field_offset) as *mut u64,
            tsc,
        );
    }
}

fn extension_offset(boot_info: *const u8) -> Option<usize> {
    // SAFETY: boot info pointer is valid for the guest lifetime when non-null.
    let header_size = unsafe { read_boot_info_size(boot_info) };
    guest_boot_info_relay_measurement_offset(header_size)
}

unsafe fn read_boot_info_size(boot_info: *const u8) -> u32 {
    let size_bytes = core::slice::from_raw_parts(boot_info.add(12), 4);
    u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]])
}

fn read_tsc() -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY: RDTSC is defined in ring 0/3 on x86_64.
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack, preserves_flags),
        );
    }
    u64::from(hi) << 32 | u64::from(lo)
}

#[allow(dead_code)]
fn validate_extension(bytes: &[u8]) -> bool {
    parse_guest_boot_info_relay_measurement(bytes).is_some()
}

const _: () = assert!(size_of::<GuestBootInfoRelayMeasurement>() == 32);
