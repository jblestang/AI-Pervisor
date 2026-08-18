//! Datapath forward topology types shared by forwarding and integrity modules.

use alloc::vec::Vec;

use hv_types::VmId;

use crate::e1000::E1000MmioState;

/// Runtime IPC channel backing store for mock datapath execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcChannelRuntime {
    /// Shared queue backing bytes.
    pub bytes: Vec<u8>,
    /// Producer VM id.
    pub producer_vm_id: VmId,
    /// Consumer VM id.
    pub consumer_vm_id: VmId,
}

/// Planned in→mid→out forwarding topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathForwardPlan {
    /// chan_a: in → mid.
    pub chan_a: IpcChannelRuntime,
    /// chan_b: mid → out.
    pub chan_b: IpcChannelRuntime,
    /// IN partition e1000 MMIO state.
    pub in_e1000: E1000MmioState,
    /// OUT partition e1000 MMIO state.
    pub out_e1000: E1000MmioState,
}
