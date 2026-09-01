//! Deterministic host physical layout planner for static platform IR.

use hv_config_model::StaticIntentIR;
use hv_types::{align_up, HostPhysAddr, PciBdf};

use crate::coherence::validate_layout_host_network_coherence;
use crate::constants::{platform_phys_base, REGION_ALIGNMENT_BYTES};
use crate::error::{PlatformError, PlatformErrorKind};
use crate::platform_ir::{
    HostNetworkInterface, HostNetworkPlan, PlannedGuestMemory, PlannedHypervisorReserve,
    PlannedIpcMemory, PlannedPciDevice, StaticPlatformIR,
};

/// Plans static platform IR with resolved host physical addresses from intent IR.
pub fn plan_static_platform_ir(intent: &StaticIntentIR) -> Result<StaticPlatformIR, PlatformError> {
    let alignment = REGION_ALIGNMENT_BYTES as usize;
    let mut cursor = platform_phys_base().raw();

    let mut guest_memory = Vec::new();
    let mut partitions = intent.partitions.clone();
    partitions.sort_by_key(|partition| partition.vm_id.raw());
    for partition in partitions {
        cursor = align_cursor(cursor, alignment)?;
        let region = PlannedGuestMemory {
            partition_id: partition.id.clone(),
            vm_id: partition.vm_id,
            host_phys: HostPhysAddr::new(cursor),
            size: partition.memory_bytes,
        };
        cursor = advance_cursor(cursor, partition.memory_bytes.bytes())?;
        guest_memory.push(region);
    }

    let mut ipc_memory = Vec::new();
    let mut channels = intent.ipc.clone();
    channels.sort_by_key(|channel| channel.channel_id.raw());
    for channel in channels {
        cursor = align_cursor(cursor, alignment)?;
        let region = PlannedIpcMemory {
            channel_name: channel.id.clone(),
            channel_id: channel.channel_id,
            producer_vm_id: channel.producer_vm_id,
            consumer_vm_id: channel.consumer_vm_id,
            host_phys: HostPhysAddr::new(cursor),
            size: channel.shared_bytes,
        };
        cursor = advance_cursor(cursor, channel.shared_bytes.bytes())?;
        ipc_memory.push(region);
    }

    cursor = align_cursor(cursor, alignment)?;
    let reserve_size = intent.memory_intent.hypervisor_reserve_bytes;
    let hypervisor_reserve = PlannedHypervisorReserve {
        host_phys: HostPhysAddr::new(cursor),
        size: reserve_size,
    };
    let _ = advance_cursor(cursor, reserve_size.bytes())?;

    let mut pci_devices = Vec::new();
    for (bdf, vm_id, kind, role, mmio_guest_phys) in &intent.pci_intent.devices {
        pci_devices.push(PlannedPciDevice {
            bdf: *bdf,
            vm_id: *vm_id,
            kind: kind.clone(),
            mmio_guest_phys: *mmio_guest_phys,
            mmio_size_bytes: 0x20_000,
            role: role.clone(),
        });
    }

    let host_network = plan_host_network(intent)?;

    let layout = StaticPlatformIR {
        platform_name: intent.platform_name.clone(),
        guest_memory,
        ipc_memory,
        hypervisor_reserve,
        pci_devices,
        host_network,
    };
    validate_layout_host_network_coherence(&layout)?;
    Ok(layout)
}

fn plan_host_network(intent: &StaticIntentIR) -> Result<HostNetworkPlan, PlatformError> {
    let mut interfaces = Vec::new();
    for interface in &intent.qemu_plan.network.interfaces {
        let vm_id = intent
            .partitions
            .iter()
            .find(|partition| partition.id == interface.partition)
            .map(|partition| partition.vm_id)
            .ok_or_else(|| {
                PlatformError::new(
                    PlatformErrorKind::Planning,
                    "host network interface references unknown partition",
                )
            })?;
        interfaces.push(HostNetworkInterface {
            partition_id: interface.partition.clone(),
            vm_id,
            bdf: interface.bdf,
            pci_addr: interface.pci_addr.clone(),
            netdev_id: interface.netdev_id.clone(),
            mmio_guest_phys: mmio_guest_phys_for_bdf(intent, interface.bdf)?,
            tap_ifname: interface.tap_ifname.clone(),
        });
    }
    interfaces.sort_by_key(|interface| {
        (
            interface.bdf.segment.raw(),
            interface.bdf.bus.raw(),
            interface.bdf.device.raw(),
            interface.bdf.function.raw(),
        )
    });
    Ok(HostNetworkPlan {
        enabled: intent.qemu_plan.network.enabled,
        backend: intent.qemu_plan.network.backend.clone(),
        interfaces,
    })
}

