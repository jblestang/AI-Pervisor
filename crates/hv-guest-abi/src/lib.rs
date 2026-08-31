//! Versioned guest boot ABI shared by the hypervisor and guest partitions.
//!
//! Guests discover their resources exclusively through the boot info blob.

#![no_std]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

use hv_types::{GuestPhysAddr, VcpuId, VmId};

/// Current guest ABI version.
pub const GUEST_ABI_VERSION: u32 = 2;

/// Minimum supported guest ABI version.
pub const GUEST_ABI_VERSION_MIN: u32 = 1;

/// Magic bytes for `GuestBootInfoHeader`.
pub const GUEST_BOOT_INFO_MAGIC: [u8; 8] = *b"HVGUEST\0";

/// Hypervisor to guest boot information header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBootInfoHeader {
    /// Magic identifier (`HVGUEST\0`).
    pub magic: [u8; 8],
    /// ABI version number.
    pub version: u32,
    /// Total guest boot info size in bytes.
    pub size: u32,
    /// Assigned VM identifier.
    pub vm_id: VmId,
    /// Initial vCPU identifier.
    pub vcpu_id: VcpuId,
    /// Offset to memory region table.
    pub memory_table_offset: u32,
    /// Number of memory regions.
    pub memory_region_count: u32,
    /// Offset to IPC region table.
    pub ipc_table_offset: u32,
    /// Number of IPC regions.
    pub ipc_region_count: u32,
    /// Offset to device region table.
    pub device_table_offset: u32,
    /// Number of device regions.
    pub device_region_count: u32,
}

/// Guest memory region kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestMemoryKind {
    /// Normal guest RAM.
    Ram = 1,
    /// MMIO region.
    Mmio = 2,
}

/// One guest memory region descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestMemoryRegion {
    /// Region kind.
    pub kind: GuestMemoryKind,
    /// Guest physical base address.
    pub guest_phys: GuestPhysAddr,
    /// Region size in bytes.
    pub size: u64,
}

/// IPC role for a guest mapping.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestIpcRole {
    /// Producer side of an IPC channel.
    Producer = 1,
    /// Consumer side of an IPC channel.
    Consumer = 2,
}

/// One IPC region descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestIpcRegion {
    /// IPC channel identifier.
    pub channel_id: u32,
    /// IPC role for this guest.
    pub role: GuestIpcRole,
    /// Guest physical base address.
    pub guest_phys: GuestPhysAddr,
    /// Mapping size in bytes.
    pub size: u64,
}

/// Device kind exposed to a guest.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestDeviceKind {
    /// Intel e1000 NIC MMIO and registers.
    NicE1000 = 1,
}

/// One device MMIO region descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestDeviceRegion {
    /// Device kind.
    pub kind: GuestDeviceKind,
    /// MMIO guest physical base.
    pub mmio_guest_phys: GuestPhysAddr,
    /// MMIO region size.
    pub mmio_size: u64,
}

/// Magic identifying the relay measurement extension (`RLAY`).
pub const GUEST_RELAY_MEASUREMENT_MAGIC: u32 = 0x5941_4C52;

/// Relay measurement extension schema version.
pub const GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION: u32 = 2;

/// Explicit ABI v2 relay measurement extension appended to guest boot info blobs.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestBootInfoRelayMeasurement {
    /// Extension magic (`GUEST_RELAY_MEASUREMENT_MAGIC`).
    pub magic: u32,
    /// Extension schema version.
    pub version: u32,
    /// Out-partition end-to-end relay frames completed.
    pub frames_completed: u64,
    /// TSC value captured before the sustained relay loop.
    pub tsc_start: u64,
    /// TSC value captured after the sustained relay loop.
    pub tsc_end: u64,
    /// Guest physical base of the hypervisor-owned relay measurement page (extension v2+).
    pub measurement_page_gpa: u64,
}

/// Size of the relay measurement extension tail appended to guest boot info blobs.
pub const GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES: usize =
    core::mem::size_of::<GuestBootInfoRelayMeasurement>();

