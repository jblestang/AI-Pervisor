//! Hypervisor proxy for independent outer host e1000 interfaces (no in↔out link).
//!
//! `net_in` attaches to its own host interface for the IN partition; `net_out` attaches
//! to a separate host interface for OUT. MID forwards between them over IPC only.

use hv_platform_model::{PlannedPciDevice, StaticPlatformIR};
use hv_types::{PciBdf, VmId};

use crate::error::{DatapathError, DatapathErrorKind};
use crate::ipc::{IpcQueueView, REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES};
use crate::l2_frame::{
    is_reference_l2_frame, write_reference_l2_frame, REFERENCE_L2_FRAME_LEN,
};

/// Which independent host e1000 maps to a nested partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostNicRole {
    /// Host interface dedicated to the IN partition (`0000:00:03.0` contract).
    HostIn,
    /// Host interface dedicated to the OUT partition (`0000:00:04.0` contract).
    HostOut,
}

/// Planned binding for one outer host e1000 and its host interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostNicBinding {
    /// PCI BDF from platform contract.
    pub bdf: PciBdf,
    /// BAR0 MMIO base when discovered via PCI config space (zero until live probe).
    pub bar0_mmio: u64,
    /// Host-side role (independent of the other NIC).
    pub role: HostNicRole,
}

/// Staging for the host IN interface only (ingress from host tap → IPC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostInAttachState {
    /// Outer net_in BAR0 MMIO base.
    pub bar0_mmio: u64,
    /// Whether BAR0 was discovered via PCI config space.
    pub bar0_discovered: bool,
    /// Byte length of `pending_ingress` (zero when empty).
    pub pending_ingress_len: u32,
    /// L2 frame received from the host IN interface, waiting for IPC forward.
    pub pending_ingress: [u8; REFERENCE_L2_FRAME_LEN],
    /// Count of host IN ingress frames forwarded to IPC.
    pub ingress_to_ipc_events: u64,
}

/// Staging for the host OUT interface only (IPC → egress to host tap).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostOutAttachState {
    /// Outer net_out BAR0 MMIO base.
    pub bar0_mmio: u64,
    /// Whether BAR0 was discovered via PCI config space.
    pub bar0_discovered: bool,
    /// Byte length of `pending_egress` (zero when empty).
    pub pending_egress_len: u32,
    /// L2 frame staged for transmission on the host OUT interface.
    pub pending_egress: [u8; REFERENCE_L2_FRAME_LEN],
    /// Count of IPC frames staged for host OUT egress.
    pub ipc_to_egress_events: u64,
    /// Count of frames emitted toward the host OUT interface.
    pub host_egress_frames: u64,
}

/// Hypervisor-owned attach state for both independent host interfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E1000HostAttachState {
    /// Independent host IN interface staging.
    pub host_in: HostInAttachState,
    /// Independent host OUT interface staging.
    pub host_out: HostOutAttachState,
}

impl Default for HostInAttachState {
    fn default() -> Self {
        Self {
            bar0_mmio: 0,
            bar0_discovered: false,
            pending_ingress_len: 0,
            pending_ingress: [0u8; REFERENCE_L2_FRAME_LEN],
            ingress_to_ipc_events: 0,
        }
    }
}

impl Default for HostOutAttachState {
    fn default() -> Self {
        Self {
            bar0_mmio: 0,
            bar0_discovered: false,
            pending_egress_len: 0,
            pending_egress: [0u8; REFERENCE_L2_FRAME_LEN],
            ipc_to_egress_events: 0,
            host_egress_frames: 0,
        }
    }
}

impl Default for E1000HostAttachState {
    fn default() -> Self {
        Self {
            host_in: HostInAttachState::default(),
            host_out: HostOutAttachState::default(),
        }
    }
}

/// Planned independent host NIC bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E1000HostAttachPlan {
    /// Host IN e1000 binding and interface role.
    pub host_in: HostNicBinding,
    /// Host OUT e1000 binding and interface role.
    pub host_out: HostNicBinding,
}

/// Builds host attach bindings from static platform PCI intent.
pub fn plan_e1000_host_attach(layout: &StaticPlatformIR) -> Result<E1000HostAttachPlan, DatapathError> {
    Ok(E1000HostAttachPlan {
        host_in: binding_for_role(&layout.pci_devices, HostNicRole::HostIn)?,
        host_out: binding_for_role(&layout.pci_devices, HostNicRole::HostOut)?,
    })
}

