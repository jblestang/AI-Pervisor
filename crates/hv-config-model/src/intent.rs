//! Static intent intermediate representation.

use serde::{Deserialize, Serialize};

use crate::constants::{hypervisor_reserve_bytes, IPC_SLOT_METADATA_BYTES};
use crate::normalize::{NormalizedConfig, NormalizedPartition};
use crate::requirements::PlatformRequirements;
use hv_types::{ByteSize, IpcChannelId, PciBdf, VmId};

/// Static intent IR describing desired platform layout without hardware resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticIntentIR {
    /// Platform profile name.
    pub platform_name: String,
    /// Partition intents sorted by VM id.
    pub partitions: Vec<PartitionIntent>,
    /// IPC intents sorted by channel id.
    pub ipc: Vec<IpcIntent>,
    /// CPU placement intent.
    pub cpu_intent: CpuPlacementIntent,
    /// Memory layout intent.
    pub memory_intent: MemoryLayoutIntent,
    /// PCI ownership intent.
    pub pci_intent: PciOwnershipIntent,
    /// Boot intent.
    pub boot_intent: BootIntent,
    /// QEMU launch intent.
    pub qemu_plan: QemuPlanIntent,
}

/// Partition intent entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionIntent {
    /// Stable partition id.
    pub id: String,
    /// Assigned VM id.
    pub vm_id: VmId,
    /// vCPU count.
    pub vcpus: u32,
    /// Private RAM size.
    pub memory_bytes: ByteSize,
    /// Reserved physical cores.
    pub physical_cores: Vec<u32>,
    /// Assigned devices.
    pub devices: Vec<PciDeviceIntent>,
}

/// PCI device intent entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PciDeviceIntent {
    /// Device kind.
    pub kind: String,
    /// PCI BDF.
    pub bdf: PciBdf,
    /// Guest physical base for the device MMIO BAR.
    pub mmio_guest_phys: u64,
    /// Optional datapath role.
    pub role: Option<String>,
}

/// IPC channel intent entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcIntent {
    /// Stable channel id.
    pub id: String,
    /// Assigned channel id.
    pub channel_id: IpcChannelId,
    /// Producer partition id.
    pub producer: String,
    /// Producer VM id.
    pub producer_vm_id: VmId,
    /// Consumer partition id.
    pub consumer: String,
    /// Consumer VM id.
    pub consumer_vm_id: VmId,
    /// Queue slot count.
    pub queue_slots: u32,
    /// Slot payload size in bytes.
    pub slot_size_bytes: u32,
    /// Total shared memory size for the queue region.
    pub shared_bytes: ByteSize,
}

/// CPU placement intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuPlacementIntent {
    /// Minimum physical cores required.
    pub min_physical_cores: u32,
    /// Per-partition core reservations sorted by VM id.
    pub core_assignments: Vec<(VmId, Vec<u32>)>,
}

/// Memory layout intent expressed as byte sizes only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryLayoutIntent {
    /// Total guest RAM across all partitions.
    pub total_guest_bytes: ByteSize,
    /// Total IPC shared memory across all channels.
    pub total_ipc_bytes: ByteSize,
    /// Reserved hypervisor memory placeholder size.
    pub hypervisor_reserve_bytes: ByteSize,
}

/// PCI ownership intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PciOwnershipIntent {
    /// Device ownership records sorted by BDF.
    pub devices: Vec<(PciBdf, VmId, String, Option<String>, u64)>,
}

/// Boot intent for guest images.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootIntent {
    /// Guest images sorted by VM id.
    pub guest_images: Vec<GuestImageIntent>,
}

/// Guest image intent entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuestImageIntent {
    /// Partition id.
    pub partition: String,
    /// VM id.
    pub vm_id: VmId,
    /// Image path.
    pub path: String,
    /// Expected SHA-256 hash.
    pub sha256: String,
}

