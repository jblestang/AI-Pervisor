//! IPC shared-memory queue layout and operations.

use hv_config_model::IPC_SLOT_METADATA_BYTES;

use crate::error::{DatapathError, DatapathErrorKind};

/// Fixed queue geometry for the reference `configs/qemu.yaml` topology.
pub const REFERENCE_IPC_QUEUE_SLOTS: u32 = 4096;
/// Fixed slot payload size for the reference `configs/qemu.yaml` topology.
pub const REFERENCE_IPC_SLOT_SIZE_BYTES: u32 = 2048;

/// IPC queue header stored at the base of a shared region.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcQueueHeader {
    /// Producer head index.
    pub head: u32,
    /// Consumer tail index.
    pub tail: u32,
    /// Ring capacity in slots.
    pub queue_slots: u32,
    /// Payload bytes per slot.
    pub slot_size_bytes: u32,
}

/// Per-slot metadata prefix.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcSlotHeader {
    /// Non-zero when the slot contains a valid frame.
    pub valid: u32,
    /// Valid payload length in bytes.
    pub payload_len: u32,
    /// Reserved padding to 64 bytes.
    pub reserved: [u8; 56],
}

/// Mutable view over one IPC queue in host memory.
pub struct IpcQueueView<'a> {
    bytes: &'a mut [u8],
    header: IpcQueueHeader,
}

impl<'a> IpcQueueView<'a> {
    /// Opens a queue view over a shared IPC mapping.
    pub fn open(
        bytes: &'a mut [u8],
        queue_slots: u32,
        slot_size_bytes: u32,
    ) -> Result<Self, DatapathError> {
        let required = queue_storage_bytes(queue_slots, slot_size_bytes)?;
        if bytes.len() < required {
            return Err(DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "ipc shared region smaller than queue layout",
            ));
        }
        let header = read_queue_header(bytes, queue_slots, slot_size_bytes)?;
        Ok(Self { bytes, header })
    }

    /// Returns the channel-local queue header snapshot.
    pub const fn header(&self) -> &IpcQueueHeader {
        &self.header
    }

    /// Enqueues one payload frame as producer.
    pub fn enqueue(&mut self, payload: &[u8]) -> Result<(), DatapathError> {
        if payload.len() > self.header.slot_size_bytes as usize {
            return Err(DatapathError::new(
                DatapathErrorKind::IpcViolation,
                "ipc payload exceeds slot size",
            ));
        }
        let occupancy = self
            .header
            .head
            .checked_sub(self.header.tail)
            .ok_or_else(|| DatapathError::new(DatapathErrorKind::IpcViolation, "ipc head underflow"))?;
        if occupancy >= self.header.queue_slots {
            return Err(DatapathError::new(
                DatapathErrorKind::IpcViolation,
                "ipc queue full",
            ));
        }
        let slot_index = self.header.head % self.header.queue_slots;
        write_slot(self.bytes, &self.header, slot_index, payload)?;
        self.header.head = self
            .header
            .head
            .checked_add(1)
            .ok_or_else(|| DatapathError::new(DatapathErrorKind::IpcViolation, "ipc head overflow"))?;
        write_queue_header(self.bytes, &self.header)?;
        Ok(())
    }

    /// Dequeues one payload frame as consumer.
    pub fn dequeue(&mut self, out: &mut [u8]) -> Result<usize, DatapathError> {
        if self.header.tail >= self.header.head {
            return Err(DatapathError::new(
                DatapathErrorKind::IpcViolation,
                "ipc queue empty",
            ));
        }
        let slot_index = self.header.tail % self.header.queue_slots;
        let payload_len = read_slot(self.bytes, &self.header, slot_index, out)?;
        invalidate_slot(self.bytes, &self.header, slot_index)?;
        self.header.tail = self
            .header
            .tail
            .checked_add(1)
            .ok_or_else(|| DatapathError::new(DatapathErrorKind::IpcViolation, "ipc tail overflow"))?;
        write_queue_header(self.bytes, &self.header)?;
        Ok(payload_len)
    }
}

/// Computes total bytes required for one IPC queue layout.
pub fn queue_storage_bytes(queue_slots: u32, slot_size_bytes: u32) -> Result<usize, DatapathError> {
    let header = core::mem::size_of::<IpcQueueHeader>() as u64;
    let per_slot = u64::from(slot_size_bytes)
        .checked_add(IPC_SLOT_METADATA_BYTES)
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "ipc slot size overflow"))?;
    let slots = u64::from(queue_slots)
        .checked_mul(per_slot)
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "ipc queue size overflow"))?;
    let total = header
        .checked_add(slots)
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::InvalidInput, "ipc storage overflow"))?;
    usize::try_from(total).map_err(|_| {
        DatapathError::new(DatapathErrorKind::InvalidInput, "ipc storage exceeds usize")
    })
}