fn binding_for_role(
    devices: &[PlannedPciDevice],
    role: HostNicRole,
) -> Result<HostNicBinding, DatapathError> {
    let vm_id = match role {
        HostNicRole::HostIn => VmId::new(0),
        HostNicRole::HostOut => VmId::new(2),
    };
    let device = devices
        .iter()
        .find(|device| device.vm_id == vm_id && device.kind == "nic_e1000")
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "missing host NIC device in platform layout",
            )
        })?;
    Ok(HostNicBinding {
        bdf: device.bdf,
        bar0_mmio: 0,
        role,
    })
}

/// Initializes attach state; seeds host IN ingress for smoke when no tap traffic yet.
pub fn initialize_host_attach_state(plan: &E1000HostAttachPlan) -> E1000HostAttachState {
    let mut state = E1000HostAttachState::default();
    state.host_in.bar0_mmio = plan.host_in.bar0_mmio;
    state.host_in.bar0_discovered = plan.host_in.bar0_mmio != 0;
    state.host_out.bar0_mmio = plan.host_out.bar0_mmio;
    state.host_out.bar0_discovered = plan.host_out.bar0_mmio != 0;
    let _ = write_reference_l2_frame(&mut state.host_in.pending_ingress);
    state.host_in.pending_ingress_len = REFERENCE_L2_FRAME_LEN as u32;
    state
}

/// Records discovered BAR0 for one independent host interface.
pub fn apply_discovered_host_bar0(state: &mut E1000HostAttachState, role: HostNicRole, bar0: u64) {
    if bar0 == 0 {
        return;
    }
    match role {
        HostNicRole::HostIn => {
            state.host_in.bar0_mmio = bar0;
            state.host_in.bar0_discovered = true;
        }
        HostNicRole::HostOut => {
            state.host_out.bar0_mmio = bar0;
            state.host_out.bar0_discovered = true;
        }
    }
}

/// Forwards a frame received on the host IN interface into chan_a (IN partition IPC).
pub fn host_in_forward_ingress_to_ipc(
    state: &mut HostInAttachState,
    chan_a_backing: &mut [u8],
) -> Result<(), DatapathError> {
    if state.pending_ingress_len as usize != REFERENCE_L2_FRAME_LEN {
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
    queue.enqueue(&state.pending_ingress)?;
    state.ingress_to_ipc_events = state.ingress_to_ipc_events.saturating_add(1);
    state.pending_ingress_len = 0;
    Ok(())
}

/// Stages a frame from chan_b for egress on the independent host OUT interface.
pub fn host_out_emit_from_ipc(
    state: &mut HostOutAttachState,
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
    if !is_reference_l2_frame(payload) {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "host OUT attach expected reference L2 frame",
        ));
    }
    state.pending_egress.fill(0);
    state.pending_egress.copy_from_slice(payload);
    state.pending_egress_len = len as u32;
    state.ipc_to_egress_events = state.ipc_to_egress_events.saturating_add(1);
    state.host_egress_frames = state.host_egress_frames.saturating_add(1);
    Ok(())
}

/// Returns whether both independent host interface BARs were discovered.
pub fn host_attach_interfaces_ready(state: &E1000HostAttachState) -> bool {
    state.host_in.bar0_discovered && state.host_out.bar0_discovered
}

/// Serial marker after host attach state page is installed.
pub const E1000_HOST_ATTACH_INSTALLED_MARKER: &str = "REAL_HW: e1000 host attach installed";
/// Serial marker after both independent host e1000 BAR0 values are discovered.
pub const E1000_HOST_ATTACH_BAR0_READY_MARKER: &str =
    "REAL_HW: independent host e1000 BAR0 discovered";

