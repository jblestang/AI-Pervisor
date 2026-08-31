//! IPC shared-memory queue operations for guest partitions.

use core::mem::size_of;

use crate::layout::{IpcQueueMapping, REFERENCE_SLOT_SIZE};

const REFERENCE_QUEUE_SLOTS: u32 = 4096;
const SLOT_METADATA_BYTES: usize = 64;

#[repr(C)]
struct IpcQueueHeader {
    head: u32,
    tail: u32,
    queue_slots: u32,
    slot_size_bytes: u32,
}

#[repr(C)]
struct IpcSlotHeader {
    valid: u32,
    payload_len: u32,
    reserved: [u8; 56],
}

/// Enqueues one payload frame as producer.
pub fn enqueue(mapping: IpcQueueMapping, payload: &[u8]) {
    if payload.len() > REFERENCE_SLOT_SIZE as usize {
        return;
    }
    let Some(bytes) = mapping_bytes(mapping) else {
        return;
    };
    let Some(header) = read_header(bytes) else {
        return;
    };
    let next = header.head.saturating_add(1);
    if header.head.wrapping_sub(header.tail) >= header.queue_slots {
        return;
    }
    let slot_index = header.head % header.queue_slots;
    let slot_offset = queue_slot_offset(&header, slot_index);
    let Some(slot) = bytes.get_mut(slot_offset..) else {
        return;
    };
    write_slot(slot, payload);
    write_header_head(bytes, next);
}

/// Dequeues one payload frame as consumer, returning the payload length.
pub fn dequeue(mapping: IpcQueueMapping, out: &mut [u8]) -> Option<usize> {
    let bytes = mapping_bytes(mapping)?;
    let header = read_header(bytes)?;
    if header.tail >= header.head {
        return None;
    }
    let slot_index = header.tail % header.queue_slots;
    let slot_offset = queue_slot_offset(&header, slot_index);
    let slot = bytes.get(slot_offset..)?;
    let (valid, len) = read_slot_header(slot)?;
    if valid == 0 {
        return None;
    }
    let len = len as usize;
    if len > out.len() {
        return None;
    }
    let payload_start = size_of::<IpcSlotHeader>();
    let payload = slot.get(payload_start..payload_start + len)?;
    out.get_mut(0..len)?.copy_from_slice(payload);
    invalidate_slot(bytes, slot_offset);
    write_header_tail(bytes, header.tail.saturating_add(1));
    Some(len)
}

fn mapping_bytes(mapping: IpcQueueMapping) -> Option<&'static mut [u8]> {
    if mapping.size == 0 {
        return None;
    }
    Some(unsafe {
        core::slice::from_raw_parts_mut(mapping.guest_phys.raw() as *mut u8, mapping.size as usize)
    })
}

fn read_header(bytes: &[u8]) -> Option<IpcQueueHeader> {
    if bytes.len() < size_of::<IpcQueueHeader>() {
        return None;
    }
    Some(IpcQueueHeader {
        head: read_u32(bytes, 0)?,
        tail: read_u32(bytes, 4)?,
        queue_slots: read_u32(bytes, 8)?,
        slot_size_bytes: read_u32(bytes, 12)?,
    })
}

fn write_header_head(bytes: &mut [u8], head: u32) {
    if let Some(slot) = bytes.get_mut(0..4) {
        slot.copy_from_slice(&head.to_le_bytes());
    }
}

fn write_header_tail(bytes: &mut [u8], tail: u32) {
    if let Some(slot) = bytes.get_mut(4..8) {
        slot.copy_from_slice(&tail.to_le_bytes());
    }
}

fn queue_slot_offset(header: &IpcQueueHeader, slot_index: u32) -> usize {
    size_of::<IpcQueueHeader>() + slot_index as usize * slot_storage_bytes(header)
}

fn slot_storage_bytes(header: &IpcQueueHeader) -> usize {
    SLOT_METADATA_BYTES + header.slot_size_bytes as usize
}

fn write_slot(slot: &mut [u8], payload: &[u8]) {
    if let Some(valid) = slot.get_mut(0..4) {
        valid.copy_from_slice(&1u32.to_le_bytes());
    }
    if let Some(len) = slot.get_mut(4..8) {
        len.copy_from_slice(&(payload.len() as u32).to_le_bytes());
    }
    let payload_start = size_of::<IpcSlotHeader>();
    if let Some(dest) = slot.get_mut(payload_start..payload_start + payload.len()) {
        dest.copy_from_slice(payload);
    }
}

fn read_slot_header(slot: &[u8]) -> Option<(u32, u32)> {
    Some((read_u32(slot, 0)?, read_u32(slot, 4)?))
}

fn invalidate_slot(bytes: &mut [u8], slot_offset: usize) {
    if let Some(valid) = bytes.get_mut(slot_offset..slot_offset + 4) {
        valid.copy_from_slice(&0u32.to_le_bytes());
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[allow(dead_code)]
const REFERENCE_QUEUE_SLOTS_CHECK: u32 = REFERENCE_QUEUE_SLOTS;
