//! Coverage-oriented loader handoff tests.

#![allow(clippy::expect_used)]

use hv_loader::{
    build_loader_handoff, encode_empty_acpi_firmware, LoaderErrorKind, LoaderHandoffInput,
};
use hv_platform_model::CpuidSnapshot;
use hv_types::SHA256_DIGEST_BYTES;

fn rsdp_from_firmware(firmware: &hv_loader::FirmwareMemoryImage) -> Vec<u8> {
    firmware.bytes.get(0x1000..0x1000 + 36).expect("rsdp").to_vec()
}

#[test]
fn build_loader_handoff_rejects_invalid_rsdp() {
    let input = LoaderHandoffInput::with_default_descriptor_size(
        [0u8; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        b"INVALID".to_vec(),
        encode_empty_acpi_firmware(),
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
    let firmware = encode_empty_acpi_firmware();
    let mut input = LoaderHandoffInput::with_default_descriptor_size(
        [0u8; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        rsdp_from_firmware(&firmware),
        firmware,
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
    let firmware = encode_empty_acpi_firmware();
    let mut input = LoaderHandoffInput::with_default_descriptor_size(
        [0u8; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        rsdp_from_firmware(&firmware),
        firmware,
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
}

#[test]
fn build_loader_handoff_collects_empty_acpi_tables_from_empty_xsdt() {
    let firmware = encode_empty_acpi_firmware();
    let handoff = build_loader_handoff(&LoaderHandoffInput::with_default_descriptor_size(
        [0xAA; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        rsdp_from_firmware(&firmware),
        firmware,
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
    ))
    .expect("handoff");
    assert!(handoff.observation.acpi_tables.is_empty());
}

#[test]
fn build_loader_handoff_rejects_missing_xsdt_in_firmware() {
    let memory = hv_loader::FirmwareMemoryImage::new(0, vec![0u8; 64]);
    let err = build_loader_handoff(&LoaderHandoffInput::with_default_descriptor_size(
        [0u8; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        hv_boot_abi::AcpiRsdp::encode_reference_v2_with_xsdt(0x5000).to_vec(),
        memory,
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
    ))
    .expect_err("must fail");
    assert_eq!(err.kind, LoaderErrorKind::AcpiWalk);
}

#[test]
fn qemu_reference_firmware_handoff_includes_dmar() {
    use hv_loader::encode_qemu_reference_firmware;
    let firmware = encode_qemu_reference_firmware();
    let handoff = build_loader_handoff(&LoaderHandoffInput::with_default_descriptor_size(
        [0xBB; SHA256_DIGEST_BYTES],
        vec![0u8; 48],
        rsdp_from_firmware(&firmware),
        firmware,
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
    ))
    .expect("handoff");
    assert!(handoff
        .observation
        .acpi_tables
        .windows(4)
        .any(|window| window == b"DMAR"));
}