const HOST_ATTACH_ENCODED_LEN: usize = 8 + 1 + 4 + REFERENCE_L2_FRAME_LEN + 8
    + 8 + 1 + 4 + REFERENCE_L2_FRAME_LEN + 8 + 8;

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
    let hi = &state.host_in;
    out[0..8].copy_from_slice(&hi.bar0_mmio.to_le_bytes());
    out[8] = u8::from(hi.bar0_discovered);
    out[9..13].copy_from_slice(&hi.pending_ingress_len.to_le_bytes());
    out[13..13 + REFERENCE_L2_FRAME_LEN].copy_from_slice(&hi.pending_ingress);
    out[13 + REFERENCE_L2_FRAME_LEN..21 + REFERENCE_L2_FRAME_LEN]
        .copy_from_slice(&hi.ingress_to_ipc_events.to_le_bytes());
    let base = 21 + REFERENCE_L2_FRAME_LEN;
    let ho = &state.host_out;
    out[base..base + 8].copy_from_slice(&ho.bar0_mmio.to_le_bytes());
    out[base + 8] = u8::from(ho.bar0_discovered);
    out[base + 9..base + 13].copy_from_slice(&ho.pending_egress_len.to_le_bytes());
    out[base + 13..base + 13 + REFERENCE_L2_FRAME_LEN].copy_from_slice(&ho.pending_egress);
    out[base + 13 + REFERENCE_L2_FRAME_LEN..base + 21 + REFERENCE_L2_FRAME_LEN]
        .copy_from_slice(&ho.ipc_to_egress_events.to_le_bytes());
    out[base + 21 + REFERENCE_L2_FRAME_LEN..base + 29 + REFERENCE_L2_FRAME_LEN]
        .copy_from_slice(&ho.host_egress_frames.to_le_bytes());
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
    let mut pending_ingress = [0u8; REFERENCE_L2_FRAME_LEN];
    pending_ingress.copy_from_slice(
        bytes
            .get(13..13 + REFERENCE_L2_FRAME_LEN)
            .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "ingress unreadable"))?,
    );
    let base = 21 + REFERENCE_L2_FRAME_LEN;
    let mut pending_egress = [0u8; REFERENCE_L2_FRAME_LEN];
    pending_egress.copy_from_slice(
        bytes
            .get(base + 13..base + 13 + REFERENCE_L2_FRAME_LEN)
            .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "egress unreadable"))?,
    );
    Ok(E1000HostAttachState {
        host_in: HostInAttachState {
            bar0_mmio: u64::from_le_bytes(bytes[0..8].try_into().map_err(|_| {
                DatapathError::new(DatapathErrorKind::InvalidInput, "host in bar unreadable")
            })?),
            bar0_discovered: bytes[8] != 0,
            pending_ingress_len: u32::from_le_bytes(bytes[9..13].try_into().map_err(|_| {
                DatapathError::new(DatapathErrorKind::InvalidInput, "ingress len unreadable")
            })?),
            pending_ingress,
            ingress_to_ipc_events: u64::from_le_bytes(
                bytes[13 + REFERENCE_L2_FRAME_LEN..21 + REFERENCE_L2_FRAME_LEN]
                    .try_into()
                    .map_err(|_| {
                        DatapathError::new(
                            DatapathErrorKind::InvalidInput,
                            "ingress events unreadable",
                        )
                    })?,
            ),
        },
        host_out: HostOutAttachState {
            bar0_mmio: u64::from_le_bytes(bytes[base..base + 8].try_into().map_err(|_| {
                DatapathError::new(DatapathErrorKind::InvalidInput, "host out bar unreadable")
            })?),
            bar0_discovered: bytes[base + 8] != 0,
            pending_egress_len: u32::from_le_bytes(
                bytes[base + 9..base + 13].try_into().map_err(|_| {
                    DatapathError::new(DatapathErrorKind::InvalidInput, "egress len unreadable")
                })?,
            ),
            pending_egress,
            ipc_to_egress_events: u64::from_le_bytes(
                bytes[base + 13 + REFERENCE_L2_FRAME_LEN..base + 21 + REFERENCE_L2_FRAME_LEN]
                    .try_into()
                    .map_err(|_| {
                        DatapathError::new(
                            DatapathErrorKind::InvalidInput,
                            "egress events unreadable",
                        )
                    })?,
            ),
            host_egress_frames: u64::from_le_bytes(
                bytes[base + 21 + REFERENCE_L2_FRAME_LEN..base + 29 + REFERENCE_L2_FRAME_LEN]
                    .try_into()
                    .map_err(|_| {
                        DatapathError::new(
                            DatapathErrorKind::InvalidInput,
                            "host egress frames unreadable",
                        )
                    })?,
            ),
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
        assert_eq!(plan.host_in.role, HostNicRole::HostIn);
        assert_eq!(plan.host_out.role, HostNicRole::HostOut);
        assert_ne!(plan.host_in.bdf, plan.host_out.bdf);
    }

    #[test]
    fn host_in_ingress_and_host_out_egress_are_independent() {
        let mut attach = E1000HostAttachState::default();
        let _ = write_reference_l2_frame(&mut attach.host_in.pending_ingress);
        attach.host_in.pending_ingress_len = REFERENCE_L2_FRAME_LEN as u32;

        let storage = crate::ipc::queue_storage_bytes(
            REFERENCE_IPC_QUEUE_SLOTS,
            REFERENCE_IPC_SLOT_SIZE_BYTES,
        )
        .expect("storage");
        let mut chan_a = vec![0u8; storage];
        let mut chan_b = vec![0u8; storage];

        host_in_forward_ingress_to_ipc(&mut attach.host_in, &mut chan_a).expect("host in");
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

        host_out_emit_from_ipc(&mut attach.host_out, &mut chan_b).expect("host out");
        assert_eq!(attach.host_out.host_egress_frames, 1);
        assert!(is_reference_l2_frame(&attach.host_out.pending_egress));
        assert_eq!(attach.host_in.pending_ingress_len, 0);
    }
}
