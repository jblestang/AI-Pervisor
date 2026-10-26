//! Memory region / permission model for EPT intent and guest debug asserts.
//!
//! Safe logic is annotated with Verus-oriented `requires!` / `ensures!` stubs
//! (see [`verus_stub`]). Real Verus verification is driven by
//! `cargo xtask verify` when the Verus toolchain is available.
//!
//! Overlapping region inserts are rejected (`IsolationFault::Overlap`) so
//! private WAN/LAN claims cannot coexist in one map (SWR-004).

#![no_std]

extern crate alloc;

mod verus_stub;

use alloc::vec::Vec;

use crate::verus_stub::{ensures, requires};

/// Access rights for a mapped region (read / write / execute).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Perms {
    bits: u8,
}

impl Perms {
    pub const READ: Perms = Perms { bits: 0b001 };
    pub const WRITE: Perms = Perms { bits: 0b010 };
    pub const EXECUTE: Perms = Perms { bits: 0b100 };

    /// No permissions.
    #[inline]
    pub const fn empty() -> Self {
        Perms { bits: 0 }
    }

    /// True if every bit in `need` is set in `self`.
    #[inline]
    pub const fn contains(self, need: Perms) -> bool {
        (self.bits & need.bits) == need.bits
    }

    #[inline]
    pub const fn bits(self) -> u8 {
        self.bits
    }
}

impl core::ops::BitOr for Perms {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Perms {
            bits: self.bits | rhs.bits,
        }
    }
}

impl core::ops::BitOrAssign for Perms {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

/// Contiguous guest-physical (or intent) range with explicit permissions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub base: u64,
    pub size: u64,
    pub perms: Perms,
}

impl Region {
    /// Exclusive end address (`base + size`), or `None` on overflow / zero size.
    #[inline]
    pub fn end(self) -> Option<u64> {
        if self.size == 0 {
            return None;
        }
        self.base.checked_add(self.size)
    }

    /// True if `[addr, addr+len)` is non-empty overflow-safe and fully inside this region.
    #[inline]
    pub fn contains_range(self, addr: u64, len: u64) -> bool {
        requires!(true);
        ensures!(true);

        if len == 0 {
            // Empty access: allow only if `addr` lies in `[base, end)`.
            let end = match self.end() {
                Some(e) => e,
                None => return false,
            };
            return addr >= self.base && addr < end;
        }
        let access_end = match addr.checked_add(len) {
            Some(e) => e,
            None => return false,
        };
        let region_end = match self.end() {
            Some(e) => e,
            None => return false,
        };
        addr >= self.base && access_end <= region_end
    }

    /// Half-open ranges `[base, end)` intersect.
    #[inline]
    fn overlaps(self, other: Region) -> bool {
        let Some(a_end) = self.end() else {
            return false;
        };
        let Some(b_end) = other.end() else {
            return false;
        };
        self.base < b_end && other.base < a_end
    }
}

/// Why an isolation check or map update failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsolationFault {
    /// Address range is outside every mapped region (or overflows).
    OutOfBounds,
    /// Covering region exists but lacks required permissions.
    PermissionDenied,
    /// Insert would overlap an existing region (e.g. private WAN vs LAN).
    Overlap,
    /// Zero-size region or `base + size` overflow.
    InvalidRegion,
}

/// Flat map of non-overlapping regions used when recording EPT intent.
#[derive(Clone, Debug, Default)]
pub struct MemoryMap {
    regions: Vec<Region>,
}

impl MemoryMap {
    #[inline]
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// Registered regions in insertion order.
    #[inline]
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    /// Insert a region. Fails on invalid geometry or any overlap with an existing region.
    pub fn insert(&mut self, region: Region) -> Result<(), IsolationFault> {
        // Verus intent: size > 0 && base + size does not overflow.
        requires!(region.size > 0 && region.base.checked_add(region.size).is_some());
        // Verus intent:
        //   Ok  => regions' contains region and pairwise non-overlapping
        //   Err(Overlap) => exists r in regions with ranges intersecting
        //   Err(InvalidRegion) => size == 0 || overflow
        ensures!(true);

        if region.end().is_none() {
            return Err(IsolationFault::InvalidRegion);
        }
        for existing in &self.regions {
            if existing.overlaps(region) {
                return Err(IsolationFault::Overlap);
            }
        }
        self.regions.push(region);
        Ok(())
    }

    /// Check that `[addr, addr+len)` lies in one region that grants `need`.
    ///
    /// Empty `len` is allowed when `addr` is inside a region that grants `need`.
    pub fn check_access(&self, addr: u64, len: u64, need: Perms) -> Result<(), IsolationFault> {
        // Verus intent: if Ok, exists region covering [addr, addr+len) with perms ⊇ need.
        requires!(true);
        ensures!(true);

        if len > 0 && addr.checked_add(len).is_none() {
            return Err(IsolationFault::OutOfBounds);
        }

        let mut covering: Option<Region> = None;
        for region in &self.regions {
            if region.contains_range(addr, len) {
                covering = Some(*region);
                break;
            }
        }
        let Some(region) = covering else {
            return Err(IsolationFault::OutOfBounds);
        };
        if !region.perms.contains(need) {
            return Err(IsolationFault::PermissionDenied);
        }
        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    pub fn perms_union_and_contains() {
        let rw = Perms::READ | Perms::WRITE;
        assert!(rw.contains(Perms::READ));
        assert!(rw.contains(Perms::WRITE));
        assert!(!rw.contains(Perms::EXECUTE));
        assert!(rw.contains(Perms::READ | Perms::WRITE));
        assert!(!rw.contains(Perms::READ | Perms::EXECUTE));
    }

    #[test]
    fn region_adjacent_does_not_overlap() {
        let a = Region {
            base: 0x1000,
            size: 0x1000,
            perms: Perms::READ,
        };
        let b = Region {
            base: 0x2000,
            size: 0x1000,
            perms: Perms::READ,
        };
        assert!(!a.overlaps(b));
        assert!(!b.overlaps(a));
    }
}
