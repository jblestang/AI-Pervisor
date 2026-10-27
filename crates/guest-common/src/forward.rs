//! WAN/LAN forward datapath over real [`SpscRing`] + [`NicBackend`].
//!
//! - WAN: host NIC RX → WAN→LAN ring; LAN→WAN ring → host NIC TX
//! - LAN: host NIC RX → LAN→WAN ring; WAN→LAN ring → host NIC TX
//!
//! Ring full → drop + [`Stats::ring_drop_full`] (no indefinite block).

use pci_vf::{NicBackend, NicError};
use ring::{FrameDesc, SpscRing};

use crate::stats::Stats;

/// Bump cursor into the shared payload arena for descriptor offsets.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArenaCursor {
    offset: u32,
}

impl ArenaCursor {
    pub const fn new() -> Self {
        Self { offset: 0 }
    }

    /// Allocate `len` bytes in `arena`, wrapping to 0 when needed.
    ///
    /// Returns `(offset, len)` for a [`FrameDesc`], or `None` if `len` exceeds
    /// the arena (cannot place the frame).
    pub fn alloc(&mut self, arena_len: usize, len: u32) -> Option<(u32, u32)> {
        if len == 0 || (len as usize) > arena_len {
            return None;
        }
        if self.offset as usize + len as usize > arena_len {
            self.offset = 0;
        }
        if self.offset as usize + len as usize > arena_len {
            return None;
        }
        let off = self.offset;
        self.offset = self.offset.wrapping_add(len);
        Some((off, len))
    }
}

/// Copy `frame` into the arena and push a descriptor; on ring full, drop.
fn push_frame(
    ring: &mut SpscRing<'_>,
    arena: &mut [u8],
    cursor: &mut ArenaCursor,
    frame: &[u8],
    stats: &Stats,
    on_fwd: impl FnOnce(&Stats),
) -> bool {
    let Some((offset, len)) = cursor.alloc(arena.len(), frame.len() as u32) else {
        stats.inc_drop_full();
        return false;
    };
    let start = offset as usize;
    let end = start + len as usize;
    arena[start..end].copy_from_slice(frame);
    match ring.try_push(FrameDesc { offset, len }) {
        Ok(()) => {
            on_fwd(stats);
            true
        }
        Err(_) => {
            stats.inc_drop_full();
            false
        }
    }
}

fn pop_to_nic(
    ring: &mut SpscRing<'_>,
    arena: &[u8],
    nic: &mut impl NicBackend,
    stats: &Stats,
    on_tx: impl FnOnce(&Stats, u64),
) -> bool {
    let Ok(desc) = ring.try_pop() else {
        return false;
    };
    let start = desc.offset as usize;
    let end = start.saturating_add(desc.len as usize);
    if end > arena.len() {
        // Corrupt / stale descriptor — count as drop, do not send.
        stats.inc_drop_full();
        return false;
    }
    match nic.try_send(&arena[start..end]) {
        Ok(()) => {
            on_tx(stats, desc.len as u64);
            true
        }
        Err(NicError::QueueFull) => {
            // Put-back is not available on SPSC; count drop (producer already advanced).
            stats.inc_drop_full();
            false
        }
        Err(_) => {
            stats.inc_drop_full();
            false
        }
    }
}

fn recv_to_ring(
    nic: &mut impl NicBackend,
    ring: &mut SpscRing<'_>,
    arena: &mut [u8],
    cursor: &mut ArenaCursor,
    scratch: &mut [u8],
    stats: &Stats,
    on_rx: impl FnOnce(&Stats, u64),
    on_fwd: impl FnOnce(&Stats),
) -> bool {
    if !nic.rx_ready() {
        return false;
    }
    let n = match nic.try_recv(scratch) {
        Ok(n) => n,
        Err(NicError::QueueEmpty) => return false,
        Err(NicError::BufferTooSmall) => {
            stats.inc_drop_full();
            return false;
        }
        Err(_) => return false,
    };
    on_rx(stats, n as u64);
    push_frame(ring, arena, cursor, &scratch[..n], stats, on_fwd)
}

/// One WAN poll: NIC→WAN→LAN ring, then LAN→WAN ring→NIC.
///
/// Returns the number of successful forward actions this poll (0..2).
pub fn forward_wan_poll(
    nic: &mut impl NicBackend,
    wan_to_lan: &mut SpscRing<'_>,
    lan_to_wan: &mut SpscRing<'_>,
    arena: &mut [u8],
    cursor: &mut ArenaCursor,
    scratch: &mut [u8],
    stats: &Stats,
) -> usize {
    let mut actions = 0;
    if recv_to_ring(
        nic,
        wan_to_lan,
        arena,
        cursor,
        scratch,
        stats,
        |s, b| s.inc_wan_rx(b),
        |s| s.inc_fwd_wan_to_lan(),
    ) {
        actions += 1;
    }
    if pop_to_nic(lan_to_wan, arena, nic, stats, |s, b| s.inc_wan_tx(b)) {
        actions += 1;
    }
    actions
}

/// One LAN poll: NIC→LAN→WAN ring, then WAN→LAN ring→NIC.
///
/// Returns the number of successful forward actions this poll (0..2).
pub fn forward_lan_poll(
    nic: &mut impl NicBackend,
    wan_to_lan: &mut SpscRing<'_>,
    lan_to_wan: &mut SpscRing<'_>,
    arena: &mut [u8],
    cursor: &mut ArenaCursor,
    scratch: &mut [u8],
    stats: &Stats,
) -> usize {
    let mut actions = 0;
    if recv_to_ring(
        nic,
        lan_to_wan,
        arena,
        cursor,
        scratch,
        stats,
        |s, b| s.inc_lan_rx(b),
        |s| s.inc_fwd_lan_to_wan(),
    ) {
        actions += 1;
    }
    if pop_to_nic(wan_to_lan, arena, nic, stats, |s, b| s.inc_lan_tx(b)) {
        actions += 1;
    }
    actions
}
