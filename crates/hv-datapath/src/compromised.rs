//! Compromised-guest attack simulation and IPC integrity enforcement.

use hv_config_model::IPC_SLOT_METADATA_BYTES;

use crate::e1000::{handle_e1000_mmio_write, E1000_REG_RDH};
use crate::error::{DatapathError, DatapathErrorKind};
use crate::topology::DatapathForwardPlan;
use crate::ipc::{
    queue_storage_bytes, IpcQueueHeader, REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES,
};

/// Which IPC channel a compromised guest action targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcChannelSelector {
    /// chan_a: in → mid.
    ChanA,
    /// chan_b: mid → out.
    ChanB,
}

/// Which partition e1000 state a compromised guest action targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E1000Partition {
    /// IN partition NIC.
    In,
    /// OUT partition NIC.
    Out,
}

/// Host-simulated write performed by a compromised guest partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompromisedGuestAction {
    /// Forges slot metadata with an oversized payload length.
    ForgedSlotMetadata {
        /// IPC channel under attack.
        channel: IpcChannelSelector,
        /// Slot index to corrupt.
        slot_index: u32,
        /// Forged payload length exceeding slot capacity.
        forged_payload_len: u32,
    },
    /// Corrupts queue head/tail counters.
    CorruptHeadTail {
        /// IPC channel under attack.
        channel: IpcChannelSelector,
        /// Forged head value.
        forged_head: u32,
        /// Forged tail value.
        forged_tail: u32,
    },
    /// MID partition corrupts chan_a shared memory (cross-partition violation).
    CrossPartitionCorruptChanA,
    /// Replays a stale slot by marking it valid after consumption.
    StaleSlotReplay {
        /// IPC channel under attack.
        channel: IpcChannelSelector,
        /// Slot index to replay.
        slot_index: u32,
    },
    /// Attempts a read-only e1000 head-register write from a guest partition.
    E1000ReadOnlyWrite {
        /// Partition whose NIC state is targeted.
        partition: E1000Partition,
        /// MMIO register offset.
        offset: u64,
        /// Forged write value.
        value: u64,
    },
}

/// Reference compromised-guest scenarios exercised by Gate D malicious init.
pub const REFERENCE_COMPROMISED_SCENARIOS: &[CompromisedGuestAction] = &[
    CompromisedGuestAction::ForgedSlotMetadata {
        channel: IpcChannelSelector::ChanA,
        slot_index: 0,
        forged_payload_len: REFERENCE_IPC_SLOT_SIZE_BYTES + 1,
    },
    CompromisedGuestAction::CorruptHeadTail {
        channel: IpcChannelSelector::ChanB,
        forged_head: 2,
        forged_tail: 5,
    },
    CompromisedGuestAction::CrossPartitionCorruptChanA,
    CompromisedGuestAction::StaleSlotReplay {
        channel: IpcChannelSelector::ChanA,
        slot_index: 0,
    },
    CompromisedGuestAction::E1000ReadOnlyWrite {
        partition: E1000Partition::Out,
        offset: E1000_REG_RDH,
        value: 1,
    },
];

/// Applies one compromised-guest write simulation to a forward plan.
pub fn apply_compromised_guest_write(
    plan: &mut DatapathForwardPlan,
    action: CompromisedGuestAction,
) -> Result<(), DatapathError> {
    match action {
        CompromisedGuestAction::ForgedSlotMetadata {
            channel,
            slot_index,
            forged_payload_len,
        } => {
            forge_slot_metadata(channel_bytes_mut(plan, channel)?, slot_index, forged_payload_len)?;
        }
        CompromisedGuestAction::CorruptHeadTail {
            channel,
            forged_head,
            forged_tail,
        } => {
            let bytes = channel_bytes_mut(plan, channel)?;
            write_u32(bytes, 0, forged_head)?;
            write_u32(bytes, 4, forged_tail)?;
        }
        CompromisedGuestAction::CrossPartitionCorruptChanA => {
            let bytes = channel_bytes_mut(plan, IpcChannelSelector::ChanA)?;
            if let Some(byte) = bytes.get_mut(16) {
                *byte = 0xDE;
            }
            write_u32(bytes, 0, 9999)?;
        }
        CompromisedGuestAction::StaleSlotReplay { channel, slot_index } => {
            prime_consumed_slot(channel_bytes_mut(plan, channel)?, slot_index)?;
        }
        CompromisedGuestAction::E1000ReadOnlyWrite {
            partition,
            offset,
            value,
        } => {
            let state = match partition {
                E1000Partition::In => &mut plan.in_e1000,
                E1000Partition::Out => &mut plan.out_e1000,
            };
            handle_e1000_mmio_write(state, offset, value)?;
        }
    }
    Ok(())
}

/// Scans one IPC queue region for integrity violations.
pub fn scan_ipc_queue_integrity(
    bytes: &[u8],
    queue_slots: u32,
    slot_size_bytes: u32,
) -> Result<(), DatapathError> {
    let required = queue_storage_bytes(queue_slots, slot_size_bytes)?;
    if bytes.len() < required {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "ipc shared region smaller than queue layout",
        ));
    }
    let header = IpcQueueHeader {
        head: read_u32(bytes, 0)?,
        tail: read_u32(bytes, 4)?,
        queue_slots,
        slot_size_bytes,
    };
    let stored_slots = read_u32(bytes, 8)?;
    let stored_size = read_u32(bytes, 12)?;
    if stored_slots != 0 && stored_slots != queue_slots {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "ipc queue slots geometry mismatch",
        ));
    }
    if stored_size != 0 && stored_size != slot_size_bytes {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "ipc slot size geometry mismatch",
        ));
    }
    if header.tail > header.head {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "ipc queue tail exceeds head",
        ));
    }
    let occupancy = header.head.saturating_sub(header.tail);
    if occupancy > header.queue_slots {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "ipc queue occupancy exceeds capacity",
        ));
    }
    for slot_index in 0..queue_slots {
        let offset = slot_offset(&header, slot_index)?;
        let valid = read_u32(bytes, offset)?;
        if valid == 0 {
            continue;
        }
        if !slot_in_active_ring_window(&header, slot_index) {
            return Err(DatapathError::new(
                DatapathErrorKind::IpcViolation,
                "stale ipc slot marked valid after consumption",
            ));
        }
        let payload_len = read_u32(bytes, offset + 4)?;
        if payload_len > header.slot_size_bytes {
            return Err(DatapathError::new(
                DatapathErrorKind::IpcViolation,
                "ipc slot payload length invalid",
            ));
        }
    }
    Ok(())
}