/// Returns the byte offset of the relay measurement extension in a boot info blob.
pub fn guest_boot_info_relay_measurement_offset(total_size: u32) -> Option<usize> {
    let total = total_size as usize;
    if total < core::mem::size_of::<GuestBootInfoHeader>() + GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES
    {
        return None;
    }
    Some(total - GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES)
}

/// Legacy alias retained for callers referencing relay-frame counter tail sizing.
pub fn guest_boot_info_relay_frames_offset(total_size: u32) -> Option<usize> {
    guest_boot_info_relay_measurement_offset(total_size)
        .map(|offset| offset + core::mem::offset_of!(GuestBootInfoRelayMeasurement, frames_completed))
}

/// Parses and validates the relay measurement extension from a boot info blob.
pub fn parse_guest_boot_info_relay_measurement(
    bytes: &[u8],
) -> Option<GuestBootInfoRelayMeasurement> {
    if bytes.len() < core::mem::size_of::<GuestBootInfoHeader>() {
        return None;
    }
    let total_size = read_u32(bytes, 12)?;
    let offset = guest_boot_info_relay_measurement_offset(total_size)?;
    let tail = bytes.get(offset..offset + GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES)?;
    let extension = GuestBootInfoRelayMeasurement {
        magic: read_u32(tail, 0)?,
        version: read_u32(tail, 4)?,
        frames_completed: read_u64(tail, 8)?,
        tsc_start: read_u64(tail, 16)?,
        tsc_end: read_u64(tail, 24)?,
        measurement_page_gpa: read_u64(tail, 32).unwrap_or(0),
    };
    if extension.magic != GUEST_RELAY_MEASUREMENT_MAGIC {
        return None;
    }
    if extension.version == 0 || extension.version > GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION {
        return None;
    }
    if extension.version >= 2 && tail.len() < 40 {
        return None;
    }
    Some(extension)
}

