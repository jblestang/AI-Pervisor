//! RSDP to XSDT/RSDT ACPI table collection.

use hv_boot_abi::AcpiRsdp;
use hv_boot_abi::RSDP_REVISION_ACPI2;

use crate::constants::{
    ACPI_COLLECTED_MAX_BYTES, ACPI_ROOT_MAX_ENTRIES, ACPI_TABLE_HEADER_LENGTH,
    ACPI_TABLE_MAX_LENGTH, RSDT_ENTRY_SIZE, RSDT_SIGNATURE, XSDT_ENTRY_SIZE, XSDT_SIGNATURE,
};
use crate::error::{AcpiWalkError, AcpiWalkErrorKind};
use crate::physical::PhysicalMemory;

/// Collects ACPI tables reachable from an RSDP by walking XSDT or RSDT.
pub fn collect_acpi_tables(
    memory: &impl PhysicalMemory,
    rsdp: &AcpiRsdp,
) -> Result<alloc::vec::Vec<u8>, AcpiWalkError> {
    let root_address = root_table_address(rsdp)?;
    let root_table = read_table(memory, root_address)?;
    validate_root_signature(&root_table)?;
    let mut output = alloc::vec::Vec::new();
    append_nested_tables(memory, &root_table, &mut output)?;
    Ok(output)
}

fn root_table_address(rsdp: &AcpiRsdp) -> Result<u64, AcpiWalkError> {
    if rsdp.revision >= RSDP_REVISION_ACPI2 && rsdp.xsdt_address != 0 {
        Ok(rsdp.xsdt_address)
    } else if rsdp.rsdt_address != 0 {
        Ok(u64::from(rsdp.rsdt_address))
    } else {
        Err(AcpiWalkError::new(
            AcpiWalkErrorKind::Parse,
            "RSDP provides no XSDT or RSDT address",
        ))
    }
}

fn validate_root_signature(root_table: &[u8]) -> Result<(), AcpiWalkError> {
    let signature = root_table
        .get(0..4)
        .ok_or(AcpiWalkError::new(
            AcpiWalkErrorKind::Parse,
            "root table signature unavailable",
        ))?;
    if signature != XSDT_SIGNATURE && signature != RSDT_SIGNATURE {
        return Err(AcpiWalkError::new(
            AcpiWalkErrorKind::Parse,
            "root table is neither XSDT nor RSDT",
        ));
    }
    Ok(())
}

fn append_nested_tables(
    memory: &impl PhysicalMemory,
    root_table: &[u8],
    output: &mut alloc::vec::Vec<u8>,
) -> Result<(), AcpiWalkError> {
    let signature = root_table
        .get(0..4)
        .ok_or(AcpiWalkError::new(
            AcpiWalkErrorKind::Parse,
            "root table signature unavailable",
        ))?;
    let entry_size = if signature == XSDT_SIGNATURE {
        XSDT_ENTRY_SIZE
    } else {
        RSDT_ENTRY_SIZE
    };

    let root_length = read_u32(root_table, 4)? as usize;
    if root_length < ACPI_TABLE_HEADER_LENGTH {
        return Err(AcpiWalkError::new(
            AcpiWalkErrorKind::Bounds,
            "root table shorter than ACPI header",
        ));
    }
    if root_length > root_table.len() {
        return Err(AcpiWalkError::new(
            AcpiWalkErrorKind::Bounds,
            "root table length exceeds buffer",
        ));
    }
    validate_table_checksum(root_table.get(0..root_length).ok_or(AcpiWalkError::new(
        AcpiWalkErrorKind::Bounds,
        "root table bounded slice unavailable",
    ))?)?;

    let mut entry_offset = ACPI_TABLE_HEADER_LENGTH;
    let mut entries_processed = 0usize;
    while entry_offset
        .checked_add(entry_size)
        .ok_or(AcpiWalkError::new(
            AcpiWalkErrorKind::Bounds,
            "root entry offset overflow",
        ))?
        <= root_length
    {
        if entries_processed >= ACPI_ROOT_MAX_ENTRIES {
            return Err(AcpiWalkError::new(
                AcpiWalkErrorKind::Bounds,
                "ACPI root table entry count exceeds limit",
            ));
        }
        entries_processed = entries_processed
            .checked_add(1)
            .ok_or(AcpiWalkError::new(
                AcpiWalkErrorKind::Bounds,
                "ACPI root entry count overflow",
            ))?;

        let pointer = read_entry_pointer(root_table, entry_offset, entry_size)?;
        if pointer != 0 {
            let table = read_table(memory, pointer)?;
            let table_length = read_u32(&table, 4)? as usize;
            if table_length < ACPI_TABLE_HEADER_LENGTH {
                return Err(AcpiWalkError::new(
                    AcpiWalkErrorKind::Bounds,
                    "ACPI table shorter than header",
                ));
            }
            if table_length > table.len() {
                return Err(AcpiWalkError::new(
                    AcpiWalkErrorKind::Bounds,
                    "ACPI table length exceeds buffer",
                ));
            }
            validate_table_checksum(table.get(0..table_length).ok_or(
                AcpiWalkError::new(AcpiWalkErrorKind::Bounds, "table bounded slice unavailable"),
            )?)?;
            let next_len = output
                .len()
                .checked_add(table_length)
                .ok_or(AcpiWalkError::new(
                    AcpiWalkErrorKind::Bounds,
                    "collected ACPI table bytes overflow",
                ))?;
            if next_len > ACPI_COLLECTED_MAX_BYTES {
                return Err(AcpiWalkError::new(
                    AcpiWalkErrorKind::Bounds,
                    "collected ACPI table bytes exceed limit",
                ));
            }
            output.extend_from_slice(table.get(0..table_length).ok_or(
                AcpiWalkError::new(AcpiWalkErrorKind::Bounds, "table append slice unavailable"),
            )?);
        }
        entry_offset = entry_offset
            .checked_add(entry_size)
            .ok_or(AcpiWalkError::new(
                AcpiWalkErrorKind::Bounds,
                "root entry advance overflow",
            ))?;
    }

    Ok(())
}

