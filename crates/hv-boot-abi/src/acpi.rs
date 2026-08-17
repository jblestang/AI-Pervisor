//! ACPI root pointer structures used during boot handoff.

use core::mem::size_of;

use crate::constants::{RSDP_REVISION_ACPI2, RSDP_SIGNATURE, RSDP_V1_CHECKSUM_LENGTH};
use crate::error::{BootError, BootErrorKind};

/// ACPI Root System Description Pointer (ACPI 2.0+).
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiRsdp {
    /// Signature (`RSD PTR `).
    pub signature: [u8; 8],
    /// Checksum of the first 20 bytes.
    pub checksum: u8,
    /// OEM identifier.
    pub oem_id: [u8; 6],
    /// Revision (0 for ACPI 1.0, 2+ for ACPI 2.0+).
    pub revision: u8,
    /// Physical address of the RSDT.
    pub rsdt_address: u32,
    /// Total length of the RSDP structure.
    pub length: u32,
    /// Physical address of the XSDT (ACPI 2.0+).
    pub xsdt_address: u64,
    /// Extended checksum covering the full RSDP length.
    pub extended_checksum: u8,
    /// Reserved bytes required by the ACPI 2.0+ layout.
    pub reserved: [u8; 3],
}

/// ACPI table header common prefix.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpiTableHeader {
    /// Four-character table signature.
    pub signature: [u8; 4],
    /// Total table length in bytes.
    pub length: u32,
    /// ACPI specification revision.
    pub revision: u8,
    /// Checksum of the entire table.
    pub checksum: u8,
    /// OEM identifier.
    pub oem_id: [u8; 6],
    /// OEM table identifier.
    pub oem_table_id: [u8; 8],
    /// OEM revision number.
    pub oem_revision: u32,
    /// Creator ID.
    pub creator_id: u32,
    /// Creator revision.
    pub creator_revision: u32,
}

impl AcpiRsdp {
    /// Parses and validates an ACPI RSDP blob.
    pub fn parse(bytes: &[u8]) -> Result<Self, BootError> {
        if bytes.len() < size_of::<AcpiRsdp>() {
            return Err(BootError::new(
                BootErrorKind::Parse,
                "RSDP shorter than ACPI 2.0 layout",
            ));
        }

        let signature = bytes.get(0..RSDP_SIGNATURE.len()).ok_or(BootError::new(
            BootErrorKind::Parse,
            "RSDP signature unavailable",
        ))?;
        if signature != RSDP_SIGNATURE {
            return Err(BootError::new(
                BootErrorKind::Parse,
                "invalid RSDP signature",
            ));
        }

        if !acpi_bytes_sum_to_zero(bytes.get(0..RSDP_V1_CHECKSUM_LENGTH).ok_or(BootError::new(
            BootErrorKind::Parse,
            "RSDP v1 checksum range unavailable",
        ))?) {
            return Err(BootError::new(
                BootErrorKind::Parse,
                "invalid RSDP v1 checksum",
            ));
        }

        let revision = *bytes.get(15).ok_or(BootError::new(
            BootErrorKind::Parse,
            "RSDP revision missing",
        ))?;
        let length = read_u32(bytes, 20)?;
        if revision >= RSDP_REVISION_ACPI2 {
            if length as usize > bytes.len() {
                return Err(BootError::new(
                    BootErrorKind::Bounds,
                    "RSDP declared length exceeds buffer",
                ));
            }
            if (length as usize) < size_of::<AcpiRsdp>() {
                return Err(BootError::new(
                    BootErrorKind::Bounds,
                    "RSDP declared length too small",
                ));
            }
            if !acpi_bytes_sum_to_zero(bytes.get(0..length as usize).ok_or(BootError::new(
                BootErrorKind::Parse,
                "RSDP extended checksum range unavailable",
            ))?) {
                return Err(BootError::new(
                    BootErrorKind::Parse,
                    "invalid RSDP extended checksum",
                ));
            }
        }

        Ok(Self {
            signature: read_signature(bytes)?,
            checksum: *bytes.get(8).ok_or(BootError::new(
                BootErrorKind::Parse,
                "RSDP checksum byte missing",
            ))?,
            oem_id: read_oem_id(bytes)?,
            revision,
            rsdt_address: read_u32(bytes, 16)?,
            length,
            xsdt_address: read_u64(bytes, 24)?,
            extended_checksum: *bytes.get(32).ok_or(BootError::new(
                BootErrorKind::Parse,
                "RSDP extended checksum byte missing",
            ))?,
            reserved: read_reserved(bytes)?,
        })
    }

