//! Synthetic in→mid→out datapath forwarding over IPC queues.

use alloc::vec;
use alloc::vec::Vec;

use hv_platform_model::StaticPlatformIR;
use hv_types::VmId;

use crate::e1000::{handle_e1000_mmio_write, E1000MmioState, E1000_REG_TDT};
use crate::error::{DatapathError, DatapathErrorKind};
use crate::ipc::{
    queue_storage_bytes, IpcQueueView, REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES,
};

/// Synthetic frame payload used for host-side datapath smoke.
pub const SYNTHETIC_FRAME_PAYLOAD: &[u8] = b"HVDP18FR";

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

/// Builds a mock forward plan from static platform layout.
pub fn plan_datapath_forward(layout: &StaticPlatformIR) -> Result<DatapathForwardPlan, DatapathError> {
    let chan_a_region = layout
        .ipc_memory
        .first()
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "missing chan_a ipc region"))?;
    let chan_b_region = layout
        .ipc_memory
        .get(1)
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "missing chan_b ipc region"))?;
    let storage = queue_storage_bytes(REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES)?;
    Ok(DatapathForwardPlan {
        chan_a: IpcChannelRuntime {
            bytes: vec![0u8; storage],
            producer_vm_id: chan_a_region.producer_vm_id,
            consumer_vm_id: chan_a_region.consumer_vm_id,
        },
        chan_b: IpcChannelRuntime {
            bytes: vec![0u8; storage],
            producer_vm_id: chan_b_region.producer_vm_id,
            consumer_vm_id: chan_b_region.consumer_vm_id,
        },
        in_e1000: E1000MmioState::default(),
        out_e1000: E1000MmioState::default(),
    })
}

/// Forwards one synthetic frame in→mid→out through IPC queues.
pub fn forward_synthetic_frame(plan: &mut DatapathForwardPlan) -> Result<(), DatapathError> {
    validate_reference_topology(plan)?;
    handle_e1000_mmio_write(&mut plan.in_e1000, E1000_REG_TDT, 1)?;

    let mut chan_a = IpcQueueView::open(
        &mut plan.chan_a.bytes,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES,
    )?;
    chan_a.enqueue(SYNTHETIC_FRAME_PAYLOAD)?;

    let mut mid_buffer = vec![0u8; REFERENCE_IPC_SLOT_SIZE_BYTES as usize];
    let len = chan_a.dequeue(&mut mid_buffer)?;
    let mut chan_b = IpcQueueView::open(
        &mut plan.chan_b.bytes,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES,
    )?;
    if let Some(payload) = mid_buffer.get(0..len) {
        chan_b.enqueue(payload)?;
    }

    let mut out_buffer = vec![0u8; REFERENCE_IPC_SLOT_SIZE_BYTES as usize];
    let out_len = chan_b.dequeue(&mut out_buffer)?;
    if out_len != SYNTHETIC_FRAME_PAYLOAD.len() {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "forwarded frame length mismatch",
        ));
    }
    if out_buffer.get(0..out_len) != Some(SYNTHETIC_FRAME_PAYLOAD) {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "forwarded frame payload mismatch",
        ));
    }
    plan.out_e1000.rdh = plan.out_e1000.rdh.saturating_add(1);
    Ok(())
}

fn validate_reference_topology(plan: &DatapathForwardPlan) -> Result<(), DatapathError> {
    if plan.chan_a.producer_vm_id != VmId::new(0) || plan.chan_a.consumer_vm_id != VmId::new(1) {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "chan_a topology mismatch for reference datapath",
        ));
    }
    if plan.chan_b.producer_vm_id != VmId::new(1) || plan.chan_b.consumer_vm_id != VmId::new(2) {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "chan_b topology mismatch for reference datapath",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn forward_synthetic_frame_traverses_in_mid_out() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let mut plan = plan_datapath_forward(&layout).expect("plan");
        forward_synthetic_frame(&mut plan).expect("forward");
        assert!(plan.in_e1000.tx_doorbell);
    }
}
