//! Relay measurement extension in guest boot info (ABI v2).
//!
//! When the hypervisor publishes a read-only measurement page GPA, guests record
//! counters and TSC in the boot-info tail; the hypervisor publishes authoritative
//! frame counts to the host page after execution.

use core::mem::{offset_of, size_of};

use hv_guest_abi::{
    guest_boot_info_relay_measurement_offset, parse_guest_boot_info_relay_measurement,
    GuestBootInfoRelayMeasurement, GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION,
    GUEST_RELAY_MEASUREMENT_MAGIC,
};

/// Initializes relay measurement state in the guest boot-info tail.
pub fn init_relay_measurement(boot_info: *const u8) {
    init_extension_tail(boot_info);
}

/// Records TSC at the start of the sustained relay benchmark.
pub fn set_relay_measurement_tsc_start(boot_info: *const u8) {
    write_tsc_field(boot_info, offset_of!(GuestBootInfoRelayMeasurement, tsc_start));
}

/// Records TSC at the end of the sustained relay benchmark.
pub fn set_relay_measurement_tsc_end(boot_info: *const u8) {
    write_tsc_field(boot_info, offset_of!(GuestBootInfoRelayMeasurement, tsc_end));
}

/// Increments the out-partition end-to-end relay frame counter in boot info.
pub fn record_relay_frame_completed(boot_info: *const u8) {
    record_relay_frame_in_boot_info_tail(boot_info);
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
        core::ptr::write_unaligned(boot_info.add(offset + field_offset) as *mut u64, tsc);
    }
}

fn init_extension_tail(boot_info: *const u8) {
    if boot_info.is_null() {
        return;
    }
    let measurement_page_gpa = read_measurement_page_gpa(boot_info).unwrap_or(0);
    let extension = GuestBootInfoRelayMeasurement {
        magic: GUEST_RELAY_MEASUREMENT_MAGIC,
        version: GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION,
        frames_completed: 0,
        tsc_start: 0,
        tsc_end: 0,
        measurement_page_gpa,
    };
    let Some(offset) = extension_offset(boot_info) else {
        return;
    };
    // SAFETY: offset validated against boot info size and extension length.
    unsafe {
        write_extension(boot_info.add(offset) as *mut u8, extension);
    }
}

fn record_relay_frame_in_boot_info_tail(boot_info: *const u8) {
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

fn read_measurement_page_gpa(boot_info: *const u8) -> Option<u64> {
    let header_size = unsafe { read_boot_info_size(boot_info) };
    // SAFETY: boot info pointer is valid for the guest lifetime when non-null.
    let bytes = unsafe { core::slice::from_raw_parts(boot_info, header_size as usize) };
    parse_guest_boot_info_relay_measurement(bytes)
        .map(|extension| extension.measurement_page_gpa)
        .filter(|gpa| *gpa != 0)
}

unsafe fn write_extension(base: *mut u8, extension: GuestBootInfoRelayMeasurement) {
    core::ptr::write_unaligned(base as *mut GuestBootInfoRelayMeasurement, extension);
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

const _: () = assert!(size_of::<GuestBootInfoRelayMeasurement>() == 40);