    /// Builds a valid ACPI 2.0+ RSDP blob for host-side tests and fixtures.
    pub fn encode_reference_v2() -> [u8; 36] {
        let mut rsdp = [0u8; 36];
        rsdp[0..8].copy_from_slice(&RSDP_SIGNATURE);
        rsdp[15] = RSDP_REVISION_ACPI2;
        rsdp[20..24].copy_from_slice(&(size_of::<AcpiRsdp>() as u32).to_le_bytes());
        write_v1_checksum(&mut rsdp);
        write_extended_checksum(&mut rsdp);
        rsdp
    }

    /// Builds a reference RSDP pointing at the given XSDT physical address.
    pub fn encode_reference_v2_with_xsdt(xsdt_address: u64) -> [u8; 36] {
        let mut rsdp = Self::encode_reference_v2();
        rsdp[24..32].copy_from_slice(&xsdt_address.to_le_bytes());
        write_extended_checksum(&mut rsdp);
        rsdp
    }
}

fn read_signature(bytes: &[u8]) -> Result<[u8; 8], BootError> {
    let slice = bytes.get(0..8).ok_or(BootError::new(
        BootErrorKind::Parse,
        "RSDP signature truncated",
    ))?;
    let chunk: [u8; 8] = slice
        .try_into()
        .map_err(|_| BootError::new(BootErrorKind::Parse, "RSDP signature truncated"))?;
    Ok(chunk)
}

fn read_oem_id(bytes: &[u8]) -> Result<[u8; 6], BootError> {
    let slice = bytes.get(9..15).ok_or(BootError::new(
        BootErrorKind::Parse,
        "RSDP OEM ID truncated",
    ))?;
    let chunk: [u8; 6] = slice
        .try_into()
        .map_err(|_| BootError::new(BootErrorKind::Parse, "RSDP OEM ID truncated"))?;
    Ok(chunk)
}

fn read_reserved(bytes: &[u8]) -> Result<[u8; 3], BootError> {
    let slice = bytes.get(33..36).ok_or(BootError::new(
        BootErrorKind::Parse,
        "RSDP reserved truncated",
    ))?;
    let chunk: [u8; 3] = slice
        .try_into()
        .map_err(|_| BootError::new(BootErrorKind::Parse, "RSDP reserved truncated"))?;
    Ok(chunk)
}

fn read_u32(bytes: &[u8], start: usize) -> Result<u32, BootError> {
    let slice = bytes.get(start..start + 4).ok_or(BootError::new(
        BootErrorKind::Parse,
        "RSDP u32 field truncated",
    ))?;
    let chunk: [u8; 4] = slice
        .try_into()
        .map_err(|_| BootError::new(BootErrorKind::Parse, "RSDP u32 field truncated"))?;
    Ok(u32::from_le_bytes(chunk))
}

fn read_u64(bytes: &[u8], start: usize) -> Result<u64, BootError> {
    let slice = bytes.get(start..start + 8).ok_or(BootError::new(
        BootErrorKind::Parse,
        "RSDP u64 field truncated",
    ))?;
    let chunk: [u8; 8] = slice
        .try_into()
        .map_err(|_| BootError::new(BootErrorKind::Parse, "RSDP u64 field truncated"))?;
    Ok(u64::from_le_bytes(chunk))
}

fn acpi_bytes_sum_to_zero(bytes: &[u8]) -> bool {
    bytes.iter().fold(0u8, |acc, byte| acc.wrapping_add(*byte)) == 0
}

fn write_v1_checksum(rsdp: &mut [u8; 36]) {
    rsdp[8] = 0;
    let sum = rsdp
        .iter()
        .take(RSDP_V1_CHECKSUM_LENGTH)
        .fold(0u8, |acc, byte| acc.wrapping_add(*byte));
    rsdp[8] = 0u8.wrapping_sub(sum);
}

