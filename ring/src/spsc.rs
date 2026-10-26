//! Single-producer / single-consumer shared ring over a caller-owned byte arena.
//!
//! # Layout (little-endian)
//!
//! ```text
//! [ RingHeader | FrameDesc × capacity ]
//! ```
//!
//! - `capacity` is a power of two (slot count).
//! - Free-running `write_idx` / `read_idx`; slot = `idx & (capacity - 1)`.
//! - Full when `write_idx.wrapping_sub(read_idx) == capacity`.
//! - Empty when `write_idx == read_idx`.
//!
//! # Concurrency contract (SPSC)
//!
//! Exactly one producer may call [`SpscRing::try_push`]; exactly one consumer may
//! call [`SpscRing::try_pop`]. Indices use atomics with Acquire/Release so a
//! descriptor store is visible before the write cursor advances, and a pop
//! observes the write cursor before reading the slot. Cross-guest mapping must
//! provide coherent shared pages (hv / EPT responsibility).

use core::mem::{align_of, size_of};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::verus_stub::{ensures, requires};

/// Zero-copy frame descriptor into a caller-owned payload arena (`IF-RING-001`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct FrameDesc {
    pub offset: u32,
    pub len: u32,
}

/// Ring is full; producer must drop or retry later (`IF-RING-003`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingFull;

/// Ring is empty; consumer has nothing to pop (`IF-RING-003`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingEmpty;

/// Capacity was not a power of two, was zero, or the arena was too small.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingInitError;

#[repr(C)]
struct RingHeader {
    capacity: u32,
    _pad: u32,
    write_idx: AtomicU32,
    read_idx: AtomicU32,
}

const HEADER_SIZE: usize = size_of::<RingHeader>();

/// Shared SPSC ring view over mutable shared memory (`IF-RING-002` / `IF-RING-003`).
///
/// The lifetime `'a` ties the view to the underlying arena bytes. Producers and
/// consumers in separate address spaces map the same physical pages and construct
/// independent views with the same layout.
pub struct SpscRing<'a> {
    header: &'a RingHeader,
    slots: &'a mut [FrameDesc],
    mask: u32,
}

impl<'a> SpscRing<'a> {
    /// Initialize a new empty ring in `arena` with `capacity` descriptor slots.
    ///
    /// `capacity` must be a non-zero power of two. Arena must hold the header plus
    /// `capacity` tightly packed `FrameDesc` values after header alignment.
    pub fn new_in(arena: &'a mut [u8], capacity: u32) -> Result<Self, RingInitError> {
        // Verus intent: capacity > 0 && is_power_of_two(capacity).
        requires!(capacity > 0 && capacity.is_power_of_two());
        // Verus intent: Ok => header.capacity == capacity && write_idx == 0 &&
        // read_idx == 0 && slots.len() == capacity (layout size does not overflow).
        ensures!(true);

        if capacity == 0 || !capacity.is_power_of_two() {
            return Err(RingInitError);
        }
        let cap = capacity as usize;
        let slots_bytes = cap
            .checked_mul(size_of::<FrameDesc>())
            .ok_or(RingInitError)?;
        let need = HEADER_SIZE
            .checked_add(slots_bytes)
            .ok_or(RingInitError)?;

        let arena_ptr = arena.as_mut_ptr();
        let align = align_of::<RingHeader>();
        let offset = arena_ptr.align_offset(align);
        if offset > arena.len() || arena.len() - offset < need {
            return Err(RingInitError);
        }

        // SAFETY: offset is within arena; remaining length >= need; alignment satisfied.
        let header_ptr = unsafe { arena_ptr.add(offset) as *mut RingHeader };
        let slots_ptr = unsafe { header_ptr.add(1) as *mut FrameDesc };

        // SAFETY: exclusive mutable borrow of arena; we write header then build slices.
        unsafe {
            header_ptr.write(RingHeader {
                capacity,
                _pad: 0,
                write_idx: AtomicU32::new(0),
                read_idx: AtomicU32::new(0),
            });
            for i in 0..cap {
                slots_ptr.add(i).write(FrameDesc { offset: 0, len: 0 });
            }
        }

        let header = unsafe { &*header_ptr };
        let slots = unsafe { core::slice::from_raw_parts_mut(slots_ptr, cap) };

        Ok(Self {
            header,
            slots,
            mask: capacity - 1,
        })
    }

    /// Slot capacity (power of two).
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.header.capacity
    }

    /// Number of descriptors currently enqueued (SPSC-safe for the owning pair).
    #[inline]
    pub fn len(&self) -> u32 {
        let w = self.header.write_idx.load(Ordering::Relaxed);
        let r = self.header.read_idx.load(Ordering::Relaxed);
        w.wrapping_sub(r)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Enqueue a descriptor. Fails with [`RingFull`] without mutating slots when full.
    ///
    /// Caller must be the sole producer. Occupancy is checked before `write_idx`
    /// advances; slot index uses masking (`idx & (capacity - 1)`), so indices never
    /// overflow the slot array.
    pub fn try_push(&mut self, desc: FrameDesc) -> Result<(), RingFull> {
        requires!(self.capacity().is_power_of_two() && self.capacity() > 0);
        // Verus intent:
        //   Ok  => len' == len + 1 && slots[old_write & mask] == desc
        //   Err => len == capacity && slots unchanged
        ensures!(true);

        let cap = self.header.capacity;
        let w = self.header.write_idx.load(Ordering::Relaxed);
        let r = self.header.read_idx.load(Ordering::Acquire);
        if w.wrapping_sub(r) == cap {
            return Err(RingFull);
        }
        let slot = (w & self.mask) as usize;
        self.slots[slot] = desc;
        // Publish descriptor before advancing write cursor.
        self.header
            .write_idx
            .store(w.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Dequeue a descriptor. Fails with [`RingEmpty`] when empty.
    ///
    /// Caller must be the sole consumer.
    pub fn try_pop(&mut self) -> Result<FrameDesc, RingEmpty> {
        requires!(self.capacity().is_power_of_two() && self.capacity() > 0);
        // Verus intent:
        //   Ok  => len' == len - 1
        //   Err => len == 0
        ensures!(true);

        let w = self.header.write_idx.load(Ordering::Acquire);
        let r = self.header.read_idx.load(Ordering::Relaxed);
        if w == r {
            return Err(RingEmpty);
        }
        let slot = (r & self.mask) as usize;
        let desc = self.slots[slot];
        self.header
            .read_idx
            .store(r.wrapping_add(1), Ordering::Release);
        Ok(desc)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn layout_sizes_fit_small_arena() {
        let need = HEADER_SIZE + 2 * size_of::<FrameDesc>() + align_of::<RingHeader>();
        let mut arena = alloc::vec![0u8; need];
        let ring = SpscRing::new_in(&mut arena, 2).expect("init");
        assert_eq!(ring.capacity(), 2);
        assert!(ring.is_empty());
    }
}
