//! Coverage-oriented ACPI walk tests.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_acpi_walk::{
    collect_acpi_tables, AcpiWalkErrorKind, FirmwareMemoryImage, PhysicalMemory,
    ACPI_TABLE_HEADER_LENGTH,
};
use hv_boot_abi::{finalize_acpi_table_checksum, AcpiRsdp, encode_reference_dmar_with_intr_remap};

#[test]
fn physical_memory_rejects_read_below_image_base() {
    let memory = FirmwareMemoryImage::new(0x1000, vec![0u8; 16]);
    let mut buffer = [0u8; 4];
    let err = memory
        .read_physical(0x800, &mut buffer)
        .expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Memory);
}

#[test]
fn physical_memory_rejects_read_past_image_end() {
    let memory = FirmwareMemoryImage::new(0, vec![0u8; 8]);
    let mut buffer = [0u8; 16];
    let err = memory
        .read_physical(0, &mut buffer)
        .expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Memory);
}

#[test]
fn collect_acpi_tables_rejects_missing_root_address() {
    let memory = FirmwareMemoryImage::new(0, vec![0u8; 64]);
    let mut parsed = AcpiRsdp::parse(&AcpiRsdp::encode_reference_v2()).expect("parse");
    parsed.xsdt_address = 0;
    parsed.rsdt_address = 0;
    let err = collect_acpi_tables(&memory, &parsed).expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Parse);
}

#[test]
fn collect_acpi_tables_rejects_invalid_root_signature() {
    let mut parsed = AcpiRsdp::parse(&AcpiRsdp::encode_reference_v2_with_xsdt(64)).expect("parse");
    parsed.xsdt_address = 64;
    let mut root = vec![0u8; 48];
    root[0..4].copy_from_slice(b"BAD!");
    root[4..8].copy_from_slice(&48u32.to_le_bytes());
    finalize_acpi_table_checksum(&mut root);
    let mut image = vec![0u8; 128];
    image[64..64 + root.len()].copy_from_slice(&root);
    let memory = FirmwareMemoryImage::new(0, image);
    let err = collect_acpi_tables(&memory, &parsed).expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Parse);
}

#[test]
fn collect_acpi_tables_rejects_invalid_table_checksum() {
    let dmar_address = 0x3000u64;
    let xsdt_address = 0x2000u64;
    let mut dmar = encode_reference_dmar_with_intr_remap();
    if let Some(byte) = dmar.get_mut(9) {
        *byte ^= 0xFF;
    }
    let mut xsdt = vec![0u8; ACPI_TABLE_HEADER_LENGTH + 8];
    xsdt[0..4].copy_from_slice(b"XSDT");
    xsdt[ACPI_TABLE_HEADER_LENGTH..ACPI_TABLE_HEADER_LENGTH + 8]
        .copy_from_slice(&dmar_address.to_le_bytes());
    let length = xsdt.len() as u32;
    xsdt[4..8].copy_from_slice(&length.to_le_bytes());
    finalize_acpi_table_checksum(&mut xsdt);
    let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(xsdt_address);
    let mut image = vec![0u8; 0x4000];
    image[0x1000..0x1000 + 36].copy_from_slice(&rsdp);
    image[xsdt_address as usize..xsdt_address as usize + xsdt.len()].copy_from_slice(&xsdt);
    image[dmar_address as usize..dmar_address as usize + dmar.len()].copy_from_slice(&dmar);
    let memory = FirmwareMemoryImage::new(0, image);
    let parsed = AcpiRsdp::parse(&rsdp).expect("parse");
    let err = collect_acpi_tables(&memory, &parsed).expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Parse);
}

#[test]
fn collect_acpi_tables_skips_null_xsdt_entries() {
    let dmar_address = 0x3000u64;
    let xsdt_address = 0x2000u64;
    let dmar = encode_reference_dmar_with_intr_remap();
    let mut xsdt = vec![0u8; ACPI_TABLE_HEADER_LENGTH + 16];
    xsdt[0..4].copy_from_slice(b"XSDT");
    xsdt[ACPI_TABLE_HEADER_LENGTH..ACPI_TABLE_HEADER_LENGTH + 8].copy_from_slice(&0u64.to_le_bytes());
    xsdt[ACPI_TABLE_HEADER_LENGTH + 8..ACPI_TABLE_HEADER_LENGTH + 16]
        .copy_from_slice(&dmar_address.to_le_bytes());
    let length = xsdt.len() as u32;
    xsdt[4..8].copy_from_slice(&length.to_le_bytes());
    finalize_acpi_table_checksum(&mut xsdt);
    let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(xsdt_address);
    let mut image = vec![0u8; 0x4000];
    image[0x1000..0x1000 + 36].copy_from_slice(&rsdp);
    image[xsdt_address as usize..xsdt_address as usize + xsdt.len()].copy_from_slice(&xsdt);
    image[dmar_address as usize..dmar_address as usize + dmar.len()].copy_from_slice(&dmar);
    let memory = FirmwareMemoryImage::new(0, image);
    let parsed = AcpiRsdp::parse(&rsdp).expect("parse");
    let collected = collect_acpi_tables(&memory, &parsed).expect("collect");
    assert!(collected.windows(4).any(|window| window == b"DMAR"));
}

#[test]
fn collect_acpi_tables_rejects_invalid_root_table_checksum() {
    let xsdt_address = 0x2000u64;
    let mut xsdt = vec![0u8; ACPI_TABLE_HEADER_LENGTH];
    xsdt[0..4].copy_from_slice(b"XSDT");
    xsdt[4..8].copy_from_slice(&(ACPI_TABLE_HEADER_LENGTH as u32).to_le_bytes());
    if let Some(byte) = xsdt.get_mut(9) {
        *byte = 0xFF;
    }
    let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(xsdt_address);
    let mut image = vec![0u8; 0x3000];
    image[xsdt_address as usize..xsdt_address as usize + xsdt.len()].copy_from_slice(&xsdt);
    let memory = FirmwareMemoryImage::new(0, image);
    let parsed = AcpiRsdp::parse(&rsdp).expect("parse");
    let err = collect_acpi_tables(&memory, &parsed).expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Parse);
}
