//! EPT *intent* builder: `isolation::MemoryMap` per guest from [`GuestLayout`].
//!
//! Hardware EPT walks stay in the TCB (`vmx` / table load). This module is the
//! host-testable policy: WAN never maps LAN private, shared ring pages are RW
//! to both, boot-info is R to the owning guest only.

use isolation::{IsolationFault, MemoryMap, Perms, Region};

use crate::guest_mem::{GpaRegion, GuestLayout};

/// Which guest's EPT is being consulted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EptGuest {
    Wan,
    Lan,
}

/// Result of classifying an EPT-miss GPA against intent maps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EptFault {
    pub guest: EptGuest,
    pub gpa: u64,
    /// Spike policy: faulting guest is halted; peer may continue.
    pub halt_guest: bool,
}

/// Dual-guest EPT intent derived from layout.
#[derive(Clone, Debug)]
pub struct EptIntent {
    pub global: MemoryMap,
    pub wan: MemoryMap,
    pub lan: MemoryMap,
}

impl EptIntent {
    /// If `gpa` is not present (with any access) in `guest`'s map, return a halt fault.
    pub fn classify_gpa_fault(&self, guest: EptGuest, gpa: u64) -> Option<EptFault> {
        let map = match guest {
            EptGuest::Wan => &self.wan,
            EptGuest::Lan => &self.lan,
        };
        match map.check_access(gpa, 1, Perms::READ) {
            Ok(()) => None,
            Err(IsolationFault::PermissionDenied) => {
                // Present but wrong perms still exits — treat as halt for spike.
                Some(EptFault {
                    guest,
                    gpa,
                    halt_guest: true,
                })
            }
            Err(_) => Some(EptFault {
                guest,
                gpa,
                halt_guest: true,
            }),
        }
    }

    pub fn map_for(&self, guest: EptGuest) -> &MemoryMap {
        match guest {
            EptGuest::Wan => &self.wan,
            EptGuest::Lan => &self.lan,
        }
    }
}

fn region(gpa: GpaRegion, perms: Perms) -> Region {
    Region {
        base: gpa.base,
        size: gpa.size,
        perms,
    }
}

const RW: fn() -> Perms = || Perms::READ | Perms::WRITE;
const RWX: fn() -> Perms = || Perms::READ | Perms::WRITE | Perms::EXECUTE;
const R: fn() -> Perms = || Perms::READ;

fn insert(map: &mut MemoryMap, gpa: GpaRegion, perms: Perms) -> Result<(), IsolationFault> {
    map.insert(region(gpa, perms))
}

/// Build global + per-guest `MemoryMap`s encoding EPT isolation policy.
pub fn build_ept_intent(layout: &GuestLayout) -> Result<EptIntent, IsolationFault> {
    let mut global = MemoryMap::new();
    insert(&mut global, layout.wan_private, RWX())?;
    insert(&mut global, layout.lan_private, RWX())?;
    insert(&mut global, layout.ring_wan_to_lan, RW())?;
    insert(&mut global, layout.ring_lan_to_wan, RW())?;
    insert(&mut global, layout.arena, RW())?;
    insert(&mut global, layout.stats, RW())?;
    insert(&mut global, layout.boot_info_wan, R())?;
    insert(&mut global, layout.boot_info_lan, R())?;

    let mut wan = MemoryMap::new();
    insert(&mut wan, layout.wan_private, RWX())?;
    insert(&mut wan, layout.ring_wan_to_lan, RW())?;
    insert(&mut wan, layout.ring_lan_to_wan, RW())?;
    insert(&mut wan, layout.arena, RW())?;
    insert(&mut wan, layout.stats, RW())?;
    insert(&mut wan, layout.boot_info_wan, R())?;

    let mut lan = MemoryMap::new();
    insert(&mut lan, layout.lan_private, RWX())?;
    insert(&mut lan, layout.ring_wan_to_lan, RW())?;
    insert(&mut lan, layout.ring_lan_to_wan, RW())?;
    insert(&mut lan, layout.arena, RW())?;
    insert(&mut lan, layout.stats, RW())?;
    insert(&mut lan, layout.boot_info_lan, R())?;

    Ok(EptIntent { global, wan, lan })
}

/// Software EPT leaf permission bits (Intel SDM style: R/W/X in low 3 bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EptLeafPerms {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl EptLeafPerms {
    pub fn from_isolation(p: Perms) -> Self {
        Self {
            read: p.contains(Perms::READ),
            write: p.contains(Perms::WRITE),
            execute: p.contains(Perms::EXECUTE),
        }
    }

    /// Pack into EPT PTE low bits (bit0=R, bit1=W, bit2=X).
    pub fn as_ept_bits(self) -> u64 {
        let mut b = 0u64;
        if self.read {
            b |= 1 << 0;
        }
        if self.write {
            b |= 1 << 1;
        }
        if self.execute {
            b |= 1 << 2;
        }
        b
    }
}

/// Describe one identity-mapped leaf the hardware table builder should install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EptMapEntry {
    pub gpa: u64,
    pub size: u64,
    pub perms: EptLeafPerms,
}

/// Flatten a guest `MemoryMap` into page-granular map entries (host-testable).
pub fn map_entries(map: &MemoryMap) -> Result<alloc::vec::Vec<EptMapEntry>, IsolationFault> {
    let mut out = alloc::vec::Vec::new();
    for r in map.regions() {
        if r.size == 0 || r.base % crate::PAGE_SIZE != 0 || r.size % crate::PAGE_SIZE != 0 {
            return Err(IsolationFault::InvalidRegion);
        }
        out.push(EptMapEntry {
            gpa: r.base,
            size: r.size,
            perms: EptLeafPerms::from_isolation(r.perms),
        });
    }
    Ok(out)
}
