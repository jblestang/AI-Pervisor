//! Synthetic in→mid→out datapath forwarding over IPC queues.

use alloc::vec;

use hv_platform_model::StaticPlatformIR;
use hv_types::VmId;

use crate::compromised::apply_compromised_guest_write;
use crate::compromised::{enforce_forward_integrity, CompromisedGuestAction};
use crate::e1000::{handle_e1000_mmio_write, E1000MmioState, E1000_REG_TDT};
use crate::error::{DatapathError, DatapathErrorKind};
use crate::ipc::{
    queue_storage_bytes, IpcQueueView, REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES,
};
use crate::topology::{DatapathForwardPlan, IpcChannelRuntime};

/// Synthetic frame payload used for host-side datapath smoke.
pub const SYNTHETIC_FRAME_PAYLOAD: &[u8] = b"HVDP18FR";

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

/// Forwards one payload in→mid→out through IPC queues and verifies OUT egress.
pub fn forward_frame_in_mid_out(
    plan: &mut DatapathForwardPlan,
    payload: &[u8],
) -> Result<(), DatapathError> {
    let mut chan_a = IpcQueueView::open(
        &mut plan.chan_a.bytes,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES,
    )?;
    chan_a.enqueue(payload)?;

    let mut mid_buffer = vec![0u8; REFERENCE_IPC_SLOT_SIZE_BYTES as usize];
    let len = chan_a.dequeue(&mut mid_buffer)?;
    let mut chan_b = IpcQueueView::open(
        &mut plan.chan_b.bytes,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES,
    )?;
    if let Some(mid_payload) = mid_buffer.get(0..len) {
        chan_b.enqueue(mid_payload)?;
    }

    let mut out_buffer = vec![0u8; REFERENCE_IPC_SLOT_SIZE_BYTES as usize];
    let out_len = chan_b.dequeue(&mut out_buffer)?;
    if out_len != payload.len() {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "forwarded frame length mismatch",
        ));
    }
    if out_buffer.get(0..out_len) != Some(payload) {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "forwarded frame payload mismatch",
        ));
    }
    plan.out_e1000.rdh = plan.out_e1000.rdh.saturating_add(1);
    Ok(())
}

/// Forwards one synthetic frame in→mid→out through IPC queues.
pub fn forward_synthetic_frame(plan: &mut DatapathForwardPlan) -> Result<(), DatapathError> {
    validate_reference_topology(plan)?;
    enforce_forward_integrity(plan)?;
    handle_e1000_mmio_write(&mut plan.in_e1000, E1000_REG_TDT, 1)?;
    forward_frame_in_mid_out(plan, SYNTHETIC_FRAME_PAYLOAD)
}

/// Returns whether a compromised action is blocked by integrity enforcement or forwarding.
pub fn is_compromised_action_blocked(
    plan: &mut DatapathForwardPlan,
    action: CompromisedGuestAction,
) -> bool {
    if apply_compromised_guest_write(plan, action).is_err() {
        return true;
    }
    if enforce_forward_integrity(plan).is_err() {
        return true;
    }
    forward_synthetic_frame(plan).is_err()
}

/// Runs the reference compromised-guest scenario suite against a fresh forward plan factory.
pub fn run_reference_compromised_scenarios(
    make_plan: impl Fn() -> Result<DatapathForwardPlan, DatapathError>,
) -> Result<(bool, u32), DatapathError> {
    use crate::compromised::REFERENCE_COMPROMISED_SCENARIOS;

    let mut blocked = 0u32;
    for &action in REFERENCE_COMPROMISED_SCENARIOS {
        let mut plan = make_plan()?;
        if is_compromised_action_blocked(&mut plan, action) {
            blocked = blocked.saturating_add(1);
        } else {
            return Err(DatapathError::new(
                DatapathErrorKind::IpcViolation,
                "compromised guest scenario was not blocked",
            ));
        }
    }
    let clean = make_plan()?;
    let integrity_checks_passed = enforce_forward_integrity(&clean).is_ok();
    Ok((integrity_checks_passed, blocked))
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
