//! Guest boot info blob builder from static platform layout.

use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;

use hv_datapath::{plan_datapath_for_partition, plan_datapath_for_vm_id, DatapathPartitionPlan};
use hv_guest_abi::{
    guest_abi_is_compatible, GuestBootInfoHeader, GuestDeviceRegion, GuestIpcRegion,
    GuestMemoryRegion, GUEST_ABI_VERSION, GUEST_BOOT_INFO_MAGIC,
    GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES,
};
use hv_platform_model::StaticPlatformIR;
use hv_types::VmId;

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
    let plan = plan_datapath_for_partition(layout, partition_id).map_err(map_datapath_error)?;
    serialize_guest_boot_info(&plan)
}

/// Builds a guest boot info blob for one partition by VM id.
pub fn build_guest_boot_info_for_vm_id(
    layout: &StaticPlatformIR,
    vm_id: VmId,
) -> Result<Vec<u8>, GuestBootInfoBuildError> {
    let plan = plan_datapath_for_vm_id(layout, vm_id).map_err(map_datapath_error)?;
    serialize_guest_boot_info(&plan)
}

/// Builds guest boot info blobs for every partition in the static layout.
pub fn build_guest_boot_infos_all_partitions(
    layout: &StaticPlatformIR,
) -> Result<Vec<(VmId, Vec<u8>)>, GuestBootInfoBuildError> {
    let mut blobs = Vec::new();
    for guest in &layout.guest_memory {
        let blob = build_guest_boot_info_for_vm_id(layout, guest.vm_id)?;
        blobs.push((guest.vm_id, blob));
    }
    Ok(blobs)
}

fn map_datapath_error(err: hv_datapath::DatapathError) -> GuestBootInfoBuildError {
    GuestBootInfoBuildError::new(GuestBootInfoBuildErrorKind::InvalidInput, err.message)
}

fn serialize_guest_boot_info(plan: &DatapathPartitionPlan) -> Result<Vec<u8>, GuestBootInfoBuildError> {
    let header_size = size_of::<GuestBootInfoHeader>();
    let memory_table_bytes = size_of::<GuestMemoryRegion>() * plan.memory_regions.len();
    let ipc_table_bytes = size_of::<GuestIpcRegion>() * plan.ipc_regions.len();
    let device_table_bytes = size_of::<GuestDeviceRegion>() * plan.device_regions.len();
    let memory_table_offset = header_size as u32;
    let ipc_table_offset = memory_table_offset + memory_table_bytes as u32;
    let device_table_offset = ipc_table_offset + ipc_table_bytes as u32;
    let tables_size = device_table_offset + device_table_bytes as u32;
    let total_size = tables_size + GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES as u32;
    let header = GuestBootInfoHeader {
        magic: GUEST_BOOT_INFO_MAGIC,
        version: GUEST_ABI_VERSION,
        size: total_size,
        vm_id: plan.vm_id,
        vcpu_id: hv_types::VcpuId::new(0),
        memory_table_offset,
        memory_region_count: plan.memory_regions.len() as u32,
        ipc_table_offset,
        ipc_region_count: plan.ipc_regions.len() as u32,
        device_table_offset,
        device_region_count: plan.device_regions.len() as u32,
    };
    if !guest_abi_is_compatible(&header) {
        return Err(GuestBootInfoBuildError::new(
            GuestBootInfoBuildErrorKind::InvalidInput,
            "constructed guest boot info header failed compatibility check",
        ));
    }
    let mut bytes = vec![0u8; total_size as usize];
    write_guest_boot_info_header(&mut bytes, &header);
    for (index, region) in plan.memory_regions.iter().enumerate() {
        let offset = header_size + index * size_of::<GuestMemoryRegion>();
        write_guest_memory_region(&mut bytes, offset, region);
    }
    for (index, region) in plan.ipc_regions.iter().enumerate() {
        let offset = ipc_table_offset as usize + index * size_of::<GuestIpcRegion>();
        write_guest_ipc_region(&mut bytes, offset, region);
    }
    for (index, region) in plan.device_regions.iter().enumerate() {
        let offset = device_table_offset as usize + index * size_of::<GuestDeviceRegion>();
        write_guest_device_region(&mut bytes, offset, region);
    }
    // Relay-frame counter tail (guest increments under live VMX; hypervisor reads after execution).
    let tail_offset = tables_size as usize;
    bytes[tail_offset..tail_offset + GUEST_BOOT_INFO_RELAY_MEASUREMENT_TAIL_BYTES]
        .copy_from_slice(&0u64.to_le_bytes());
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

fn write_guest_ipc_region(bytes: &mut [u8], offset: usize, region: &GuestIpcRegion) {
    write_u32(bytes, offset, region.channel_id);
    write_u32(bytes, offset + 4, region.role as u32);
    write_u64(bytes, offset + 8, region.guest_phys.raw());
    write_u64(bytes, offset + 16, region.size);
}

fn write_guest_device_region(bytes: &mut [u8], offset: usize, region: &GuestDeviceRegion) {
    write_u32(bytes, offset, region.kind as u32);
    write_u64(bytes, offset + 8, region.mmio_guest_phys.raw());
    write_u64(bytes, offset + 16, region.mmio_size);
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
    use hv_guest_abi::GuestIpcRole;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn build_guest_boot_info_for_reference_in_partition() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let blob = build_guest_boot_info_for_partition(&layout, "in").expect("build");
        let header = read_guest_boot_info_header(&blob);
        assert!(guest_abi_is_compatible(&header));
        assert_eq!(header.memory_region_count, 1);
        assert_eq!(header.ipc_region_count, 1);
        assert_eq!(header.device_region_count, 1);
        assert_eq!(header.vm_id, VmId::new(0));
    }

    #[test]
    fn build_guest_boot_info_for_reference_mid_partition() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let blob = build_guest_boot_info_for_partition(&layout, "mid").expect("build");
        let header = read_guest_boot_info_header(&blob);
        assert_eq!(header.ipc_region_count, 2);
        assert_eq!(header.device_region_count, 0);
    }

    #[test]
    fn build_guest_boot_infos_all_partitions_returns_three_blobs() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let blobs = build_guest_boot_infos_all_partitions(&layout).expect("build all");
        assert_eq!(blobs.len(), 3);
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

    #[test]
    fn in_partition_ipc_role_is_producer() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let blob = build_guest_boot_info_for_partition(&layout, "in").expect("build");
        let view = crate::parse::GuestBootInfoView::parse(&blob).expect("parse");
        let ipc = view.ipc_region(0).expect("ipc");
        assert_eq!(ipc.role, GuestIpcRole::Producer);
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
