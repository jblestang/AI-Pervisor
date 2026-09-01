//! Hypervisor proxy for independent outer host e1000 interfaces (no in↔out link).
//!
//! `net_in` attaches to its own host interface for the IN partition; `net_out` attaches
//! to a separate host interface for OUT. MID forwards between them over IPC only.

use hv_platform_model::{PlannedPciDevice, StaticPlatformIR};
use hv_types::{PciBdf, VmId};

use crate::error::{DatapathError, DatapathErrorKind};
use crate::forward::SYNTHETIC_FRAME_PAYLOAD;
use crate::ipc::{IpcQueueView, REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES};

const HOST_ATTACH_FRAME_LEN: usize = 8;

/// Staging for the host IN interface only (ingress from host tap → IPC).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct HostInAttachState {
    pending_ingress_len: u32,
    pending_ingress: [u8; HOST_ATTACH_FRAME_LEN],
}

/// Staging for the host OUT interface only (IPC → egress to host tap).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct HostOutAttachState {
    pending_egress_len: u32,
    pending_egress: [u8; HOST_ATTACH_FRAME_LEN],
}

/// Hypervisor-owned attach state for both independent host interfaces.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct E1000HostAttachState {
    host_in: HostInAttachState,
    host_out: HostOutAttachState,
}

/// Planned independent host NIC bindings from platform layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E1000HostAttachPlan {
    /// PCI BDF for the IN partition outer e1000.
    pub host_in_bdf: PciBdf,
    /// PCI BDF for the OUT partition outer e1000.
    pub host_out_bdf: PciBdf,
}

/// Builds host attach bindings from static platform PCI intent.
pub fn plan_e1000_host_attach(layout: &StaticPlatformIR) -> Result<E1000HostAttachPlan, DatapathError> {
    Ok(E1000HostAttachPlan {
        host_in_bdf: bdf_for_vm(&layout.pci_devices, VmId::new(0))?,
        host_out_bdf: bdf_for_vm(&layout.pci_devices, VmId::new(2))?,
    })
}

fn bdf_for_vm(devices: &[PlannedPciDevice], vm_id: VmId) -> Result<PciBdf, DatapathError> {
    devices
        .iter()
        .find(|device| device.vm_id == vm_id && device.kind == "nic_e1000")
        .map(|device| device.bdf)
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "missing host NIC device in platform layout",
            )
        })
}

/// Initializes attach state; seeds host IN ingress for smoke when no tap traffic yet.
pub fn initialize_host_attach_state() -> E1000HostAttachState {
    let mut state = E1000HostAttachState::default();
    state
        .host_in
        .pending_ingress
        .copy_from_slice(SYNTHETIC_FRAME_PAYLOAD);
    state.host_in.pending_ingress_len = SYNTHETIC_FRAME_PAYLOAD.len() as u32;
    state
}

/// Forwards a frame received on the host IN interface into chan_a (IN partition IPC).
pub fn host_in_forward_ingress_to_ipc(
    state: &mut E1000HostAttachState,
    chan_a_backing: &mut [u8],
) -> Result<(), DatapathError> {
    if state.host_in.pending_ingress_len as usize != SYNTHETIC_FRAME_PAYLOAD.len() {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "host IN interface has no pending ingress frame",
        ));
    }
    let mut queue = IpcQueueView::open(
        chan_a_backing,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES,
    )?;
    queue.enqueue(&state.host_in.pending_ingress)?;
    state.host_in.pending_ingress_len = 0;
    Ok(())
}

/// Stages a frame from chan_b for egress on the independent host OUT interface.
pub fn host_out_emit_from_ipc(
    state: &mut E1000HostAttachState,
    chan_b_backing: &mut [u8],
) -> Result<(), DatapathError> {
    let mut buffer = [0u8; REFERENCE_IPC_SLOT_SIZE_BYTES as usize];
    let mut queue = IpcQueueView::open(
        chan_b_backing,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES,
    )?;
    let len = queue.dequeue(&mut buffer)?;
    let payload = buffer.get(0..len).ok_or_else(|| {
        DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "host OUT attach dequeue returned invalid bounds",
        )
    })?;
    if payload != SYNTHETIC_FRAME_PAYLOAD {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "host OUT attach expected synthetic frame payload",
        ));
    }
    state.host_out.pending_egress.fill(0);
    state
        .host_out
        .pending_egress
        .copy_from_slice(SYNTHETIC_FRAME_PAYLOAD);
    state.host_out.pending_egress_len = SYNTHETIC_FRAME_PAYLOAD.len() as u32;
    Ok(())
}

