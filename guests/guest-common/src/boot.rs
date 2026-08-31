//! Guest boot info resolution with reference-layout fallback.

use core::mem::size_of;

use hv_guest_abi::{
    guest_abi_is_compatible, GuestBootInfoHeader, GuestDeviceKind, GuestDeviceRegion,
    GuestIpcRegion, GuestIpcRole, GUEST_BOOT_INFO_MAGIC,
};
use hv_types::GuestPhysAddr;

use crate::layout::{IpcQueueMapping, ResolvedLayout, Role};

/// Resolves layout for a known role with boot info fallback.
pub fn resolve_layout_for_role(role: Role, boot_info: *const u8) -> ResolvedLayout {
    if boot_info.is_null() {
        return reference_layout_for_role(role);
    }
    unsafe {
        if let Some(layout) = layout_from_boot_info_for_role(role, boot_info) {
            return layout;
        }
    }
    reference_layout_for_role(role)
}

/// Resolves layout from boot info when valid, otherwise uses IN reference constants.
pub fn resolve_layout(boot_info: *const u8) -> ResolvedLayout {
    resolve_layout_for_role(Role::In, boot_info)
}

fn reference_layout_for_role(role: Role) -> ResolvedLayout {
    match role {
        Role::In => ResolvedLayout::reference_in(),
        Role::Mid => ResolvedLayout::reference_mid(),
        Role::Out => ResolvedLayout::reference_out(),
    }
}

unsafe fn layout_from_boot_info_for_role(role: Role, boot_info: *const u8) -> Option<ResolvedLayout> {
    let header = read_header(boot_info)?;
    let layout = build_layout(boot_info, &header)?;
    if layout_matches_role(role, &layout, &header) {
        Some(layout)
    } else {
        None
    }
}

unsafe fn build_layout(boot_info: *const u8, header: &GuestBootInfoHeader) -> Option<ResolvedLayout> {
    let mut e1000_mmio = None;
    let mut ipc_producer = None;
    let mut ipc_consumer = None;

    for index in 0..header.device_region_count {
        let region = read_device_region(boot_info, header, index)?;
        if region.kind == GuestDeviceKind::NicE1000 {
            e1000_mmio = Some(region.mmio_guest_phys);
        }
    }

    for index in 0..header.ipc_region_count {
        let region = read_ipc_region(boot_info, header, index)?;
        let mapping = IpcQueueMapping {
            guest_phys: region.guest_phys,
            size: region.size,
        };
        match region.role {
            GuestIpcRole::Producer => ipc_producer = Some(mapping),
            GuestIpcRole::Consumer => ipc_consumer = Some(mapping),
        }
    }

    Some(ResolvedLayout {
        e1000_mmio,
        ipc_producer,
        ipc_consumer,
    })
}

fn layout_matches_role(role: Role, layout: &ResolvedLayout, header: &GuestBootInfoHeader) -> bool {
    let _ = header;
    match role {
        Role::In => layout.ipc_producer.is_some() && layout.e1000_mmio.is_some(),
        Role::Mid => layout.ipc_producer.is_some() && layout.ipc_consumer.is_some(),
        Role::Out => layout.ipc_consumer.is_some() && layout.e1000_mmio.is_some(),
    }
}

unsafe fn read_header(boot_info: *const u8) -> Option<GuestBootInfoHeader> {
    let bytes = core::slice::from_raw_parts(boot_info, size_of::<GuestBootInfoHeader>());
    let magic = read_bytes::<8>(bytes, 0)?;
    if magic != GUEST_BOOT_INFO_MAGIC {
        return None;
    }
    let header = GuestBootInfoHeader {
        magic,
        version: read_u32(bytes, 8)?,
        size: read_u32(bytes, 12)?,
        vm_id: hv_types::VmId::new(read_u32(bytes, 16)?),
        vcpu_id: hv_types::VcpuId::new(read_u32(bytes, 20)?),
        memory_table_offset: read_u32(bytes, 24)?,
        memory_region_count: read_u32(bytes, 28)?,
        ipc_table_offset: read_u32(bytes, 32)?,
        ipc_region_count: read_u32(bytes, 36)?,
        device_table_offset: read_u32(bytes, 40)?,
        device_region_count: read_u32(bytes, 44)?,
    };
    if !guest_abi_is_compatible(&header) {
        return None;
    }
    Some(header)
}

unsafe fn read_ipc_region(
    boot_info: *const u8,
    header: &GuestBootInfoHeader,
    index: u32,
) -> Option<GuestIpcRegion> {
    let offset = header.ipc_table_offset as usize + index as usize * size_of::<GuestIpcRegion>();
    let bytes = core::slice::from_raw_parts(boot_info, header.size as usize);
    let table = bytes.get(offset..)?;
    Some(GuestIpcRegion {
        channel_id: read_u32(table, 0)?,
        role: match read_u32(table, 4)? {
            1 => GuestIpcRole::Producer,
            2 => GuestIpcRole::Consumer,
            _ => return None,
        },
        guest_phys: GuestPhysAddr::new(read_u64(table, 8)?),
        size: read_u64(table, 16)?,
    })
}

unsafe fn read_device_region(
    boot_info: *const u8,
    header: &GuestBootInfoHeader,
    index: u32,
) -> Option<GuestDeviceRegion> {
    let offset = header.device_table_offset as usize + index as usize * size_of::<GuestDeviceRegion>();
    let bytes = core::slice::from_raw_parts(boot_info, header.size as usize);
    let table = bytes.get(offset..)?;
    Some(GuestDeviceRegion {
        kind: match read_u32(table, 0)? {
            1 => GuestDeviceKind::NicE1000,
            _ => return None,
        },
        mmio_guest_phys: GuestPhysAddr::new(read_u64(table, 8)?),
        mmio_size: read_u64(table, 16)?,
    })
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

fn read_bytes<const N: usize>(bytes: &[u8], offset: usize) -> Option<[u8; N]> {
    let slice = bytes.get(offset..offset + N)?;
    let mut out = [0u8; N];
    out.copy_from_slice(slice);
    Some(out)
}