/// QEMU plan intent derived from configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QemuPlanIntent {
    /// Machine type.
    pub machine: String,
    /// CPU count.
    pub cpus: u32,
    /// Memory bytes.
    pub memory_bytes: ByteSize,
    /// CPU model.
    pub cpu_model: String,
    /// Accelerator.
    pub accel: String,
    /// Socket count.
    pub smp_sockets: u32,
    /// Core count per socket.
    pub smp_cores: u32,
    /// Thread count per core.
    pub smp_threads: u32,
    /// Host network plan for outer QEMU e1000 devices.
    pub network: QemuNetworkPlanIntent,
}

/// Host network interface intent from configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QemuNetworkInterfaceIntent {
    /// Owning partition id.
    pub partition: String,
    /// PCI BDF.
    pub bdf: PciBdf,
    /// QEMU PCI slot address.
    pub pci_addr: String,
    /// QEMU netdev identifier.
    pub netdev_id: String,
    /// Tap interface name when backend is `tap`.
    pub tap_ifname: Option<String>,
}

/// Host network plan intent from configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QemuNetworkPlanIntent {
    /// Whether host networking is enabled.
    pub enabled: bool,
    /// Netdev backend (`user` or `tap`).
    pub backend: String,
    /// Independent host interface entries sorted by BDF.
    pub interfaces: Vec<QemuNetworkInterfaceIntent>,
}

