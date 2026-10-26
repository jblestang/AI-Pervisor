//! Host-side tests for the isolation region / permission model.
//! Real `Region` / `MemoryMap` structures — no mocks (SWR-004, SWR-044).

use isolation::{IsolationFault, MemoryMap, Perms, Region};

/// Illustrative guest layout (values would come from config / boot blob in hv).
fn wan_private() -> Region {
    Region {
        base: 0x0010_0000,
        size: 0x0010_0000,
        perms: Perms::READ | Perms::WRITE,
    }
}

fn lan_private() -> Region {
    Region {
        base: 0x0020_0000,
        size: 0x0010_0000,
        perms: Perms::READ | Perms::WRITE,
    }
}

fn shared_ring() -> Region {
    Region {
        base: 0x0030_0000,
        size: 0x0001_0000,
        perms: Perms::READ | Perms::WRITE,
    }
}

fn wan_guest_map() -> MemoryMap {
    let mut map = MemoryMap::new();
    map.insert(wan_private()).expect("wan private");
    map.insert(shared_ring()).expect("shared ring");
    map
}

fn lan_guest_map() -> MemoryMap {
    let mut map = MemoryMap::new();
    map.insert(lan_private()).expect("lan private");
    map.insert(shared_ring()).expect("shared ring");
    map
}

#[test]
fn perms_empty_bits_and_bitor_assign() {
    let empty = Perms::empty();
    assert_eq!(empty.bits(), 0);
    let mut p = Perms::READ;
    p |= Perms::WRITE;
    assert!(p.contains(Perms::READ | Perms::WRITE));
    assert_eq!(p.bits(), (Perms::READ | Perms::WRITE).bits());
}

#[test]
fn contains_range_empty_len_and_overflow() {
    let r = Region {
        base: 0x1000,
        size: 0x100,
        perms: Perms::READ,
    };
    assert!(r.contains_range(0x1050, 0));
    assert!(!r.contains_range(0x2000, 0));
    assert!(!r.contains_range(u64::MAX - 8, 64));

    let bad = Region {
        base: u64::MAX - 8,
        size: 64,
        perms: Perms::READ,
    };
    assert!(bad.end().is_none());
    assert!(!bad.contains_range(u64::MAX - 8, 1));
    assert!(!bad.contains_range(u64::MAX - 8, 0));
}

#[test]
fn memory_map_regions_accessor() {
    let mut map = MemoryMap::new();
    map.insert(wan_private()).unwrap();
    assert_eq!(map.regions().len(), 1);
    assert_eq!(map.regions()[0].base, wan_private().base);
}

#[test]
fn wan_allows_own_private_and_shared_ring() {
    let map = wan_guest_map();
    map.check_access(0x0010_0000, 64, Perms::READ | Perms::WRITE)
        .expect("WAN private R/W");
    map.check_access(0x0030_0100, 128, Perms::READ)
        .expect("shared ring read");
    map.check_access(0x0030_0200, 64, Perms::WRITE)
        .expect("shared ring write");
}

#[test]
fn wan_denies_lan_private_region() {
    let map = wan_guest_map();
    let err = map
        .check_access(0x0020_0000, 64, Perms::READ)
        .expect_err("WAN must not see LAN private");
    assert_eq!(err, IsolationFault::OutOfBounds);
}

#[test]
fn lan_allows_own_private_and_shared_ring() {
    let map = lan_guest_map();
    map.check_access(0x0020_8000, 64, Perms::READ | Perms::WRITE)
        .expect("LAN private R/W");
    map.check_access(0x0030_0000, 64, Perms::WRITE)
        .expect("shared ring write");
}

#[test]
fn lan_denies_wan_private_region() {
    let map = lan_guest_map();
    let err = map
        .check_access(0x0010_1000, 32, Perms::READ)
        .expect_err("LAN must not see WAN private");
    assert_eq!(err, IsolationFault::OutOfBounds);
}

#[test]
fn denies_execute_on_rw_only_region() {
    let map = wan_guest_map();
    let err = map
        .check_access(0x0010_0000, 16, Perms::EXECUTE)
        .expect_err("RX not granted on R/W region");
    assert_eq!(err, IsolationFault::PermissionDenied);
}

#[test]
fn denies_access_past_region_end() {
    let map = wan_guest_map();
    // Last byte of WAN private is 0x001F_FFFF; spanning past end must fail.
    let err = map
        .check_access(0x001F_FFC0, 128, Perms::READ)
        .expect_err("must not span past region end");
    assert_eq!(err, IsolationFault::OutOfBounds);
}

#[test]
fn insert_rejects_overlapping_private_wan_lan_claims() {
    let mut map = MemoryMap::new();
    map.insert(wan_private()).unwrap();
    // Overlapping LAN claim into WAN private range (SWR-004).
    let overlapping = Region {
        base: 0x0018_0000,
        size: 0x0010_0000,
        perms: Perms::READ | Perms::WRITE,
    };
    let err = map
        .insert(overlapping)
        .expect_err("overlapping private claims must be rejected");
    assert_eq!(err, IsolationFault::Overlap);
}

#[test]
fn insert_rejects_region_overlapping_shared_ring() {
    let mut map = MemoryMap::new();
    map.insert(shared_ring()).unwrap();
    let err = map
        .insert(Region {
            base: 0x002F_F000,
            size: 0x0002_0000,
            perms: Perms::READ,
        })
        .expect_err("overlap with shared ring");
    assert_eq!(err, IsolationFault::Overlap);
}

#[test]
fn insert_rejects_zero_size_and_overflow() {
    let mut map = MemoryMap::new();
    assert_eq!(
        map.insert(Region {
            base: 0x1000,
            size: 0,
            perms: Perms::READ,
        }),
        Err(IsolationFault::InvalidRegion)
    );
    assert_eq!(
        map.insert(Region {
            base: u64::MAX - 8,
            size: 16,
            perms: Perms::READ,
        }),
        Err(IsolationFault::InvalidRegion)
    );
}

#[test]
fn check_access_rejects_len_overflow() {
    let map = wan_guest_map();
    let err = map
        .check_access(u64::MAX - 4, 16, Perms::READ)
        .expect_err("addr+len overflow");
    assert_eq!(err, IsolationFault::OutOfBounds);
}

#[test]
fn adjacent_non_overlapping_regions_ok() {
    let mut map = MemoryMap::new();
    map.insert(wan_private()).unwrap();
    // LAN starts exactly where WAN ends — no overlap.
    map.insert(lan_private()).unwrap();
    map.insert(shared_ring()).unwrap();
    map.check_access(0x001F_FFC0, 64, Perms::READ).unwrap();
    map.check_access(0x0020_0000, 64, Perms::READ).unwrap();
}
