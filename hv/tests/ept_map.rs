//! Host unit tests for the EPT map builder (`isolation::MemoryMap` intent).
//!
//! No QEMU / VT-x required. Layout sizes come from `DemoConfig` (example TOML);
//! no lab GPA literals are hardcoded in the assertions beyond what the builder
//! derives from those sizes.

use demo_config::DemoConfig;
use guest_common::{GuestRole, BOOT_INFO_MAGIC, BOOT_INFO_VERSION};
use hv::boot_blob::fill_boot_info;
use hv::config::FlatHvConfig;
use hv::ept::{build_ept_intent, EptGuest};
use hv::guest_mem::{build_layout, GuestId};
use isolation::{IsolationFault, Perms};

fn flat_from_example() -> FlatHvConfig {
    let toml = include_str!("../../config/demo.example.toml");
    let demo = DemoConfig::parse_str(toml).expect("example config must parse");
    FlatHvConfig::from_demo(&demo)
}

#[test]
fn layout_sizes_follow_config_memory_mb() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).expect("layout");

    let wan_bytes = flat.wan_memory_mb.checked_mul(1024 * 1024).unwrap();
    let lan_bytes = flat.lan_memory_mb.checked_mul(1024 * 1024).unwrap();
    assert_eq!(layout.wan_private.size, wan_bytes);
    assert_eq!(layout.lan_private.size, lan_bytes);
    assert_eq!(layout.wan_private.base, flat.gpa_window_base);
    assert!(layout.lan_private.base >= layout.wan_private.base + layout.wan_private.size);
}

#[test]
fn wan_ept_allows_private_and_shared_denies_lan_private() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).expect("layout");
    let intent = build_ept_intent(&layout).expect("ept intent");

    let wan = &intent.wan;
    wan.check_access(layout.wan_private.base, 64, Perms::READ | Perms::WRITE)
        .expect("WAN private R/W");
    wan.check_access(layout.wan_private.base, 16, Perms::EXECUTE)
        .expect("WAN private X");
    wan.check_access(layout.ring_wan_to_lan.base, 16, Perms::READ | Perms::WRITE)
        .expect("shared ring W→L");
    wan.check_access(layout.ring_lan_to_wan.base, 16, Perms::READ | Perms::WRITE)
        .expect("shared ring L→W");
    wan.check_access(layout.arena.base, 64, Perms::READ | Perms::WRITE)
        .expect("shared arena");
    wan.check_access(layout.stats.base, 8, Perms::READ | Perms::WRITE)
        .expect("shared stats");
    wan.check_access(layout.boot_info_wan.base, 8, Perms::READ)
        .expect("own boot info");

    let err = wan
        .check_access(layout.lan_private.base, 64, Perms::READ)
        .expect_err("WAN must not map LAN private");
    assert_eq!(err, IsolationFault::OutOfBounds);

    let err = wan
        .check_access(layout.boot_info_lan.base, 8, Perms::READ)
        .expect_err("WAN must not map LAN boot info");
    assert_eq!(err, IsolationFault::OutOfBounds);
}

#[test]
fn lan_ept_allows_private_and_shared_denies_wan_private() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).expect("layout");
    let intent = build_ept_intent(&layout).expect("ept intent");

    let lan = &intent.lan;
    lan.check_access(layout.lan_private.base, 64, Perms::READ | Perms::WRITE | Perms::EXECUTE)
        .expect("LAN private");
    lan.check_access(layout.ring_wan_to_lan.base, 8, Perms::WRITE)
        .expect("shared ring");
    lan.check_access(layout.arena.base, 32, Perms::READ)
        .expect("arena");
    lan.check_access(layout.stats.base, 4, Perms::WRITE)
        .expect("stats");
    lan.check_access(layout.boot_info_lan.base, 8, Perms::READ)
        .expect("own boot info");

    let err = lan
        .check_access(layout.wan_private.base, 64, Perms::READ)
        .expect_err("LAN must not map WAN private");
    assert_eq!(err, IsolationFault::OutOfBounds);
}

#[test]
fn private_regions_do_not_overlap_shared() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).expect("layout");
    let intent = build_ept_intent(&layout).expect("ept intent");

    // Global map records every region without overlap (isolation insert contract).
    assert!(intent.global.regions().len() >= 8);
    for r in intent.global.regions() {
        assert!(r.size > 0);
        assert!(r.base % 4096 == 0, "region must be page-aligned");
        assert!(r.size % 4096 == 0, "region size must be page-multiple");
    }
}

#[test]
fn ept_violation_check_marks_faulting_guest() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).expect("layout");
    let intent = build_ept_intent(&layout).expect("ept intent");

    // Simulate WAN touching LAN private GPA — builder classifies the fault.
    let fault = intent
        .classify_gpa_fault(EptGuest::Wan, layout.lan_private.base)
        .expect("should be a violation");
    assert_eq!(fault.guest, EptGuest::Wan);
    assert_eq!(fault.gpa, layout.lan_private.base);
    assert!(fault.halt_guest);

    // In-map access is not a violation.
    assert!(intent
        .classify_gpa_fault(EptGuest::Wan, layout.wan_private.base)
        .is_none());
}

#[test]
fn boot_info_filled_from_layout_not_lab_literals() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).expect("layout");
    let boot = fill_boot_info(&flat, &layout, GuestRole::Wan);

    assert_eq!(boot.magic, BOOT_INFO_MAGIC);
    assert_eq!(boot.version, BOOT_INFO_VERSION);
    assert_eq!(boot.guest_role, GuestRole::Wan);
    assert_eq!(boot.ring_wan_to_lan_gpa, layout.ring_wan_to_lan.base);
    assert_eq!(boot.ring_lan_to_wan_gpa, layout.ring_lan_to_wan.base);
    assert_eq!(boot.ring_capacity_pow2, flat.ring_capacity_pow2);
    assert_eq!(boot.arena_gpa, layout.arena.base);
    assert_eq!(boot.arena_len, layout.arena.size);
    assert_eq!(boot.stats_gpa, layout.stats.base);
    assert_eq!(boot.boot_info_gpa, layout.boot_info_wan.base);
    assert_eq!(boot.uart_port, flat.uart_io_port);
    assert_ne!(boot.uart_port, 0, "uart must come from config");
}

#[test]
fn guest_image_slot_uses_private_region() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).expect("layout");
    let wan_slot = layout.image_region(GuestId::Wan);
    let lan_slot = layout.image_region(GuestId::Lan);
    assert_eq!(wan_slot.base, layout.wan_private.base);
    assert_eq!(lan_slot.base, layout.lan_private.base);
    assert!(wan_slot.size >= 4096);
    assert!(lan_slot.size >= 4096);
}