fn slot_in_active_ring_window(header: &IpcQueueHeader, slot_index: u32) -> bool {
    let occupancy = header.head.saturating_sub(header.tail);
    if occupancy == 0 {
        return false;
    }
    for k in 0..occupancy {
        if (header.tail + k) % header.queue_slots == slot_index {
            return true;
        }
    }
    false
}

/// Enforces IPC integrity on both forward-plan channels before datapath execution.
pub fn enforce_forward_integrity(plan: &DatapathForwardPlan) -> Result<(), DatapathError> {
    scan_ipc_queue_integrity(
        &plan.chan_a.bytes,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES,
    )?;
    scan_ipc_queue_integrity(
        &plan.chan_b.bytes,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES,
    )?;
    Ok(())
}

fn channel_bytes_mut(
    plan: &mut DatapathForwardPlan,
    channel: IpcChannelSelector,
) -> Result<&mut [u8], DatapathError> {
    Ok(match channel {
        IpcChannelSelector::ChanA => plan.chan_a.bytes.as_mut_slice(),
        IpcChannelSelector::ChanB => plan.chan_b.bytes.as_mut_slice(),
    })
}

fn forge_slot_metadata(
    bytes: &mut [u8],
    slot_index: u32,
    forged_payload_len: u32,
) -> Result<(), DatapathError> {
    let header = IpcQueueHeader {
        head: read_u32(bytes, 0)?,
        tail: read_u32(bytes, 4)?,
        queue_slots: REFERENCE_IPC_QUEUE_SLOTS,
        slot_size_bytes: REFERENCE_IPC_SLOT_SIZE_BYTES,
    };
    let offset = slot_offset(&header, slot_index)?;
    write_u32(bytes, offset, 1)?;
    write_u32(bytes, offset + 4, forged_payload_len)?;
    Ok(())
}

fn prime_consumed_slot(bytes: &mut [u8], slot_index: u32) -> Result<(), DatapathError> {
    let mut head = read_u32(bytes, 0)?;
    if head == 0 {
        head = 1;
        write_u32(bytes, 0, head)?;
    }
    write_u32(bytes, 4, head)?;
    forge_slot_metadata(bytes, slot_index, 8)
}

fn slot_offset(header: &IpcQueueHeader, slot_index: u32) -> Result<usize, DatapathError> {
    if slot_index >= header.queue_slots {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "ipc slot index out of range",
        ));
    }
    let header_bytes = core::mem::size_of::<IpcQueueHeader>() as u64;
    let per_slot = u64::from(header.slot_size_bytes)
        .checked_add(IPC_SLOT_METADATA_BYTES)
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::IpcViolation, "slot stride overflow"))?;
    let slot_base = u64::from(slot_index)
        .checked_mul(per_slot)
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::IpcViolation, "slot offset overflow"))?;
    let offset = header_bytes
        .checked_add(slot_base)
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::IpcViolation, "slot address overflow"))?;
    usize::try_from(offset).map_err(|_| {
        DatapathError::new(DatapathErrorKind::IpcViolation, "slot offset exceeds usize")
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DatapathError> {
    let slice = bytes.get(offset..offset + 4).ok_or_else(|| {
        DatapathError::new(DatapathErrorKind::IpcViolation, "ipc u32 read out of bounds")
    })?;
    Ok(u32::from_le_bytes([
        slice.first().copied().unwrap_or(0),
        slice.get(1).copied().unwrap_or(0),
        slice.get(2).copied().unwrap_or(0),
        slice.get(3).copied().unwrap_or(0),
    ]))
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), DatapathError> {
    let slice = bytes.get_mut(offset..offset + 4).ok_or_else(|| {
        DatapathError::new(DatapathErrorKind::IpcViolation, "ipc u32 write out of bounds")
    })?;
    slice.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use crate::forward::plan_datapath_forward;

    #[test]
    fn clean_plan_passes_integrity_scan() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_datapath_forward(&layout).expect("plan");
        enforce_forward_integrity(&plan).expect("clean");
    }

    #[test]
    fn forged_metadata_is_blocked() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let mut plan = plan_datapath_forward(&layout).expect("plan");
        let action = CompromisedGuestAction::ForgedSlotMetadata {
            channel: IpcChannelSelector::ChanA,
            slot_index: 0,
            forged_payload_len: REFERENCE_IPC_SLOT_SIZE_BYTES + 64,
        };
        assert!(crate::forward::is_compromised_action_blocked(&mut plan, action));
    }

    #[test]
    fn ring_buffer_queue_passes_integrity_after_many_forwards() {
        use crate::forward::forward_synthetic_frame;

        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let mut plan = plan_datapath_forward(&layout).expect("plan");
        for _ in 0..512 {
            forward_synthetic_frame(&mut plan).expect("forward");
        }
        enforce_forward_integrity(&plan).expect("ring buffer integrity");
    }
}
