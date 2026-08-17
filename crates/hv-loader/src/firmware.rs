//! Firmware memory fixtures and ACPI image builders for loader tests.

#![allow(clippy::indexing_slicing)]

use hv_acpi_walk::{ACPI_TABLE_HEADER_LENGTH, FirmwareMemoryImage};
use hv_boot_abi::{finalize_acpi_table_checksum, AcpiRsdp, encode_reference_dmar_with_intr_remap};

/// Physical addresses used by the reference QEMU firmware fixture.
pub mod reference_addresses {
    /// Physical address of the RSDP copy.
    pub const RSDP: u64 = 0x0000_0000_0000_1000;
    /// Physical address of the XSDT root table.
    pub const XSDT: u64 = 0x0000_0000_0000_2000;
    /// Physical address of the DMAR table.
    pub const DMAR: u64 = 0x0000_0000_0000_3000;
    /// Size of the reference firmware image.
    pub const IMAGE_SIZE: usize = 0x4000;
}

/// Builds the reference QEMU-like firmware memory image with RSDP, XSDT, and DMAR.
pub fn encode_qemu_reference_firmware() -> FirmwareMemoryImage {
    let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(reference_addresses::XSDT);
    let xsdt = encode_xsdt_with_table(reference_addresses::DMAR);
    let dmar = encode_reference_dmar_with_intr_remap();

    let mut bytes = vec![0u8; reference_addresses::IMAGE_SIZE];
    write_at(&mut bytes, reference_addresses::RSDP, &rsdp);
    write_at(&mut bytes, reference_addresses::XSDT, &xsdt);
    write_at(&mut bytes, reference_addresses::DMAR, &dmar);

    FirmwareMemoryImage::new(0, bytes)
}

/// Builds a firmware image with an empty XSDT and no nested tables.
pub fn encode_empty_acpi_firmware() -> FirmwareMemoryImage {
    let xsdt_address = reference_addresses::XSDT;
    let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(xsdt_address);
    let xsdt = encode_empty_xsdt();

    let mut bytes = vec![0u8; reference_addresses::IMAGE_SIZE];
    write_at(&mut bytes, reference_addresses::RSDP, &rsdp);
    write_at(&mut bytes, xsdt_address, &xsdt);

    FirmwareMemoryImage::new(0, bytes)
}

fn encode_empty_xsdt() -> Vec<u8> {
    let mut xsdt = vec![0u8; ACPI_TABLE_HEADER_LENGTH];
    xsdt[0..4].copy_from_slice(b"XSDT");
    xsdt[4..8].copy_from_slice(&(ACPI_TABLE_HEADER_LENGTH as u32).to_le_bytes());
    finalize_acpi_table_checksum(&mut xsdt);
    xsdt
}

fn encode_xsdt_with_table(table_address: u64) -> Vec<u8> {
    let mut xsdt = vec![0u8; ACPI_TABLE_HEADER_LENGTH + 8];
    xsdt[0..4].copy_from_slice(b"XSDT");
    xsdt[ACPI_TABLE_HEADER_LENGTH..ACPI_TABLE_HEADER_LENGTH + 8]
        .copy_from_slice(&table_address.to_le_bytes());
    let length = xsdt.len() as u32;
    xsdt[4..8].copy_from_slice(&length.to_le_bytes());
    finalize_acpi_table_checksum(&mut xsdt);
    xsdt
}

fn write_at(image: &mut [u8], address: u64, data: &[u8]) {
    let start = address as usize;
    let end = start + data.len();
    if let Some(slice) = image.get_mut(start..end) {
        slice.copy_from_slice(data);
    }
}
