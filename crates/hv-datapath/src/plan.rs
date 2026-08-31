//! Datapath descriptor planning from static platform layout.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use hv_guest_abi::{
    GuestDeviceKind, GuestDeviceRegion, GuestIpcRegion, GuestIpcRole, GuestMemoryKind,
    GuestMemoryRegion,
};
use hv_platform_model::{PlannedGuestMemory, StaticPlatformIR};
use hv_types::{GuestPhysAddr, VmId};

use crate::constants::{
    E1000_MMIO_GUEST_PHYS_BASE, E1000_MMIO_GUEST_PHYS_STRIDE, E1000_MMIO_SIZE_BYTES,
};
use crate::error::{DatapathError, DatapathErrorKind};

/// Per-partition datapath view derived from static platform layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathPartitionPlan {
    /// Assigned VM identifier.
    pub vm_id: VmId,
    /// Partition identifier when available from host planning.
    pub partition_id: String,
    /// Guest RAM regions for the partition.
    pub memory_regions: Vec<GuestMemoryRegion>,
    /// IPC mappings owned by the partition.
    pub ipc_regions: Vec<GuestIpcRegion>,
    /// Device MMIO regions assigned to the partition.
    pub device_regions: Vec<GuestDeviceRegion>,
}

/// Plans datapath descriptors for one partition by stable partition id.
pub fn plan_datapath_for_partition(
    layout: &StaticPlatformIR,
    partition_id: &str,
) -> Result<DatapathPartitionPlan, DatapathError> {
    let guest = layout
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
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "partition not found in static platform layout",
            )
        })?;
    plan_datapath_for_guest(layout, guest)
}

/// Plans datapath descriptors for one partition by VM id.
pub fn plan_datapath_for_vm_id(
    layout: &StaticPlatformIR,
    vm_id: VmId,
) -> Result<DatapathPartitionPlan, DatapathError> {
    let guest = layout
        .guest_memory
        .iter()
        .find(|entry| entry.vm_id == vm_id)
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "vm id not found in static platform layout",
            )
        })?;
    plan_datapath_for_guest(layout, guest)
}

fn plan_datapath_for_guest(
    layout: &StaticPlatformIR,
    guest: &PlannedGuestMemory,
) -> Result<DatapathPartitionPlan, DatapathError> {
    let memory_regions = vec![GuestMemoryRegion {
        kind: GuestMemoryKind::Ram,
        guest_phys: GuestPhysAddr::new(guest.host_phys.raw()),
        size: guest.size.bytes(),
    }];

    let mut ipc_regions = Vec::new();
    for channel in &layout.ipc_memory {
        if let Some(role) = ipc_role_for_vm(channel.producer_vm_id, channel.consumer_vm_id, guest.vm_id)
        {
            ipc_regions.push(GuestIpcRegion {
                channel_id: channel.channel_id.raw(),
                role,
                guest_phys: GuestPhysAddr::new(channel.host_phys.raw()),
                size: channel.size.bytes(),
            });
        }
    }

    let mut device_regions = Vec::new();
    for device in &layout.pci_devices {
        if device.vm_id != guest.vm_id {
            continue;
        }
        if device.kind != "nic_e1000" {
            continue;
        }
        device_regions.push(GuestDeviceRegion {
            kind: GuestDeviceKind::NicE1000,
            mmio_guest_phys: plan_e1000_mmio_guest_phys(guest.vm_id)?,
            mmio_size: E1000_MMIO_SIZE_BYTES,
        });
    }

    Ok(DatapathPartitionPlan {
        vm_id: guest.vm_id,
        partition_id: guest.partition_id.clone(),
        memory_regions,
        ipc_regions,
        device_regions,
    })
}

fn ipc_role_for_vm(
    producer_vm_id: VmId,
    consumer_vm_id: VmId,
    vm_id: VmId,
) -> Option<GuestIpcRole> {
    if producer_vm_id == vm_id {
        Some(GuestIpcRole::Producer)
    } else if consumer_vm_id == vm_id {
        Some(GuestIpcRole::Consumer)
    } else {
        None
    }
}

/// Plans the guest physical base for an e1000 MMIO window.
pub fn plan_e1000_mmio_guest_phys(vm_id: VmId) -> Result<GuestPhysAddr, DatapathError> {
    let offset = u64::from(vm_id.raw())
        .checked_mul(E1000_MMIO_GUEST_PHYS_STRIDE)
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "e1000 mmio guest phys offset overflow",
            )
        })?;
    let base = E1000_MMIO_GUEST_PHYS_BASE
        .checked_add(offset)
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "e1000 mmio guest phys base overflow",
            )
        })?;
    Ok(GuestPhysAddr::new(base))
}

