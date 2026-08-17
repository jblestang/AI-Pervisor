//! Coverage-oriented boot ABI parsing tests.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_boot_abi::{
    descriptor_kind, validate_rsdp_section, BootErrorKind, BootInfoView, RSDP_SIGNATURE,
    BOOT_ABI_VERSION, BOOT_INFO_MAGIC,
};
use hv_loader::{build_boot_info_blob, BootInfoSection};
use hv_types::SHA256_DIGEST_BYTES;

#[test]
fn parse_rejects_truncated_boot_info() {
    let err = BootInfoView::parse(&[0u8; 8]).expect_err("must fail");
    assert_eq!(err.kind, BootErrorKind::Parse);
}

#[test]
fn parse_rejects_declared_size_larger_than_buffer() {
    let digest = [0u8; SHA256_DIGEST_BYTES];
    let mut blob = encode_header_only_blob(digest);
    blob[12..16].copy_from_slice(&128u32.to_le_bytes());
    let err = BootInfoView::parse(&blob).expect_err("must fail");
    assert_eq!(err.kind, BootErrorKind::Bounds);
}

#[test]
fn parse_rejects_declared_size_smaller_than_header() {
    let digest = [0u8; SHA256_DIGEST_BYTES];
    let mut blob = encode_header_only_blob(digest);
    blob[12..16].copy_from_slice(&8u32.to_le_bytes());
    let err = BootInfoView::parse(&blob).expect_err("must fail");
    assert_eq!(err.kind, BootErrorKind::Bounds);
}

#[test]
fn boot_info_view_exposes_sections_and_bounded_bytes() {
    let digest = [0xCC; SHA256_DIGEST_BYTES];
    let blob = build_boot_info_blob(
        digest,
        &[
            BootInfoSection {
                kind: descriptor_kind::MEMORY_MAP,
                data: vec![0xAB; 8],
            },
            BootInfoSection {
                kind: descriptor_kind::RSDP,
                data: RSDP_SIGNATURE.to_vec(),
            },
        ],
    )
    .expect("build");

    let view = BootInfoView::parse(&blob).expect("parse");
    assert_eq!(view.descriptor_count(), 2);
    assert_eq!(view.header().version, BOOT_ABI_VERSION);
    assert_eq!(view.bounded_bytes().expect("bounded").len(), blob.len());
    let descriptor = view.descriptor(0).expect("descriptor");
    assert_eq!(descriptor.kind, descriptor_kind::MEMORY_MAP);
    assert_eq!(view.section(&descriptor).expect("section"), &[0xAB; 8]);
    assert!(view
        .memory_map_section()
        .expect("memory map")
        .is_some());
    assert!(view.rsdp_section().expect("rsdp").is_some());
    assert!(view
        .find_descriptor(descriptor_kind::CONFIG)
        .expect("find")
        .is_none());
}

#[test]
fn validate_rsdp_rejects_bad_signature() {
    let err = validate_rsdp_section(b"BAD SIG").expect_err("must fail");
    assert_eq!(err.kind, BootErrorKind::Parse);
}

fn encode_header_only_blob(digest: [u8; SHA256_DIGEST_BYTES]) -> [u8; 56] {
    let mut blob = [0u8; 56];
    blob[0..8].copy_from_slice(&BOOT_INFO_MAGIC);
    blob[8..12].copy_from_slice(&BOOT_ABI_VERSION.to_le_bytes());
    blob[12..16].copy_from_slice(&56u32.to_le_bytes());
    blob[16..48].copy_from_slice(&digest);
    blob[48..52].copy_from_slice(&56u32.to_le_bytes());
    blob[52..56].copy_from_slice(&0u32.to_le_bytes());
    blob
}
