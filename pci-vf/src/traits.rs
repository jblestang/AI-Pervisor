//! NicBackend trait and shared error / config types (`IF-NIC-*`).

use core::fmt;

/// Logical datapath backend selection (mirrors `demo_config::VfBackend`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    Sriov,
    Inject,
}

/// Flattened per-NIC open parameters prepared by `hv` / host tools from `DemoConfig`.
///
/// No BDF or lab constants live in this crate — callers pass values from config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlattenedNicConfig<'a> {
    pub kind: BackendKind,
    pub allow_inject_fallback: bool,
    /// PCI BDF string when `kind == Sriov` (e.g. from `datapath.wan_vf_bdf`).
    pub vf_bdf: Option<&'a str>,
    /// Capacity of the Inject RX/TX queues (frames).
    pub inject_queue_capacity: usize,
}

/// Errors from NicBackend open and I/O (`IF-NIC-004`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NicError {
    /// Sriov required but BDF absent/empty in config (SWR-023).
    MissingBdf,
    /// Sriov BDF present but VF not openable / not bound (SWR-023).
    DeviceMissing {
        bdf: alloc::string::String,
    },
    BufferTooSmall,
    QueueFull,
    QueueEmpty,
    IoFailure,
}

impl fmt::Display for NicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NicError::MissingBdf => write!(f, "sriov required but VF BDF missing"),
            NicError::DeviceMissing { bdf } => {
                write!(f, "sriov VF device missing or unavailable: {bdf}")
            }
            NicError::BufferTooSmall => write!(f, "receive buffer too small for frame"),
            NicError::QueueFull => write!(f, "NIC queue full"),
            NicError::QueueEmpty => write!(f, "NIC queue empty"),
            NicError::IoFailure => write!(f, "NIC I/O failure"),
        }
    }
}

impl core::error::Error for NicError {}

/// Unified NIC datapath used by guests (`IF-NIC-001`).
pub trait NicBackend {
    fn rx_ready(&self) -> bool;
    fn try_recv(&mut self, buf: &mut [u8]) -> Result<usize, NicError>;
    fn try_send(&mut self, frame: &[u8]) -> Result<(), NicError>;
}
