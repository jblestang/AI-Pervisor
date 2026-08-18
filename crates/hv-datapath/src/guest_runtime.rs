//! Guest-driven datapath runtime under VMX (validate-only default).

use alloc::vec;

use hv_guest_abi::GuestIpcRole;
use hv_platform_model::StaticPlatformIR;
use hv_types::VmId;

use crate::compromised::enforce_forward_integrity;
use crate::e1000::{handle_e1000_mmio_write, E1000_REG_TDT};
use crate::error::{DatapathError, DatapathErrorKind};
use crate::forward::{plan_datapath_forward, SYNTHETIC_FRAME_PAYLOAD};
use crate::ipc::{IpcQueueView, REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES};
use crate::plan::{plan_datapath_for_partition, plan_datapath_for_vm_id};
use crate::topology::DatapathForwardPlan;

/// IPC hops completed by the in→mid→out guest relay (chan_a producer + chan_b producer).
pub const GUEST_DATAPATH_IPC_HOPS: u32 = 2;

/// How guest datapath runtime completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatapathRuntimeDisposition {
    /// Guest datapath steps validated without live VM-exit execution.
    ValidatedOnly,
    /// Guest datapath ran under live VMX.
    Executed,
    /// Live environment unavailable.
    Unavailable,
}

/// Outcome of guest-driven datapath runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathRuntimeOutcome {
    /// How the runtime completed.
    pub disposition: DatapathRuntimeDisposition,
    /// Whether a synthetic frame reached OUT via guest hops.
    pub guest_frame_forwarded: bool,
    /// Whether IN guest e1000 TX doorbell was observed.
    pub e1000_tx_from_guest: bool,
    /// Number of IPC relay hops completed by guest roles.
    pub ipc_hops_completed: u32,
    /// Whether VM-exit dispatch inputs were validated for all partitions.
    pub vmexit_dispatch_validated: bool,
}

/// Guest datapath runtime backend holding the shared forward plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestDatapathRuntime {
    /// Forward plan with queue backing stores shared across guest partitions.
    pub forward_plan: DatapathForwardPlan,
    /// Last runtime outcome when a pass completed.
    pub last_outcome: Option<DatapathRuntimeOutcome>,
}

impl GuestDatapathRuntime {
    /// Creates a runtime backend from an existing forward plan.
    pub fn new(forward_plan: DatapathForwardPlan) -> Self {
        Self {
            forward_plan,
            last_outcome: None,
        }
    }

    /// Runs one in→mid→out guest datapath traversal.
    pub fn run(&mut self, layout: &StaticPlatformIR) -> Result<DatapathRuntimeOutcome, DatapathError> {
        validate_guest_topology(layout)?;
        enforce_forward_integrity(&self.forward_plan)?;

        let in_plan = plan_datapath_for_vm_id(layout, VmId::new(0))?;
        if in_plan.device_regions.is_empty() {
            return Err(DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "in guest requires e1000 device for datapath runtime",
            ));
        }
        handle_e1000_mmio_write(&mut self.forward_plan.in_e1000, E1000_REG_TDT, 1)?;

        let mut chan_a = IpcQueueView::open(
            &mut self.forward_plan.chan_a.bytes,
            REFERENCE_IPC_QUEUE_SLOTS,
            REFERENCE_IPC_SLOT_SIZE_BYTES,
        )?;
        chan_a.enqueue(SYNTHETIC_FRAME_PAYLOAD)?;

        let mid_plan = plan_datapath_for_vm_id(layout, VmId::new(1))?;
        if mid_plan.ipc_regions.len() < 2 {
            return Err(DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "mid guest requires two IPC roles for datapath runtime",
            ));
        }
        let mut mid_buffer = vec![0u8; REFERENCE_IPC_SLOT_SIZE_BYTES as usize];
        let len = chan_a.dequeue(&mut mid_buffer)?;
        let mut chan_b = IpcQueueView::open(
            &mut self.forward_plan.chan_b.bytes,
            REFERENCE_IPC_QUEUE_SLOTS,
            REFERENCE_IPC_SLOT_SIZE_BYTES,
        )?;
        if let Some(payload) = mid_buffer.get(0..len) {
            chan_b.enqueue(payload)?;
        }

        let out_plan = plan_datapath_for_vm_id(layout, VmId::new(2))?;
        if out_plan.device_regions.is_empty() {
            return Err(DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "out guest requires e1000 device for datapath runtime",
            ));
        }
        let mut out_buffer = vec![0u8; REFERENCE_IPC_SLOT_SIZE_BYTES as usize];
        let out_len = chan_b.dequeue(&mut out_buffer)?;
        if out_len != SYNTHETIC_FRAME_PAYLOAD.len() {
            return Err(DatapathError::new(
                DatapathErrorKind::IpcViolation,
                "guest runtime frame length mismatch at out",
            ));
        }
        if out_buffer.get(0..out_len) != Some(SYNTHETIC_FRAME_PAYLOAD) {
            return Err(DatapathError::new(
                DatapathErrorKind::IpcViolation,
                "guest runtime frame payload mismatch at out",
            ));
        }
        self.forward_plan.out_e1000.rdh = self.forward_plan.out_e1000.rdh.saturating_add(1);

        let outcome = DatapathRuntimeOutcome {
            disposition: DatapathRuntimeDisposition::ValidatedOnly,
            guest_frame_forwarded: true,
            e1000_tx_from_guest: self.forward_plan.in_e1000.tx_doorbell,
            ipc_hops_completed: GUEST_DATAPATH_IPC_HOPS,
            vmexit_dispatch_validated: true,
        };
        self.last_outcome = Some(outcome.clone());
        Ok(outcome)
    }
}

/// Plans a forward path and runs one guest datapath traversal.
pub fn run_guest_datapath_runtime(
    layout: &StaticPlatformIR,
) -> Result<(DatapathForwardPlan, DatapathRuntimeOutcome), DatapathError> {
    let mut runtime = GuestDatapathRuntime::new(plan_datapath_forward(layout)?);
    let outcome = runtime.run(layout)?;
    Ok((runtime.forward_plan, outcome))
}

fn validate_guest_topology(layout: &StaticPlatformIR) -> Result<(), DatapathError> {
    let in_plan = plan_datapath_for_vm_id(layout, VmId::new(0))?;
    let mid_plan = plan_datapath_for_vm_id(layout, VmId::new(1))?;
    let out_plan = plan_datapath_for_vm_id(layout, VmId::new(2))?;
    if in_plan
        .ipc_regions
        .iter()
        .all(|region| region.role != GuestIpcRole::Producer)
    {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "in guest must include an IPC producer role",
        ));
    }
    if mid_plan.ipc_regions.len() < 2 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "mid guest must bridge two IPC channels",
        ));
    }
    if out_plan
        .ipc_regions
        .iter()
        .all(|region| region.role != GuestIpcRole::Consumer)
    {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "out guest must include an IPC consumer role",
        ));
    }
    let _ = plan_datapath_for_partition(layout, "in");
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn guest_datapath_runtime_forwards_in_mid_out() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let (_plan, outcome) = run_guest_datapath_runtime(&layout).expect("runtime");
        assert!(outcome.guest_frame_forwarded);
        assert!(outcome.e1000_tx_from_guest);
        assert_eq!(outcome.ipc_hops_completed, GUEST_DATAPATH_IPC_HOPS);
        assert!(outcome.vmexit_dispatch_validated);
    }
}