const HOST_ATTACH_ENCODED_LEN: usize =
    4 + HOST_ATTACH_FRAME_LEN + 4 + HOST_ATTACH_FRAME_LEN;

/// Encodes attach state into a host page slice.
#[allow(clippy::indexing_slicing)]
pub fn encode_host_attach_state(
    state: &E1000HostAttachState,
    out: &mut [u8],
) -> Result<(), DatapathError> {
    if out.len() < HOST_ATTACH_ENCODED_LEN {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "host attach state buffer too small",
        ));
    }
    out.fill(0);
    out[0..4].copy_from_slice(&state.host_in.pending_ingress_len.to_le_bytes());
    out[4..4 + HOST_ATTACH_FRAME_LEN].copy_from_slice(&state.host_in.pending_ingress);
    let base = 4 + HOST_ATTACH_FRAME_LEN;
    out[base..base + 4].copy_from_slice(&state.host_out.pending_egress_len.to_le_bytes());
    out[base + 4..base + 4 + HOST_ATTACH_FRAME_LEN].copy_from_slice(&state.host_out.pending_egress);
    Ok(())
}

/// Decodes attach state from a host page slice.
#[allow(clippy::indexing_slicing)]
pub fn decode_host_attach_state(bytes: &[u8]) -> Result<E1000HostAttachState, DatapathError> {
    if bytes.len() < HOST_ATTACH_ENCODED_LEN {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "host attach state buffer too small",
        ));
    }
    let base = 4 + HOST_ATTACH_FRAME_LEN;
    let mut pending_ingress = [0u8; HOST_ATTACH_FRAME_LEN];
    pending_ingress.copy_from_slice(
        bytes
            .get(4..4 + HOST_ATTACH_FRAME_LEN)
            .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "ingress unreadable"))?,
    );
    let mut pending_egress = [0u8; HOST_ATTACH_FRAME_LEN];
    pending_egress.copy_from_slice(
        bytes
            .get(base + 4..base + 4 + HOST_ATTACH_FRAME_LEN)
            .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "egress unreadable"))?,
    );
    Ok(E1000HostAttachState {
        host_in: HostInAttachState {
            pending_ingress_len: u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| {
                DatapathError::new(DatapathErrorKind::InvalidInput, "ingress len unreadable")
            })?),
            pending_ingress,
        },
        host_out: HostOutAttachState {
            pending_egress_len: u32::from_le_bytes(
                bytes[base..base + 4].try_into().map_err(|_| {
                    DatapathError::new(DatapathErrorKind::InvalidInput, "egress len unreadable")
                })?,
            ),
            pending_egress,
        },
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn plan_finds_independent_host_in_and_host_out_nics() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_e1000_host_attach(&layout).expect("plan");
        assert_ne!(plan.host_in_bdf, plan.host_out_bdf);
    }

    #[test]
    fn host_in_ingress_and_host_out_egress_are_independent() {
        let mut attach = initialize_host_attach_state();
        let storage = crate::ipc::queue_storage_bytes(
            REFERENCE_IPC_QUEUE_SLOTS,
            REFERENCE_IPC_SLOT_SIZE_BYTES,
        )
        .expect("storage");
        let mut chan_a = vec![0u8; storage];
        let mut chan_b = vec![0u8; storage];

        host_in_forward_ingress_to_ipc(&mut attach, &mut chan_a).expect("host in");
        let mut mid = [0u8; REFERENCE_IPC_SLOT_SIZE_BYTES as usize];
        let mut qa = IpcQueueView::open(
            &mut chan_a,
            REFERENCE_IPC_QUEUE_SLOTS,
            REFERENCE_IPC_SLOT_SIZE_BYTES,
        )
        .expect("qa");
        let len = qa.dequeue(&mut mid).expect("dequeue");
        let mut qb = IpcQueueView::open(
            &mut chan_b,
            REFERENCE_IPC_QUEUE_SLOTS,
            REFERENCE_IPC_SLOT_SIZE_BYTES,
        )
        .expect("qb");
        qb.enqueue(mid.get(0..len).expect("slice")).expect("enqueue b");

        host_out_emit_from_ipc(&mut attach, &mut chan_b).expect("host out");
        assert_eq!(&attach.host_out.pending_egress[..], SYNTHETIC_FRAME_PAYLOAD);
        assert_eq!(attach.host_in.pending_ingress_len, 0);
    }
}
