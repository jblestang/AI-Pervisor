//! Relay measurement extension in guest boot info (ABI v2).
//!
//! Guests signal completed frames via writes to the read-only measurement page GPA,
//! which trap as EPT violations so the hypervisor increments authoritative counters.

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

/// Signals one completed relay frame; the hypervisor counts the EPT violation.
pub fn record_relay_frame_completed(boot_info: *const u8) {
    signal_relay_frame_via_measurement_page_write(boot_info);
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

fn signal_relay_frame_via_measurement_page_write(boot_info: *const u8) {
    if boot_info.is_null() {
        return;
    }
    let Some(measurement_page_gpa) = read_measurement_page_gpa(boot_info) else {
        return;
    };
    let frames_offset = offset_of!(GuestBootInfoRelayMeasurement, frames_completed) as u64;
    let signal_address = measurement_page_gpa.saturating_add(frames_offset) as *mut u64;
    // SAFETY: write to read-only measurement page GPA traps to the hypervisor VM-exit handler.
    unsafe {
        core::ptr::write_volatile(signal_address, 0);
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

const _: () = assert!(size_of::<GuestBootInfoRelayMeasurement>() == 40);
