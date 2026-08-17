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
}
