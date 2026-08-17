//! Coverage-oriented ACPI walk tests.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_acpi_walk::{
    collect_acpi_tables, AcpiWalkErrorKind, FirmwareMemoryImage, PhysicalMemory,
    ACPI_TABLE_HEADER_LENGTH, ACPI_TABLE_MAX_LENGTH,
};
use hv_boot_abi::{encode_reference_dmar_with_intr_remap, finalize_acpi_table_checksum, AcpiRsdp};

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
    let err = memory.read_physical(0, &mut buffer).expect_err("must fail");
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
    xsdt[ACPI_TABLE_HEADER_LENGTH..ACPI_TABLE_HEADER_LENGTH + 8]
        .copy_from_slice(&0u64.to_le_bytes());
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
fn collect_acpi_tables_rejects_declared_length_above_limit() {
    let table_address = 0x3000u64;
    let xsdt_address = 0x2000u64;
    let mut xsdt = vec![0u8; ACPI_TABLE_HEADER_LENGTH + 8];
    xsdt[0..4].copy_from_slice(b"XSDT");
    xsdt[ACPI_TABLE_HEADER_LENGTH..ACPI_TABLE_HEADER_LENGTH + 8]
        .copy_from_slice(&table_address.to_le_bytes());
    let xsdt_length = xsdt.len() as u32;
    xsdt[4..8].copy_from_slice(&xsdt_length.to_le_bytes());
    finalize_acpi_table_checksum(&mut xsdt);

    let mut table_header = vec![0u8; ACPI_TABLE_HEADER_LENGTH];
    table_header[0..4].copy_from_slice(b"DMAR");
    table_header[4..8].copy_from_slice(&((ACPI_TABLE_MAX_LENGTH + 1) as u32).to_le_bytes());

    let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(xsdt_address);
    let mut image = vec![0u8; 0x4000];
    image[xsdt_address as usize..xsdt_address as usize + xsdt.len()].copy_from_slice(&xsdt);
    image[table_address as usize..table_address as usize + table_header.len()]
        .copy_from_slice(&table_header);
    let memory = FirmwareMemoryImage::new(0, image);
    let parsed = AcpiRsdp::parse(&rsdp).expect("parse");
    let err = collect_acpi_tables(&memory, &parsed).expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Bounds);
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

fn encode_xsdt_with_entry_pointers(pointers: &[u64]) -> Vec<u8> {
    let mut xsdt = vec![0u8; ACPI_TABLE_HEADER_LENGTH + pointers.len() * 8];
    xsdt[0..4].copy_from_slice(b"XSDT");
    for (index, pointer) in pointers.iter().enumerate() {
        let start = ACPI_TABLE_HEADER_LENGTH + index * 8;
        xsdt[start..start + 8].copy_from_slice(&pointer.to_le_bytes());
    }
    let length = xsdt.len() as u32;
    xsdt[4..8].copy_from_slice(&length.to_le_bytes());
    finalize_acpi_table_checksum(&mut xsdt);
    xsdt
}

fn encode_large_table(signature: &[u8; 4], total_length: usize) -> Vec<u8> {
    let mut table = vec![0u8; total_length];
    table[0..4].copy_from_slice(signature);
    table[4..8].copy_from_slice(&(total_length as u32).to_le_bytes());
    finalize_acpi_table_checksum(&mut table);
    table
}

#[test]
fn collect_acpi_tables_rejects_excessive_root_entries() {
    use hv_acpi_walk::ACPI_ROOT_MAX_ENTRIES;

    let xsdt_address = 0x2000u64;
    let pointers = vec![0u64; ACPI_ROOT_MAX_ENTRIES + 1];
    let xsdt = encode_xsdt_with_entry_pointers(&pointers);
    let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(xsdt_address);
    let mut image = vec![0u8; 0x4000];
    image[xsdt_address as usize..xsdt_address as usize + xsdt.len()].copy_from_slice(&xsdt);
    let memory = FirmwareMemoryImage::new(0, image);
    let parsed = AcpiRsdp::parse(&rsdp).expect("parse");
    let err = collect_acpi_tables(&memory, &parsed).expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Bounds);
}

#[test]
fn collect_acpi_tables_rejects_collected_byte_limit() {
    use hv_acpi_walk::{ACPI_COLLECTED_MAX_BYTES, ACPI_TABLE_MAX_LENGTH};

    let table_length = ACPI_TABLE_MAX_LENGTH;
    let tables_needed = ACPI_COLLECTED_MAX_BYTES / table_length + 1;
    let table_stride = table_length as u64;
    let mut pointers = Vec::with_capacity(tables_needed);
    for index in 0..tables_needed {
        pointers.push(0x1_0000_0000u64 + index as u64 * table_stride);
    }
    let xsdt_address = 0x2000u64;
    let xsdt = encode_xsdt_with_entry_pointers(&pointers);
    let image_end = pointers.last().expect("pointer") + table_stride;
    let mut image = vec![0u8; image_end as usize];
    for pointer in &pointers {
        let table = encode_large_table(b"TEST", table_length);
        let start = *pointer as usize;
        image[start..start + table.len()].copy_from_slice(&table);
    }
    image[xsdt_address as usize..xsdt_address as usize + xsdt.len()].copy_from_slice(&xsdt);
    let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(xsdt_address);
    let memory = FirmwareMemoryImage::new(0, image);
    let parsed = AcpiRsdp::parse(&rsdp).expect("parse");
    let err = collect_acpi_tables(&memory, &parsed).expect_err("must fail");
    assert_eq!(err.kind, AcpiWalkErrorKind::Bounds);
}
