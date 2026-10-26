//! Shared stats page counters (`IF-STATS-*`).
//!
//! Monotonic `u64` updated by the owning guest; readers tolerate torn reads for
//! smoke. Bench sampling is harness-defined (Task 9).

use core::sync::atomic::{AtomicU64, Ordering};

/// Shared statistics region (`IF-STATS-001`).
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