/// Builds static intent IR from normalized configuration and requirements.
pub fn static_intent_ir(
    config: &NormalizedConfig,
    requirements: &PlatformRequirements,
) -> Result<StaticIntentIR, crate::error::ConfigError> {
    let partitions = config
        .partitions
        .iter()
        .map(partition_intent)
        .collect::<Vec<_>>();

    let ipc = config
        .ipc_channels
        .iter()
        .map(|channel| {
            let shared_bytes =
                compute_ipc_shared_bytes(channel.queue_slots, channel.slot_size_bytes)?;
            Ok(IpcIntent {
                id: channel.id.clone(),
                channel_id: channel.channel_id,
                producer: channel.producer.clone(),
                producer_vm_id: channel.producer_vm_id,
                consumer: channel.consumer.clone(),
                consumer_vm_id: channel.consumer_vm_id,
                queue_slots: channel.queue_slots,
                slot_size_bytes: channel.slot_size_bytes,
                shared_bytes,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let total_guest_bytes =
        config
            .partitions
            .iter()
            .try_fold(ByteSize::new(0), |acc, partition| {
                let sum = acc
                    .bytes()
                    .checked_add(partition.memory_bytes.bytes())
                    .ok_or_else(|| {
                        crate::error::ConfigError::new(
                            crate::error::ConfigErrorKind::Arithmetic,
                            "guest memory sum overflow",
                        )
                    })?;
                Ok(ByteSize::new(sum))
            })?;

    let total_ipc_bytes = ipc.iter().try_fold(ByteSize::new(0), |acc, channel| {
        let sum = acc
            .bytes()
            .checked_add(channel.shared_bytes.bytes())
            .ok_or_else(|| {
                crate::error::ConfigError::new(
                    crate::error::ConfigErrorKind::Arithmetic,
                    "ipc memory sum overflow",
                )
            })?;
        Ok(ByteSize::new(sum))
    })?;

    let mut core_assignments = config
        .partitions
        .iter()
        .map(|partition| (partition.vm_id, partition.physical_cores.clone()))
        .collect::<Vec<_>>();
    core_assignments.sort_by_key(|(vm_id, _)| vm_id.raw());

    let mut pci_devices = Vec::new();
    for partition in &config.partitions {
        for device in &partition.devices {
            pci_devices.push((
                device.bdf,
                partition.vm_id,
                device.kind.as_str().to_string(),
                device.role.map(|role| role.as_str().to_string()),
                device.mmio_guest_phys,
            ));
        }
    }
    pci_devices.sort_by_key(|(bdf, _, _, _, _)| {
        (
            bdf.segment.raw(),
            bdf.bus.raw(),
            bdf.device.raw(),
            bdf.function.raw(),
        )
    });

    let guest_images = config
        .boot
        .guest_images
        .iter()
        .map(|image| GuestImageIntent {
            partition: image.partition.clone(),
            vm_id: image.vm_id,
            path: image.path.clone(),
            sha256: image.sha256.clone(),
        })
        .collect();

    Ok(StaticIntentIR {
        platform_name: config.platform_name.clone(),
        partitions,
        ipc,
        cpu_intent: CpuPlacementIntent {
            min_physical_cores: requirements.min_physical_cores,
            core_assignments,
        },
        memory_intent: MemoryLayoutIntent {
            total_guest_bytes,
            total_ipc_bytes,
            hypervisor_reserve_bytes: hypervisor_reserve_bytes(),
        },
        boot_intent: BootIntent { guest_images },
        pci_intent: PciOwnershipIntent {
            devices: pci_devices,
        },
        qemu_plan: QemuPlanIntent {
            machine: config.qemu.machine.clone(),
            cpus: config.qemu.cpus,
            memory_bytes: config.qemu.memory_bytes,
            cpu_model: config.qemu.cpu_model.clone(),
            accel: config.qemu.accel.clone(),
            smp_sockets: config.qemu.smp_sockets,
            smp_cores: config.qemu.smp_cores,
            smp_threads: config.qemu.smp_threads,
            network: qemu_network_plan_intent(config),
        },
    })
}

fn qemu_network_plan_intent(config: &NormalizedConfig) -> QemuNetworkPlanIntent {
    let mut interfaces = config
        .qemu
        .network
        .interfaces
        .iter()
        .filter_map(|interface| {
            let _vm_id = config
                .partitions
                .iter()
                .find(|partition| partition.id == interface.partition)
                .map(|partition| partition.vm_id)?;
            let bdf = crate::pci::parse_bdf(&interface.bdf).ok()?;
            Some(QemuNetworkInterfaceIntent {
                partition: interface.partition.clone(),
                bdf,
                pci_addr: interface.pci_addr.clone(),
                netdev_id: interface.netdev_id.clone(),
                tap_ifname: interface.tap_ifname.clone(),
            })
        })
        .collect::<Vec<_>>();
    interfaces.sort_by_key(|interface| {
        (
            interface.bdf.segment.raw(),
            interface.bdf.bus.raw(),
            interface.bdf.device.raw(),
            interface.bdf.function.raw(),
        )
    });
    QemuNetworkPlanIntent {
        enabled: config.qemu.network.enabled,
        backend: config.qemu.network.backend.clone(),
        interfaces,
    }
}

fn partition_intent(partition: &NormalizedPartition) -> PartitionIntent {
    PartitionIntent {
        id: partition.id.clone(),
        vm_id: partition.vm_id,
        vcpus: partition.vcpus,
        memory_bytes: partition.memory_bytes,
        physical_cores: partition.physical_cores.clone(),
        devices: partition
            .devices
            .iter()
            .map(|device| PciDeviceIntent {
                kind: device.kind.as_str().to_string(),
                bdf: device.bdf,
                mmio_guest_phys: device.mmio_guest_phys,
                role: device.role.map(|role| role.as_str().to_string()),
            })
            .collect(),
    }
}

fn compute_ipc_shared_bytes(
    queue_slots: u32,
    slot_size_bytes: u32,
) -> Result<ByteSize, crate::error::ConfigError> {
    let per_slot = u64::from(slot_size_bytes) + IPC_SLOT_METADATA_BYTES;
    let total = per_slot
        .checked_mul(u64::from(queue_slots))
        .ok_or_else(|| {
            crate::error::ConfigError::new(
                crate::error::ConfigErrorKind::Arithmetic,
                "ipc shared memory overflow",
            )
        })?;
    Ok(ByteSize::new(total))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::compile_config_from_str;

    #[test]
    fn static_intent_ir_populates_memory_and_pci() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        assert!(compiled.intent.memory_intent.total_guest_bytes.bytes() > 0);
        assert!(compiled.intent.memory_intent.total_ipc_bytes.bytes() > 0);
        assert_eq!(compiled.intent.pci_intent.devices.len(), 2);
    }
}
