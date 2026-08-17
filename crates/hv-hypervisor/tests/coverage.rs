//! Coverage-oriented hypervisor boot-path tests.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::compile_config_from_str;
use hv_hypervisor::{boot_check, BootCheckError, BootCheckErrorKind};
use hv_loader::{build_loader_handoff, encode_empty_acpi_firmware, LoaderHandoffInput};
use hv_platform_model::{
    observe_platform, CpuidSnapshot, ObservationInputs, PlatformError, PlatformErrorKind,
    CPUID_1_ECX_VMX_BIT,
};
use hv_types::SHA256_DIGEST_BYTES;

fn rsdp_from_firmware(firmware: &hv_loader::FirmwareMemoryImage) -> Vec<u8> {
    firmware.bytes.get(0x1000..0x1000 + 36).expect("rsdp").to_vec()
}

#[test]
fn boot_check_rejects_digest_mismatch() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let firmware = encode_empty_acpi_firmware();
    let handoff = build_loader_handoff(&LoaderHandoffInput::with_default_descriptor_size(
        compiled.digest.bytes,
        vec![0u8; 48],
        rsdp_from_firmware(&firmware),
        firmware,
        CpuidSnapshot {
            leaf1_ecx: 1 << CPUID_1_ECX_VMX_BIT,
            leaf1_edx: 0,
            leaf1_ebx: 1 << 16,
            leaf80000007_edx: None,
            leaf80000008_ecx: None,
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        Vec::new(),
    ))
    .expect("handoff");
    let bad = [0xFF; SHA256_DIGEST_BYTES];
    let err = boot_check(
        &handoff.boot_info_blob,
        &bad,
        &compiled.requirements,
        &handoff.observation,
    )
    .expect_err("must fail");
    assert_eq!(err.kind, BootCheckErrorKind::BootAbi);
}

#[test]
fn boot_check_rejects_invalid_boot_info() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let err = boot_check(
        b"not-a-boot-info-blob",
        &compiled.digest.bytes,
        &compiled.requirements,
        &ObservationInputs {
            cpuid: CpuidSnapshot {
                leaf1_ecx: 0,
                leaf1_edx: 0,
                leaf1_ebx: 0,
                leaf80000007_edx: None,
                leaf80000008_ecx: None,
                leaf480_ecx: None,
                leaf480_ebx: None,
            },
            acpi_tables: Vec::new(),
            memory_map: Vec::new(),
            memory_descriptor_size: 48,
            pci_devices: Vec::new(),
        },
    )
    .expect_err("must fail");
    assert_eq!(err.kind, BootCheckErrorKind::BootAbi);
}

#[test]
fn boot_check_maps_observation_errors() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let firmware = encode_empty_acpi_firmware();
    let handoff = build_loader_handoff(&LoaderHandoffInput::with_default_descriptor_size(
        compiled.digest.bytes,
        vec![0u8; 48],
        rsdp_from_firmware(&firmware),
        firmware,
        CpuidSnapshot {
            leaf1_ecx: 1 << CPUID_1_ECX_VMX_BIT,
            leaf1_edx: 0,
            leaf1_ebx: 1 << 16,
            leaf80000007_edx: None,
            leaf80000008_ecx: None,
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        Vec::new(),
    ))
    .expect("handoff");
    let mut observation = handoff.observation.clone();
    observation.memory_descriptor_size = 8;
    let err = boot_check(
        &handoff.boot_info_blob,
        &compiled.digest.bytes,
        &compiled.requirements,
        &observation,
    )
    .expect_err("must fail");
    assert_eq!(err.kind, BootCheckErrorKind::Observation);
}

#[test]
fn boot_check_rejects_memory_map_mismatch() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let firmware = encode_empty_acpi_firmware();
    let handoff = build_loader_handoff(&LoaderHandoffInput::with_default_descriptor_size(
        compiled.digest.bytes,
        vec![0u8; 48],
        rsdp_from_firmware(&firmware),
        firmware,
        CpuidSnapshot {
            leaf1_ecx: 1 << CPUID_1_ECX_VMX_BIT,
            leaf1_edx: 0,
            leaf1_ebx: 1 << 16,
            leaf80000007_edx: None,
            leaf80000008_ecx: None,
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        Vec::new(),
    ))
    .expect("handoff");
    let mut observation = handoff.observation.clone();
    observation.memory_map.push(0);
    let err = boot_check(
        &handoff.boot_info_blob,
        &compiled.digest.bytes,
        &compiled.requirements,
        &observation,
    )
    .expect_err("must fail");
    assert_eq!(err.kind, BootCheckErrorKind::BootAbi);
}

#[test]
fn boot_check_error_display_covers_all_kinds() {
    assert!(BootCheckError::new(BootCheckErrorKind::BootAbi, "x")
        .to_string()
        .contains("boot abi error"));
    assert!(BootCheckError::new(BootCheckErrorKind::Observation, "x")
        .to_string()
        .contains("boot observation error"));
    assert!(BootCheckError::new(BootCheckErrorKind::Platform, "x")
        .to_string()
        .contains("boot platform error"));
    let platform_err: BootCheckError = PlatformError::new(
        PlatformErrorKind::Validation,
        "missing vmx",
    )
    .into();
    assert_eq!(platform_err.kind, BootCheckErrorKind::Platform);
    let observed = observe_platform(&ObservationInputs {
        cpuid: CpuidSnapshot {
            leaf1_ecx: 0,
            leaf1_edx: 0,
            leaf1_ebx: 0,
            leaf80000007_edx: None,
            leaf80000008_ecx: None,
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        acpi_tables: Vec::new(),
        memory_map: Vec::new(),
        memory_descriptor_size: 4,
        pci_devices: Vec::new(),
    })
    .expect_err("must fail");
    let mapped: BootCheckError = observed.into();
    assert_eq!(mapped.kind, BootCheckErrorKind::Observation);
}
