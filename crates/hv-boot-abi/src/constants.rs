//! Firmware and boot-time numeric constants.

/// UEFI memory type for conventional RAM.
pub const EFI_MEMORY_CONVENTIONAL: u32 = 7;

/// UEFI firmware page size in bytes.
pub const UEFI_PAGE_SIZE: u64 = 4096;

/// ACPI RSDP signature bytes (`RSD PTR `).
pub const RSDP_SIGNATURE: [u8; 8] = *b"RSD PTR ";

/// ACPI DMAR table signature.
pub const DMAR_SIGNATURE: [u8; 4] = *b"DMAR";

/// Byte offset of the DMAR flags field from the table header base.
pub const DMAR_FLAGS_OFFSET: usize = 0x28;

/// DMAR flags bit indicating interrupt remapping support.
pub const DMAR_FLAG_INTR_REMAP: u8 = 0x01;
