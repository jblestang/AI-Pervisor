//! Shared stats page counters (`IF-STATS-*`).
//!
//! Monotonic `u64` updated by the owning guest; readers tolerate torn reads for
//! smoke. Bench sampling: harness takes [`StatsSnapshot`] at t0/t1 (see
//! `docs/HARNESS.md` / ICD `IF-STATS-003`).

use core::sync::atomic::{AtomicU64, Ordering};

/// Byte size of the packed stats page (`IF-STATS-003`).
pub const STATS_PAGE_SIZE: usize = 11 * core::mem::size_of::<u64>();

/// Little-endian `u64` field offsets within the shared stats page (`IF-STATS-003`).
pub mod stats_off {
    pub const WAN_RX: usize = 0;
    pub const WAN_TX: usize = 8;
    pub const LAN_RX: usize = 16;
    pub const LAN_TX: usize = 24;
    pub const RING_FWD_WAN_TO_LAN: usize = 32;
    pub const RING_FWD_LAN_TO_WAN: usize = 40;
    pub const RING_DROP_FULL: usize = 48;
    pub const BYTES_WAN_RX: usize = 56;
    pub const BYTES_WAN_TX: usize = 64;
    pub const BYTES_LAN_RX: usize = 72;
    pub const BYTES_LAN_TX: usize = 80;
}

/// Point-in-time copy of [`Stats`] for harness deltas (`IF-STATS-002`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatsSnapshot {
    pub wan_rx: u64,
    pub wan_tx: u64,
    pub lan_rx: u64,
    pub lan_tx: u64,
    pub ring_fwd_wan_to_lan: u64,
    pub ring_fwd_lan_to_wan: u64,
    pub ring_drop_full: u64,
    pub bytes_wan_rx: u64,
    pub bytes_wan_tx: u64,
    pub bytes_lan_rx: u64,
    pub bytes_lan_tx: u64,
}

impl StatsSnapshot {
    /// Saturating counter deltas (`end - start`).
    pub fn delta(self, start: Self) -> Self {
        Self {
            wan_rx: self.wan_rx.saturating_sub(start.wan_rx),
            wan_tx: self.wan_tx.saturating_sub(start.wan_tx),
            lan_rx: self.lan_rx.saturating_sub(start.lan_rx),
            lan_tx: self.lan_tx.saturating_sub(start.lan_tx),
            ring_fwd_wan_to_lan: self
                .ring_fwd_wan_to_lan
                .saturating_sub(start.ring_fwd_wan_to_lan),
            ring_fwd_lan_to_wan: self
                .ring_fwd_lan_to_wan
                .saturating_sub(start.ring_fwd_lan_to_wan),
            ring_drop_full: self.ring_drop_full.saturating_sub(start.ring_drop_full),
            bytes_wan_rx: self.bytes_wan_rx.saturating_sub(start.bytes_wan_rx),
            bytes_wan_tx: self.bytes_wan_tx.saturating_sub(start.bytes_wan_tx),
            bytes_lan_rx: self.bytes_lan_rx.saturating_sub(start.bytes_lan_rx),
            bytes_lan_tx: self.bytes_lan_tx.saturating_sub(start.bytes_lan_tx),
        }
    }
}

/// Shared statistics region (`IF-STATS-001` / `IF-STATS-003`).
#[derive(Debug)]
#[repr(C)]
pub struct Stats {
    wan_rx: AtomicU64,
    wan_tx: AtomicU64,
    lan_rx: AtomicU64,
    lan_tx: AtomicU64,
    ring_fwd_wan_to_lan: AtomicU64,
    ring_fwd_lan_to_wan: AtomicU64,
    ring_drop_full: AtomicU64,
    bytes_wan_rx: AtomicU64,
    bytes_wan_tx: AtomicU64,
    bytes_lan_rx: AtomicU64,
    bytes_lan_tx: AtomicU64,
}

