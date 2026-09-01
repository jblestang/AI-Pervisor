//! Static platform intermediate representation with resolved host addresses.

use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use hv_types::{ByteSize, HostPhysAddr, IpcChannelId, PciBdf, VmId};

/// Static platform IR with deterministic host physical layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticPlatformIR {
    /// Platform profile name.
    pub platform_name: String,
    /// Guest private memory regions sorted by VM id.
    pub guest_memory: Vec<PlannedGuestMemory>,
    /// IPC shared memory regions sorted by channel id.
    pub ipc_memory: Vec<PlannedIpcMemory>,
    /// Hypervisor private reserve region.
    pub hypervisor_reserve: PlannedHypervisorReserve,
    /// PCI device ownership with resolved host placement metadata.
    pub pci_devices: Vec<PlannedPciDevice>,
    /// Outer host network interfaces from platform configuration.
    pub host_network: HostNetworkPlan,
}

/// Planned guest private memory region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedGuestMemory {
    /// Partition identifier.
    pub partition_id: String,
    /// Assigned VM identifier.
    pub vm_id: VmId,
    /// Host physical base address.
    pub host_phys: HostPhysAddr,
    /// Region size in bytes.
    pub size: ByteSize,
}

/// Planned IPC shared memory region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedIpcMemory {
    /// Stable channel identifier.
    pub channel_name: String,
    /// Assigned channel identifier.
    pub channel_id: IpcChannelId,
    /// Producer VM id for the channel.
    pub producer_vm_id: VmId,
    /// Consumer VM id for the channel.
    pub consumer_vm_id: VmId,
    /// Host physical base address.
    pub host_phys: HostPhysAddr,
    /// Region size in bytes.
    pub size: ByteSize,
}

/// Planned hypervisor memory reserve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedHypervisorReserve {
    /// Host physical base address.
    pub host_phys: HostPhysAddr,
    /// Region size in bytes.
    pub size: ByteSize,
}

/// Planned PCI device assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedPciDevice {
    /// PCI BDF.
    pub bdf: PciBdf,
    /// Owning VM identifier.
    pub vm_id: VmId,
    /// Device kind string.
    pub kind: String,
    /// Guest physical base for the device MMIO BAR.
    pub mmio_guest_phys: u64,
    /// MMIO window size in bytes.
    pub mmio_size_bytes: u64,
    /// Optional datapath role (`datapath_in`, `datapath_out`, ...).
    pub role: Option<String>,
}

/// One outer host network interface from platform configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostNetworkInterface {
    /// Owning partition id.
    pub partition_id: String,
    /// Assigned VM identifier for the partition.
    pub vm_id: VmId,
    /// PCI BDF for the outer e1000.
    pub bdf: PciBdf,
    /// QEMU PCI slot address (for example `0x3`).
    pub pci_addr: String,
    /// QEMU netdev identifier.
    pub netdev_id: String,
    /// Guest physical MMIO BAR base for the nested e1000 (from partition PCI device).
    pub mmio_guest_phys: u64,
    /// Host tap interface when backend is `tap`.
    pub tap_ifname: Option<String>,
}

/// Host network plan for independent outer QEMU NICs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostNetworkPlan {
    /// Whether host networking is enabled.
    pub enabled: bool,
    /// Netdev backend (`user` or `tap`).
    pub backend: String,
    /// Independent host interface entries sorted by BDF.
    pub interfaces: Vec<HostNetworkInterface>,
}
