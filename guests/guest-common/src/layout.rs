//! Resolved guest datapath layout for IPC and e1000 MMIO.

use hv_types::GuestPhysAddr;

/// Guest partition role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// IN partition producer + e1000 TX.
    In,
    /// MID partition IPC bridge.
    Mid,
    /// OUT partition consumer + e1000 RX.
    Out,
}

impl Role {
    /// Serial marker emitted at guest start.
    pub const fn running_marker(self) -> &'static str {
        match self {
            Self::In => crate::GUEST_IN_RUNNING_MARKER,
            Self::Mid => crate::GUEST_MID_RUNNING_MARKER,
            Self::Out => crate::GUEST_OUT_RUNNING_MARKER,
        }
    }
}

/// Reference IPC slot payload size for `configs/qemu.yaml`.
pub const REFERENCE_SLOT_SIZE: u32 = 2048;

/// One mapped IPC queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcQueueMapping {
    /// Guest physical base of the shared queue region.
    pub guest_phys: GuestPhysAddr,
    /// Mapping size in bytes.
    pub size: u64,
}

/// Resolved layout used by guest datapath code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedLayout {
    /// Optional e1000 MMIO guest physical base.
    pub e1000_mmio: Option<GuestPhysAddr>,
    /// IPC producer queue mapping when present.
    pub ipc_producer: Option<IpcQueueMapping>,
    /// IPC consumer queue mapping when present.
    pub ipc_consumer: Option<IpcQueueMapping>,
}

impl ResolvedLayout {
    /// Reference layout for IN (vm id 0).
    pub const fn reference_in() -> Self {
        Self {
            e1000_mmio: Some(GuestPhysAddr::new(0xFEB0_0000)),
            ipc_producer: Some(IpcQueueMapping {
                guest_phys: GuestPhysAddr::new(0x4000_0000),
                size: 0x20_00_00,
            }),
            ipc_consumer: None,
        }
    }

    /// Reference layout for MID (vm id 1).
    pub const fn reference_mid() -> Self {
        Self {
            e1000_mmio: None,
            ipc_producer: Some(IpcQueueMapping {
                guest_phys: GuestPhysAddr::new(0x4020_0000),
                size: 0x20_00_00,
            }),
            ipc_consumer: Some(IpcQueueMapping {
                guest_phys: GuestPhysAddr::new(0x4000_0000),
                size: 0x20_00_00,
            }),
        }
    }

    /// Reference layout for OUT (vm id 2).
    pub const fn reference_out() -> Self {
        Self {
            e1000_mmio: Some(GuestPhysAddr::new(0xFEB1_0000)),
            ipc_producer: None,
            ipc_consumer: Some(IpcQueueMapping {
                guest_phys: GuestPhysAddr::new(0x4020_0000),
                size: 0x20_00_00,
            }),
        }
    }
}