fn write_extended_checksum(rsdp: &mut [u8; 36]) {
    rsdp[32] = 0;
    let length = rsdp.len();
    let sum = rsdp
        .iter()
        .take(length)
        .fold(0u8, |acc, byte| acc.wrapping_add(*byte));
    rsdp[32] = 0u8.wrapping_sub(sum);
}

/// Builds a minimal valid DMAR table with interrupt remapping enabled.
pub fn encode_reference_dmar_with_intr_remap() -> [u8; 48] {
    let mut table = [0u8; 48];
    table[0..4].copy_from_slice(&crate::constants::DMAR_SIGNATURE);
    table[4..8].copy_from_slice(&(crate::constants::DMAR_MIN_LENGTH as u32).to_le_bytes());
    if let Some(flag) = table.get_mut(crate::constants::DMAR_FLAGS_OFFSET) {
        *flag = crate::constants::DMAR_FLAG_INTR_REMAP;
    }
    finalize_acpi_table_checksum(&mut table);
    table
}

/// Computes the ACPI table checksum byte at offset 9.
pub fn finalize_acpi_table_checksum(table: &mut [u8]) {
    if let Some(checksum) = table.get_mut(9) {
        *checksum = 0;
    }
    let sum = table.iter().fold(0u8, |acc, byte| acc.wrapping_add(*byte));
    if let Some(checksum) = table.get_mut(9) {
        *checksum = 0u8.wrapping_sub(sum);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn acpi_rsdp_layout_is_stable() {
        assert_eq!(size_of::<AcpiRsdp>(), 36);
        assert_eq!(align_of::<AcpiRsdp>(), 1);
    }

    #[test]
    fn acpi_table_header_layout_is_stable() {
        assert_eq!(size_of::<AcpiTableHeader>(), 36);
        assert_eq!(align_of::<AcpiTableHeader>(), 4);
    }

    #[test]
    fn encode_reference_v2_round_trips_through_parse() {
        let rsdp = AcpiRsdp::encode_reference_v2();
        AcpiRsdp::parse(&rsdp).expect("parse reference rsdp");
    }

    #[test]
    fn encode_reference_dmar_matches_minimum_length() {
        assert_eq!(crate::constants::DMAR_MIN_LENGTH, 48);
        let table = encode_reference_dmar_with_intr_remap();
        assert_eq!(table.len(), crate::constants::DMAR_MIN_LENGTH);
    }

    #[test]
    fn parse_rejects_invalid_signature() {
        let mut rsdp = AcpiRsdp::encode_reference_v2();
        rsdp[0] = b'X';
        let err = AcpiRsdp::parse(&rsdp).expect_err("must fail");
        assert_eq!(err.kind, BootErrorKind::Parse);
    }

    #[test]
    fn parse_rejects_declared_length_beyond_buffer() {
        let mut rsdp = AcpiRsdp::encode_reference_v2();
        rsdp[20..24].copy_from_slice(&128u32.to_le_bytes());
        write_extended_checksum(&mut rsdp);
        let err = AcpiRsdp::parse(&rsdp).expect_err("must fail");
        assert_eq!(err.kind, BootErrorKind::Bounds);
    }

    #[test]
    fn parse_rejects_bad_extended_checksum() {
        let mut rsdp = AcpiRsdp::encode_reference_v2();
        if let Some(byte) = rsdp.get_mut(32) {
            *byte ^= 0xFF;
        }
        let err = AcpiRsdp::parse(&rsdp).expect_err("must fail");
        assert_eq!(err.kind, BootErrorKind::Parse);
    }

    #[test]
    fn parse_rejects_bad_v1_checksum() {
        let mut rsdp = AcpiRsdp::encode_reference_v2();
        if let Some(byte) = rsdp.get_mut(8) {
            *byte ^= 0xFF;
        }
        let err = AcpiRsdp::parse(&rsdp).expect_err("must fail");
        assert_eq!(err.kind, BootErrorKind::Parse);
    }
}
