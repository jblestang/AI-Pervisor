//! Coverage-oriented loader handoff tests.

#![allow(clippy::expect_used)]

use hv_boot_abi::AcpiRsdp;
use hv_loader::{
    build_loader_handoff, LoaderErrorKind, LoaderHandoffInput,
};
use hv_platform_model::CpuidSnapshot;
use hv_types::SHA256_DIGEST_BYTES;

#[test]
fn build_loader_handoff_rejects_invalid_rsdp() {
    let input = LoaderHandoffInput::with_default_descriptor_size(
        [0u8; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        b"INVALID".to_vec(),
        Vec::new(),
        CpuidSnapshot {
            leaf1_ecx: 0,
            leaf1_edx: 0,
            leaf1_ebx: 0,
            leaf80000007_edx: None,
            leaf80000008_ecx: None,
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        Vec::new(),
    );
    let err = build_loader_handoff(&input).expect_err("must fail");
    assert_eq!(err.kind, LoaderErrorKind::BootInfo);
}

#[test]
fn build_loader_handoff_rejects_descriptor_size_below_minimum() {
    let mut input = LoaderHandoffInput::with_default_descriptor_size(
        [0u8; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        AcpiRsdp::encode_reference_v2().to_vec(),
        Vec::new(),
        CpuidSnapshot {
            leaf1_ecx: 0,
            leaf1_edx: 0,
            leaf1_ebx: 0,
            leaf80000007_edx: None,
            leaf80000008_ecx: None,
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        Vec::new(),
    );
    input.memory_descriptor_size = 8;
    let err = build_loader_handoff(&input).expect_err("must fail");
    assert_eq!(err.kind, LoaderErrorKind::Observation);
}

#[test]
fn build_loader_handoff_rejects_zero_descriptor_size() {
    let mut input = LoaderHandoffInput::with_default_descriptor_size(
        [0u8; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        AcpiRsdp::encode_reference_v2().to_vec(),
        Vec::new(),
        CpuidSnapshot {
            leaf1_ecx: 0,
            leaf1_edx: 0,
            leaf1_ebx: 0,
            leaf80000007_edx: None,
            leaf80000008_ecx: None,
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        Vec::new(),
    );
    input.memory_descriptor_size = 0;
    let err = build_loader_handoff(&input).expect_err("must fail");
    assert_eq!(err.kind, LoaderErrorKind::Observation);
    assert!(err.to_string().contains("loader observation error"));
}