fn read_table(memory: &impl PhysicalMemory, address: u64) -> Result<alloc::vec::Vec<u8>, AcpiWalkError> {
    let mut header = [0u8; ACPI_TABLE_HEADER_LENGTH];
    memory.read_physical(address, &mut header)?;
    let length = read_u32(&header, 4)? as usize;
    if length < ACPI_TABLE_HEADER_LENGTH {
        return Err(AcpiWalkError::new(
            AcpiWalkErrorKind::Bounds,
            "ACPI table length smaller than header",
        ));
    }
    if length > ACPI_TABLE_MAX_LENGTH {
        return Err(AcpiWalkError::new(
            AcpiWalkErrorKind::Bounds,
            "ACPI table declared length exceeds limit",
        ));
    }
    let mut table = alloc::vec![0u8; length];
    memory.read_physical(address, table.as_mut_slice())?;
    Ok(table)
}

fn read_entry_pointer(
    root_table: &[u8],
    entry_offset: usize,
    entry_size: usize,
) -> Result<u64, AcpiWalkError> {
    let entry_end = entry_offset
        .checked_add(entry_size)
        .ok_or(AcpiWalkError::new(
            AcpiWalkErrorKind::Bounds,
            "entry slice overflow",
        ))?;
    let entry = root_table.get(entry_offset..entry_end).ok_or(
        AcpiWalkError::new(AcpiWalkErrorKind::Bounds, "entry slice unavailable"),
    )?;
    if entry_size == XSDT_ENTRY_SIZE {
        let chunk: [u8; 8] = entry.try_into().map_err(|_| {
            AcpiWalkError::new(AcpiWalkErrorKind::Parse, "XSDT entry truncated")
        })?;
        Ok(u64::from_le_bytes(chunk))
    } else {
        let chunk: [u8; 4] = entry.try_into().map_err(|_| {
            AcpiWalkError::new(AcpiWalkErrorKind::Parse, "RSDT entry truncated")
        })?;
        Ok(u64::from(u32::from_le_bytes(chunk)))
    }
}

fn read_u32(bytes: &[u8], start: usize) -> Result<u32, AcpiWalkError> {
    let slice = bytes
        .get(start..start + 4)
        .ok_or(AcpiWalkError::new(
            AcpiWalkErrorKind::Parse,
            "u32 field truncated",
        ))?;
    let chunk: [u8; 4] = slice.try_into().map_err(|_| {
        AcpiWalkError::new(AcpiWalkErrorKind::Parse, "u32 field truncated")
    })?;
    Ok(u32::from_le_bytes(chunk))
}

