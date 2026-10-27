//! Host tests for guest image load into layout-derived regions (no hardcoded GPAs).

use demo_config::DemoConfig;
use hv::config::FlatHvConfig;
use hv::guest_load::{load_guest_image, load_raw_image, LoadError};
use hv::guest_mem::{build_layout, GuestId};

fn flat_from_example() -> FlatHvConfig {
    let toml = include_str!("../../config/demo.example.toml");
    let demo = DemoConfig::parse_str(toml).expect("example");
    FlatHvConfig::from_demo(&demo)
}

#[test]
fn raw_load_uses_layout_base_as_entry() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).unwrap();
    let region = layout.image_region(GuestId::Wan);
    let mut dest = vec![0u8; region.size as usize];
    let image = b"\x90\x90\x90\xc3"; // nop nop nop ret
    let entry = load_raw_image(&mut dest, region, image).unwrap();
    assert_eq!(entry, region.base);
    assert_eq!(&dest[..image.len()], image);
}

#[test]
fn image_too_large_errors() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).unwrap();
    let region = layout.image_region(GuestId::Lan);
    let mut dest = vec![0u8; 16];
    let image = vec![0u8; 32];
    let err = load_guest_image(&mut dest, region, &image).unwrap_err();
    assert!(matches!(
        err,
        LoadError::ImageTooLarge | LoadError::Truncated
    ));
}

#[test]
fn elf_magic_selects_elf_path() {
    let flat = flat_from_example();
    let layout = build_layout(&flat).unwrap();
    let region = layout.image_region(GuestId::Wan);
    let mut dest = vec![0u8; region.size as usize];
    // Minimal invalid ELF → BadElf (proves magic branch, not raw copy).
    let mut image = vec![0u8; 64];
    image[0..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    let err = load_guest_image(&mut dest, region, &image).unwrap_err();
    assert_eq!(err, LoadError::BadElf);
}
