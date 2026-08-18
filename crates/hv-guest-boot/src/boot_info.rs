//! Guest boot info blob builder from static platform layout.

use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;

use hv_guest_abi::{
    guest_abi_is_compatible, GuestBootInfoHeader, GuestMemoryKind, GuestMemoryRegion,
    GUEST_ABI_VERSION, GUEST_BOOT_INFO_MAGIC,
};
use hv_platform_model::StaticPlatformIR;
use hv_types::GuestPhysAddr;

/// Category of guest boot info build failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestBootInfoBuildErrorKind {
    /// Partition or layout input was invalid.
    InvalidInput,
}

/// Structured guest boot info build error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestBootInfoBuildError {
    /// Error category.
    pub kind: GuestBootInfoBuildErrorKind,
    /// Human-readable message.
    pub message: alloc::string::String,
}

impl GuestBootInfoBuildError {
    /// Creates a new guest boot info build error.
    pub fn new(kind: GuestBootInfoBuildErrorKind, message: impl Into<alloc::string::String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Builds a guest boot info blob for one partition using identity guest physical addresses.
pub fn build_guest_boot_info_for_partition(
    layout: &StaticPlatformIR,
    partition_id: &str,
) -> Result<Vec<u8>, GuestBootInfoBuildError> {
    let region = layout
        .guest_memory
        .iter()
        .find(|entry| entry.partition_id == partition_id)
        .or_else(|| {
            layout
                .guest_memory
                .iter()
                .min_by_key(|entry| entry.vm_id.raw())
        })
        .ok_or_else(|| {
            GuestBootInfoBuildError::new(
                GuestBootInfoBuildErrorKind::InvalidInput,
                "partition not found in static platform layout",
            )
        })?;
    let header_size = size_of::<GuestBootInfoHeader>();
    let memory_table_offset = header_size as u32;
    let memory_table_bytes = size_of::<GuestMemoryRegion>();
    let total_size = header_size + memory_table_bytes;
    let header = GuestBootInfoHeader {
        magic: GUEST_BOOT_INFO_MAGIC,
        version: GUEST_ABI_VERSION,
        size: total_size as u32,
        vm_id: region.vm_id,
        vcpu_id: hv_types::VcpuId::new(0),
        memory_table_offset,
        memory_region_count: 1,
        ipc_table_offset: total_size as u32,
        ipc_region_count: 0,
        device_table_offset: total_size as u32,
        device_region_count: 0,
    };
    if !guest_abi_is_compatible(&header) {
        return Err(GuestBootInfoBuildError::new(
            GuestBootInfoBuildErrorKind::InvalidInput,
            "constructed guest boot info header failed compatibility check",
        ));
    }
    let memory = GuestMemoryRegion {
        kind: GuestMemoryKind::Ram,
        guest_phys: GuestPhysAddr::new(region.host_phys.raw()),
        size: region.size.bytes(),
    };
    let mut bytes = vec![0u8; total_size];
    write_guest_boot_info_header(&mut bytes, &header);
    write_guest_memory_region(&mut bytes, header_size, &memory);
    Ok(bytes)
}

fn write_guest_boot_info_header(bytes: &mut [u8], header: &GuestBootInfoHeader) {
    if let Some(prefix) = bytes.get_mut(0..8) {
        prefix.copy_from_slice(&header.magic);
    }
    write_u32(bytes, 8, header.version);
    write_u32(bytes, 12, header.size);
    write_u32(bytes, 16, header.vm_id.raw());
    write_u32(bytes, 20, header.vcpu_id.raw());
    write_u32(bytes, 24, header.memory_table_offset);
    write_u32(bytes, 28, header.memory_region_count);
    write_u32(bytes, 32, header.ipc_table_offset);
    write_u32(bytes, 36, header.ipc_region_count);
    write_u32(bytes, 40, header.device_table_offset);
    write_u32(bytes, 44, header.device_region_count);
}

fn write_guest_memory_region(bytes: &mut [u8], offset: usize, region: &GuestMemoryRegion) {
    write_u32(bytes, offset, region.kind as u32);
    write_u64(bytes, offset + 8, region.guest_phys.raw());
    write_u64(bytes, offset + 16, region.size);
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    if let Some(slice) = bytes.get_mut(offset..offset + 4) {
        slice.copy_from_slice(&value.to_le_bytes());
    }
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    if let Some(slice) = bytes.get_mut(offset..offset + 8) {
        slice.copy_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use hv_types::VmId;

    #[test]
    fn build_guest_boot_info_for_reference_in_partition() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let blob = build_guest_boot_info_for_partition(&layout, "in").expect("build");
        let header = read_guest_boot_info_header(&blob);
        assert!(guest_abi_is_compatible(&header));
        assert_eq!(header.memory_region_count, 1);
        assert_eq!(header.vm_id, VmId::new(0));
    }

    #[test]
    fn build_guest_boot_info_falls_back_to_first_guest_for_unknown_partition_id() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let blob = build_guest_boot_info_for_partition(&layout, "missing").expect("fallback");
        let header = read_guest_boot_info_header(&blob);
        assert_eq!(header.vm_id, VmId::new(0));
    }

    fn read_guest_boot_info_header(bytes: &[u8]) -> GuestBootInfoHeader {
        let mut magic = [0u8; 8];
        magic.copy_from_slice(bytes.get(0..8).expect("magic"));
        GuestBootInfoHeader {
            magic,
            version: read_u32(bytes, 8),
            size: read_u32(bytes, 12),
            vm_id: VmId::new(read_u32(bytes, 16)),
            vcpu_id: hv_types::VcpuId::new(read_u32(bytes, 20)),
            memory_table_offset: read_u32(bytes, 24),
            memory_region_count: read_u32(bytes, 28),
            ipc_table_offset: read_u32(bytes, 32),
            ipc_region_count: read_u32(bytes, 36),
            device_table_offset: read_u32(bytes, 40),
            device_region_count: read_u32(bytes, 44),
        }
    }

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        let slice = bytes.get(offset..offset + 4).expect("u32");
        u32::from_le_bytes([
            slice.first().copied().unwrap_or(0),
            slice.get(1).copied().unwrap_or(0),
            slice.get(2).copied().unwrap_or(0),
            slice.get(3).copied().unwrap_or(0),
        ])
    }
}