impl Stats {
    pub const fn new() -> Self {
        Self {
            wan_rx: AtomicU64::new(0),
            wan_tx: AtomicU64::new(0),
            lan_rx: AtomicU64::new(0),
            lan_tx: AtomicU64::new(0),
            ring_fwd_wan_to_lan: AtomicU64::new(0),
            ring_fwd_lan_to_wan: AtomicU64::new(0),
            ring_drop_full: AtomicU64::new(0),
            bytes_wan_rx: AtomicU64::new(0),
            bytes_wan_tx: AtomicU64::new(0),
            bytes_lan_rx: AtomicU64::new(0),
            bytes_lan_tx: AtomicU64::new(0),
        }
    }

    #[inline]
    fn load(a: &AtomicU64) -> u64 {
        a.load(Ordering::Relaxed)
    }

    #[inline]
    fn add(a: &AtomicU64, n: u64) {
        a.fetch_add(n, Ordering::Relaxed);
    }

    pub fn wan_rx(&self) -> u64 {
        Self::load(&self.wan_rx)
    }
    pub fn wan_tx(&self) -> u64 {
        Self::load(&self.wan_tx)
    }
    pub fn lan_rx(&self) -> u64 {
        Self::load(&self.lan_rx)
    }
    pub fn lan_tx(&self) -> u64 {
        Self::load(&self.lan_tx)
    }
    pub fn ring_fwd_wan_to_lan(&self) -> u64 {
        Self::load(&self.ring_fwd_wan_to_lan)
    }
    pub fn ring_fwd_lan_to_wan(&self) -> u64 {
        Self::load(&self.ring_fwd_lan_to_wan)
    }
    pub fn ring_drop_full(&self) -> u64 {
        Self::load(&self.ring_drop_full)
    }
    pub fn bytes_wan_rx(&self) -> u64 {
        Self::load(&self.bytes_wan_rx)
    }
    pub fn bytes_wan_tx(&self) -> u64 {
        Self::load(&self.bytes_wan_tx)
    }
    pub fn bytes_lan_rx(&self) -> u64 {
        Self::load(&self.bytes_lan_rx)
    }
    pub fn bytes_lan_tx(&self) -> u64 {
        Self::load(&self.bytes_lan_tx)
    }

    /// Load all counters for harness sampling (`IF-STATS-002`).
    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            wan_rx: self.wan_rx(),
            wan_tx: self.wan_tx(),
            lan_rx: self.lan_rx(),
            lan_tx: self.lan_tx(),
            ring_fwd_wan_to_lan: self.ring_fwd_wan_to_lan(),
            ring_fwd_lan_to_wan: self.ring_fwd_lan_to_wan(),
            ring_drop_full: self.ring_drop_full(),
            bytes_wan_rx: self.bytes_wan_rx(),
            bytes_wan_tx: self.bytes_wan_tx(),
            bytes_lan_rx: self.bytes_lan_rx(),
            bytes_lan_tx: self.bytes_lan_tx(),
        }
    }

    pub(crate) fn inc_wan_rx(&self, bytes: u64) {
        Self::add(&self.wan_rx, 1);
        Self::add(&self.bytes_wan_rx, bytes);
    }
    pub(crate) fn inc_wan_tx(&self, bytes: u64) {
        Self::add(&self.wan_tx, 1);
        Self::add(&self.bytes_wan_tx, bytes);
    }
    pub(crate) fn inc_lan_rx(&self, bytes: u64) {
        Self::add(&self.lan_rx, 1);
        Self::add(&self.bytes_lan_rx, bytes);
    }
    pub(crate) fn inc_lan_tx(&self, bytes: u64) {
        Self::add(&self.lan_tx, 1);
        Self::add(&self.bytes_lan_tx, bytes);
    }
    pub(crate) fn inc_fwd_wan_to_lan(&self) {
        Self::add(&self.ring_fwd_wan_to_lan, 1);
    }
    pub(crate) fn inc_fwd_lan_to_wan(&self) {
        Self::add(&self.ring_fwd_lan_to_wan, 1);
    }
    pub(crate) fn inc_drop_full(&self) {
        Self::add(&self.ring_drop_full, 1);
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}