fn validate_table_checksum(table: &[u8]) -> Result<(), AcpiWalkError> {
    if table.iter().fold(0u8, |acc, byte| acc.wrapping_add(*byte)) != 0 {
        return Err(AcpiWalkError::new(
            AcpiWalkErrorKind::Parse,
            "ACPI table checksum invalid",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::physical::FirmwareMemoryImage;
    use hv_boot_abi::finalize_acpi_table_checksum;

    fn write_table_checksum(table: &mut [u8]) {
        finalize_acpi_table_checksum(table);
    }

    fn encode_xsdt_with_dmar(dmar_address: u64) -> alloc::vec::Vec<u8> {
        let mut xsdt = alloc::vec::Vec::new();
        xsdt.extend_from_slice(b"XSDT");
        xsdt.extend_from_slice(&0u32.to_le_bytes());
        xsdt.resize(ACPI_TABLE_HEADER_LENGTH, 0);
        xsdt.extend_from_slice(&dmar_address.to_le_bytes());
        let length = xsdt.len() as u32;
        xsdt[4..8].copy_from_slice(&length.to_le_bytes());
        write_table_checksum(&mut xsdt);
        xsdt
    }

    #[test]
    fn collect_acpi_tables_from_xsdt_finds_dmar() {
        let dmar = hv_boot_abi::encode_reference_dmar_with_intr_remap();
        let dmar_address = 0x3000u64;
        let xsdt_address = 0x2000u64;
        let rsdp_address = 0x1000u64;
        let rsdp = AcpiRsdp::encode_reference_v2_with_xsdt(xsdt_address);
        let xsdt = encode_xsdt_with_dmar(dmar_address);

        let mut image_bytes = alloc::vec![0u8; 0x4000];
        image_bytes[rsdp_address as usize..rsdp_address as usize + rsdp.len()]
            .copy_from_slice(&rsdp);
        image_bytes[xsdt_address as usize..xsdt_address as usize + xsdt.len()].copy_from_slice(&xsdt);
        image_bytes[dmar_address as usize..dmar_address as usize + dmar.len()]
            .copy_from_slice(&dmar);

        let memory = FirmwareMemoryImage::new(0, image_bytes);
        let parsed = AcpiRsdp::parse(&rsdp).expect("parse rsdp");
        let collected = collect_acpi_tables(&memory, &parsed).expect("collect");
        assert!(collected.windows(4).any(|window| window == b"DMAR"));
    }

    #[test]
    fn collect_acpi_tables_from_rsdt_finds_dmar() {
        let dmar = hv_boot_abi::encode_reference_dmar_with_intr_remap();
        let dmar_address = 0x3000u64;
        let rsdt_address = 0x2000u64;
        let mut rsdp_v1 = [0u8; 36];
        rsdp_v1[0..8].copy_from_slice(b"RSD PTR ");
        rsdp_v1[15] = 0;
        rsdp_v1[16..20].copy_from_slice(&(rsdt_address as u32).to_le_bytes());
        rsdp_v1[20..24].copy_from_slice(&36u32.to_le_bytes());
        let sum = rsdp_v1.iter().take(20).fold(0u8, |acc, b| acc.wrapping_add(*b));
        rsdp_v1[8] = 0u8.wrapping_sub(sum);
        let parsed = AcpiRsdp::parse(&rsdp_v1).expect("parse v1");
        let mut rsdt = alloc::vec![0u8; ACPI_TABLE_HEADER_LENGTH + 4];
        rsdt[0..4].copy_from_slice(b"RSDT");
        rsdt[ACPI_TABLE_HEADER_LENGTH..ACPI_TABLE_HEADER_LENGTH + 4]
            .copy_from_slice(&(dmar_address as u32).to_le_bytes());
        let rsdt_length = rsdt.len() as u32;
        rsdt[4..8].copy_from_slice(&rsdt_length.to_le_bytes());
        finalize_acpi_table_checksum(&mut rsdt);
        let mut image = alloc::vec![0u8; 0x4000];
        image[0x1000..0x1000 + 36].copy_from_slice(&rsdp_v1);
        image[rsdt_address as usize..rsdt_address as usize + rsdt.len()].copy_from_slice(&rsdt);
        image[dmar_address as usize..dmar_address as usize + dmar.len()].copy_from_slice(&dmar);
        let memory = FirmwareMemoryImage::new(0, image);
        let collected = collect_acpi_tables(&memory, &parsed).expect("collect");
        assert!(collected.windows(4).any(|window| window == b"DMAR"));
    }
}