/// Parses a relay measurement page header sample (first 40 bytes of the counter page).
pub fn parse_relay_measurement_page_header(
    bytes: &[u8],
) -> Option<GuestBootInfoRelayMeasurement> {
    if bytes.len() < GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES {
        return None;
    }
    let tail = bytes.get(0..GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES)?;
    let extension = GuestBootInfoRelayMeasurement {
        magic: read_u32(tail, 0)?,
        version: read_u32(tail, 4)?,
        frames_completed: read_u64(tail, 8)?,
        tsc_start: read_u64(tail, 16)?,
        tsc_end: read_u64(tail, 24)?,
        measurement_page_gpa: read_u64(tail, 32)?,
    };
    if extension.magic != GUEST_RELAY_MEASUREMENT_MAGIC {
        return None;
    }
    if extension.version == 0 || extension.version > GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION {
        return None;
    }
    if extension.version >= 2 && extension.measurement_page_gpa == 0 {
        return None;
    }
    Some(extension)
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let slice = bytes.get(offset..offset + 8)?;
    Some(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

/// Returns whether a guest boot info header matches the supported ABI.
pub fn guest_abi_is_compatible(header: &GuestBootInfoHeader) -> bool {
    header.magic == GUEST_BOOT_INFO_MAGIC
        && header.version >= GUEST_ABI_VERSION_MIN
        && header.version <= GUEST_ABI_VERSION
}

/// Returns whether the header declares the relay measurement tail (ABI v2+).
pub fn guest_boot_info_has_relay_measurement_tail(header: &GuestBootInfoHeader) -> bool {
    header.version >= 2 && guest_boot_info_relay_measurement_offset(header.size).is_some()
}

/// Returns elapsed TSC ticks when `tsc_end >= tsc_start`.
pub fn guest_relay_measurement_elapsed_tsc(extension: &GuestBootInfoRelayMeasurement) -> u64 {
    extension.tsc_end.saturating_sub(extension.tsc_start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn guest_header_layout_is_stable() {
        assert_eq!(size_of::<GuestBootInfoHeader>(), 48);
        assert_eq!(align_of::<GuestBootInfoHeader>(), 4);
    }

    #[test]
    fn descriptor_sizes_are_stable() {
        assert_eq!(size_of::<GuestMemoryRegion>(), 24);
        assert_eq!(size_of::<GuestIpcRegion>(), 24);
        assert_eq!(size_of::<GuestDeviceRegion>(), 24);
    }

    #[test]
    fn guest_abi_compatibility_checks() {
        let ok = GuestBootInfoHeader {
            magic: GUEST_BOOT_INFO_MAGIC,
            version: GUEST_ABI_VERSION,
            size: size_of::<GuestBootInfoHeader>() as u32,
            vm_id: VmId::new(0),
            vcpu_id: VcpuId::new(0),
            memory_table_offset: 0,
            memory_region_count: 0,
            ipc_table_offset: 0,
            ipc_region_count: 0,
            device_table_offset: 0,
            device_region_count: 0,
        };
        assert!(guest_abi_is_compatible(&ok));

        let bad_version = GuestBootInfoHeader {
            version: GUEST_ABI_VERSION + 1,
            ..ok
        };
        assert!(!guest_abi_is_compatible(&bad_version));

        let v1 = GuestBootInfoHeader {
            version: 1,
            ..ok
        };
        assert!(guest_abi_is_compatible(&v1));
    }

    #[test]
    fn guest_boot_info_relay_tail_requires_v2_size() {
        let header = GuestBootInfoHeader {
            magic: GUEST_BOOT_INFO_MAGIC,
            version: 2,
            size: (size_of::<GuestBootInfoHeader>() + GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES)
                as u32,
            vm_id: VmId::new(0),
            vcpu_id: VcpuId::new(0),
            memory_table_offset: size_of::<GuestBootInfoHeader>() as u32,
            memory_region_count: 0,
            ipc_table_offset: size_of::<GuestBootInfoHeader>() as u32,
            ipc_region_count: 0,
            device_table_offset: size_of::<GuestBootInfoHeader>() as u32,
            device_region_count: 0,
        };
        assert!(guest_boot_info_has_relay_measurement_tail(&header));
        assert_eq!(size_of::<GuestBootInfoRelayMeasurement>(), 40);
    }

    #[test]
    fn parse_guest_boot_info_relay_measurement_validates_magic() {
        let mut blob = [0u8; 48 + 40];
        blob[0..8].copy_from_slice(&GUEST_BOOT_INFO_MAGIC);
        blob[8..12].copy_from_slice(&2u32.to_le_bytes());
        let total = blob.len() as u32;
        blob[12..16].copy_from_slice(&total.to_le_bytes());
        let offset = guest_boot_info_relay_measurement_offset(total).expect("offset");
        blob[offset..offset + 4].copy_from_slice(&GUEST_RELAY_MEASUREMENT_MAGIC.to_le_bytes());
        blob[offset + 4..offset + 8].copy_from_slice(&1u32.to_le_bytes());
        blob[offset + 8..offset + 16].copy_from_slice(&64u64.to_le_bytes());
        blob[offset + 16..offset + 24].copy_from_slice(&100u64.to_le_bytes());
        blob[offset + 24..offset + 32].copy_from_slice(&1_000_100u64.to_le_bytes());
        let extension = parse_guest_boot_info_relay_measurement(&blob).expect("parse");
        assert_eq!(extension.frames_completed, 64);
        assert_eq!(guest_relay_measurement_elapsed_tsc(&extension), 1_000_000);
    }

    #[test]
    fn parse_relay_measurement_page_header_requires_v2_gpa() {
        let mut header = [0u8; 40];
        header[0..4].copy_from_slice(&GUEST_RELAY_MEASUREMENT_MAGIC.to_le_bytes());
        header[4..8].copy_from_slice(&2u32.to_le_bytes());
        header[8..16].copy_from_slice(&8u64.to_le_bytes());
        header[32..40].copy_from_slice(&0xFEB2_0000u64.to_le_bytes());
        let extension = parse_relay_measurement_page_header(&header).expect("parse");
        assert_eq!(extension.measurement_page_gpa, 0xFEB2_0000);
        assert_eq!(extension.frames_completed, 8);

        header[32..40].copy_from_slice(&0u64.to_le_bytes());
        assert!(parse_relay_measurement_page_header(&header).is_none());
    }
}