/// Returns the out-partition IPC consumer queue guest physical address.
pub fn plan_out_ipc_consumer_guest_phys(layout: &StaticPlatformIR) -> Result<u64, DatapathError> {
    let plan = plan_datapath_for_vm_id(layout, VmId::new(2))?;
    let consumer = plan
        .ipc_regions
        .iter()
        .find(|region| region.role == GuestIpcRole::Consumer)
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "out partition missing IPC consumer region",
            )
        })?;
    Ok(consumer.guest_phys.raw())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn reference_in_partition_has_ipc_producer_and_e1000_device() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_datapath_for_partition(&layout, "in").expect("plan");
        assert_eq!(plan.vm_id, VmId::new(0));
        assert_eq!(plan.memory_regions.len(), 1);
        assert_eq!(plan.ipc_regions.len(), 1);
        assert_eq!(plan.ipc_regions.first().expect("ipc").role, GuestIpcRole::Producer);
        assert_eq!(plan.device_regions.len(), 1);
    }

    #[test]
    fn reference_mid_partition_has_two_ipc_roles_and_no_devices() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_datapath_for_partition(&layout, "mid").expect("plan");
        assert_eq!(plan.ipc_regions.len(), 2);
        assert!(plan.device_regions.is_empty());
    }

    #[test]
    fn reference_out_partition_has_ipc_consumer_and_e1000_device() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_datapath_for_partition(&layout, "out").expect("plan");
        assert_eq!(plan.ipc_regions.len(), 1);
        assert_eq!(plan.ipc_regions.first().expect("ipc").role, GuestIpcRole::Consumer);
        assert_eq!(plan.device_regions.len(), 1);
    }

    #[test]
    fn reference_guest_ipc_layout_matches_planner() {
        use crate::constants::{
            REFERENCE_IPC_CHAN_A_GUEST_PHYS, REFERENCE_IPC_CHAN_B_GUEST_PHYS,
            REFERENCE_IPC_SHARED_BYTES,
        };

        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let chan_a = layout.ipc_memory.first().expect("chan_a");
        let chan_b = layout.ipc_memory.get(1).expect("chan_b");
        assert_eq!(chan_a.host_phys.raw(), REFERENCE_IPC_CHAN_A_GUEST_PHYS);
        assert_eq!(chan_b.host_phys.raw(), REFERENCE_IPC_CHAN_B_GUEST_PHYS);
        assert_eq!(chan_a.size.bytes(), REFERENCE_IPC_SHARED_BYTES);
        assert_eq!(chan_b.size.bytes(), REFERENCE_IPC_SHARED_BYTES);

        let in_plan = plan_datapath_for_partition(&layout, "in").expect("in");
        let producer = in_plan
            .ipc_regions
            .iter()
            .find(|region| region.role == GuestIpcRole::Producer)
            .expect("in producer");
        assert_eq!(producer.guest_phys.raw(), REFERENCE_IPC_CHAN_A_GUEST_PHYS);
        assert_eq!(producer.size, REFERENCE_IPC_SHARED_BYTES);

        let mid_plan = plan_datapath_for_partition(&layout, "mid").expect("mid");
        let mid_consumer = mid_plan
            .ipc_regions
            .iter()
            .find(|region| region.role == GuestIpcRole::Consumer)
            .expect("mid consumer");
        let mid_producer = mid_plan
            .ipc_regions
            .iter()
            .find(|region| region.role == GuestIpcRole::Producer)
            .expect("mid producer");
        assert_eq!(mid_consumer.guest_phys.raw(), REFERENCE_IPC_CHAN_A_GUEST_PHYS);
        assert_eq!(mid_producer.guest_phys.raw(), REFERENCE_IPC_CHAN_B_GUEST_PHYS);

        let out_plan = plan_datapath_for_partition(&layout, "out").expect("out");
        let out_consumer = out_plan
            .ipc_regions
            .iter()
            .find(|region| region.role == GuestIpcRole::Consumer)
            .expect("out consumer");
        assert_eq!(out_consumer.guest_phys.raw(), REFERENCE_IPC_CHAN_B_GUEST_PHYS);
    }
}