fn mmio_guest_phys_for_bdf(intent: &StaticIntentIR, bdf: PciBdf) -> Result<u64, PlatformError> {
    intent
        .pci_intent
        .devices
        .iter()
        .find(|(device_bdf, _, kind, _, _)| *device_bdf == bdf && kind == "nic_e1000")
        .map(|(_, _, _, _, mmio_guest_phys)| *mmio_guest_phys)
        .ok_or_else(|| {
            PlatformError::new(
                PlatformErrorKind::Planning,
                "host network interface BDF missing from PCI intent",
            )
        })
}

fn align_cursor(cursor: u64, alignment: usize) -> Result<u64, PlatformError> {
    let aligned = align_up(cursor as usize, alignment).map_err(|_| {
        PlatformError::new(
            PlatformErrorKind::Planning,
            "failed to align platform layout cursor",
        )
    })?;
    u64::try_from(aligned).map_err(|_| {
        PlatformError::new(
            PlatformErrorKind::Planning,
            "aligned cursor exceeds u64 range",
        )
    })
}

fn advance_cursor(cursor: u64, size: u64) -> Result<u64, PlatformError> {
    cursor.checked_add(size).ok_or_else(|| {
        PlatformError::new(
            PlatformErrorKind::Planning,
            "platform layout address space overflow",
        )
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;

    #[test]
    fn planner_produces_non_overlapping_regions_for_reference_config() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let planned = plan_static_platform_ir(&compiled.intent).expect("plan");
        assert_eq!(planned.guest_memory.len(), 3);
        assert_eq!(planned.ipc_memory.len(), 2);
        assert_eq!(planned.pci_devices.len(), 2);
        assert_eq!(planned.host_network.interfaces.len(), 2);

        let mut regions = Vec::new();
        for region in &planned.guest_memory {
            regions.push((region.host_phys.raw(), region.size.bytes()));
        }
        for region in &planned.ipc_memory {
            regions.push((region.host_phys.raw(), region.size.bytes()));
        }
        regions.push((
            planned.hypervisor_reserve.host_phys.raw(),
            planned.hypervisor_reserve.size.bytes(),
        ));
        regions.sort_by_key(|(base, _)| *base);
        for index in 0..regions.len().saturating_sub(1) {
            let (base_a, size_a) = regions
                .get(index)
                .copied()
                .expect("region index must exist");
            let (base_b, _) = regions
                .get(index + 1)
                .copied()
                .expect("next region index must exist");
            assert!(base_a.saturating_add(size_a) <= base_b);
        }
    }

    #[test]
    fn planner_is_deterministic() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let first = plan_static_platform_ir(&compiled.intent).expect("plan");
        let second = plan_static_platform_ir(&compiled.intent).expect("plan");
        assert_eq!(first, second);
    }

    #[test]
    fn guest_memory_sorted_by_vm_id() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let planned = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vm_ids = planned
            .guest_memory
            .iter()
            .map(|region| region.vm_id.raw())
            .collect::<Vec<_>>();
        assert_eq!(vm_ids, vec![0, 1, 2]);
    }

    #[test]
    fn ipc_memory_sorted_by_channel_id() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let planned = plan_static_platform_ir(&compiled.intent).expect("plan");
        let channel_ids = planned
            .ipc_memory
            .iter()
            .map(|region| region.channel_id.raw())
            .collect::<Vec<_>>();
        assert_eq!(channel_ids, vec![0, 1]);
    }

    #[test]
    fn host_network_interfaces_are_independent() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let planned = plan_static_platform_ir(&compiled.intent).expect("plan");
        assert!(planned.host_network.enabled);
        assert_eq!(planned.host_network.interfaces.len(), 2);
        let tap_names = planned
            .host_network
            .interfaces
            .iter()
            .filter_map(|interface| interface.tap_ifname.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(tap_names.len(), 2);
        let first = tap_names.first().expect("first tap");
        let second = tap_names.get(1).expect("second tap");
        assert_ne!(first, second);
        let in_interface = planned
            .host_network
            .interfaces
            .iter()
            .find(|interface| interface.partition_id == "in")
            .expect("in interface");
        let out_interface = planned
            .host_network
            .interfaces
            .iter()
            .find(|interface| interface.partition_id == "out")
            .expect("out interface");
        assert_eq!(in_interface.mmio_guest_phys, 0xFEB0_0000);
        assert_eq!(out_interface.mmio_guest_phys, 0xFEB2_0000);
    }
}
