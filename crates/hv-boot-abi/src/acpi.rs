//! ACPI root pointer structures used during boot handoff.

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
}