fn read_queue_header(
    bytes: &[u8],
    queue_slots: u32,
    slot_size_bytes: u32,
) -> Result<IpcQueueHeader, DatapathError> {
    Ok(IpcQueueHeader {
        head: read_u32(bytes, 0)?,
        tail: read_u32(bytes, 4)?,
        queue_slots,
        slot_size_bytes,
    })
}

fn write_queue_header(bytes: &mut [u8], header: &IpcQueueHeader) -> Result<(), DatapathError> {
    write_u32(bytes, 0, header.head)?;
    write_u32(bytes, 4, header.tail)?;
    write_u32(bytes, 8, header.queue_slots)?;
    write_u32(bytes, 12, header.slot_size_bytes)?;
    Ok(())
}

fn write_slot(
    bytes: &mut [u8],
    header: &IpcQueueHeader,
    slot_index: u32,
    payload: &[u8],
) -> Result<(), DatapathError> {
    let offset = slot_offset(header, slot_index)?;
    write_u32(bytes, offset, 1)?;
    write_u32(bytes, offset + 4, payload.len() as u32)?;
    let payload_offset = offset + IPC_SLOT_METADATA_BYTES as usize;
    let end = payload_offset
        .checked_add(payload.len())
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::IpcViolation, "slot write overflow"))?;
    let slice = bytes.get_mut(payload_offset..end).ok_or_else(|| {
        DatapathError::new(DatapathErrorKind::IpcViolation, "slot payload out of bounds")
    })?;
    slice.copy_from_slice(payload);
    Ok(())
}

fn read_slot(
    bytes: &[u8],
    header: &IpcQueueHeader,
    slot_index: u32,
    out: &mut [u8],
) -> Result<usize, DatapathError> {
    let offset = slot_offset(header, slot_index)?;
    let valid = read_u32(bytes, offset)?;
    if valid == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "ipc slot not valid",
        ));
    }
    let payload_len = read_u32(bytes, offset + 4)? as usize;
    if payload_len > header.slot_size_bytes as usize {
        return Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "ipc slot payload length invalid",
        ));
    }
    if out.len() < payload_len {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "ipc dequeue buffer too small",
        ));
    }
    let payload_offset = offset + IPC_SLOT_METADATA_BYTES as usize;
    let end = payload_offset
        .checked_add(payload_len)
        .ok_or_else(|| DatapathError::new(DatapathErrorKind::IpcViolation, "slot read overflow"))?;
    let slice = bytes.get(payload_offset..end).ok_or_else(|| {
        DatapathError::new(DatapathErrorKind::IpcViolation, "slot payload out of bounds")
    })?;
    if let Some(target) = out.get_mut(0..payload_len) {
        target.copy_from_slice(slice);
    }
    Ok(payload_len)
}

fn invalidate_slot(
    bytes: &mut [u8],
    header: &IpcQueueHeader,
    slot_index: u32,
) -> Result<(), DatapathError> {
    let offset = slot_offset(header, slot_index)?;
    write_u32(bytes, offset, 0)?;
    write_u32(bytes, offset + 4, 0)?;
    Ok(())
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
    use alloc::vec;

    #[test]
    fn enqueue_dequeue_round_trip_one_frame() {
        let mut bytes = vec![0u8; queue_storage_bytes(4, 64).expect("storage")];
        let mut queue = IpcQueueView::open(&mut bytes, 4, 64).expect("open");
        queue.enqueue(b"frame-a").expect("enqueue");
        let mut out = [0u8; 64];
        let len = queue.dequeue(&mut out).expect("dequeue");
        assert_eq!(len, 7);
        assert_eq!(out.get(0..len), Some(b"frame-a".as_slice()));
    }

    #[test]
    fn enqueue_rejects_when_queue_full_until_consumer_drains() {
        let mut bytes = vec![0u8; queue_storage_bytes(4, 64).expect("storage")];
        let mut queue = IpcQueueView::open(&mut bytes, 4, 64).expect("open");
        for _ in 0..4 {
            queue.enqueue(b"frame").expect("enqueue");
        }
        assert!(queue.enqueue(b"blocked").is_err());
        let mut out = [0u8; 64];
        queue.dequeue(&mut out).expect("mid drains one slot");
        queue.enqueue(b"after-drain").expect("producer resumes after drain");
    }

    #[test]
    fn ring_buffer_reuses_slots_after_many_frames() {
        let mut bytes = vec![0u8; queue_storage_bytes(4, 64).expect("storage")];
        let mut queue = IpcQueueView::open(&mut bytes, 4, 64).expect("open");
        let mut out = [0u8; 64];
        for _ in 0..256 {
            queue.enqueue(b"frame").expect("enqueue");
            let len = queue.dequeue(&mut out).expect("dequeue");
            assert_eq!(len, 5);
        }
    }
}
