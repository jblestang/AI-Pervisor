//! Firmware and boot-time numeric constants.

/// UEFI memory type for conventional RAM.
pub const EFI_MEMORY_CONVENTIONAL: u32 = 7;

/// UEFI firmware page size in bytes.
pub const UEFI_PAGE_SIZE: u64 = 4096;

/// Minimum UEFI memory descriptor size per the spec (40 bytes on x86-64).
pub const UEFI_MEMORY_DESCRIPTOR_MIN_SIZE: usize = 40;

/// OVMF memory map descriptor stride on x86-64.
pub const UEFI_MEMORY_DESCRIPTOR_OVMF_SIZE: usize = 48;

/// ACPI RSDP signature bytes (`RSD PTR `).
pub const RSDP_SIGNATURE: [u8; 8] = *b"RSD PTR ";

/// ACPI 1.0 RSDP checksum coverage length in bytes.
pub const RSDP_V1_CHECKSUM_LENGTH: usize = 20;

/// ACPI 2.0+ RSDP revision threshold.
pub const RSDP_REVISION_ACPI2: u8 = 2;

/// ACPI DMAR table signature.
pub const DMAR_SIGNATURE: [u8; 4] = *b"DMAR";

/// Byte offset of the DMAR host address width field from the table base.
pub const DMAR_HOST_ADDRESS_WIDTH_OFFSET: usize = 0x24;

/// Byte offset of the DMAR flags field from the table base.
pub const DMAR_FLAGS_OFFSET: usize = 0x25;

/// Minimum valid DMAR table length in bytes.
pub const DMAR_MIN_LENGTH: usize = 0x30;

/// DMAR flags bit indicating interrupt remapping support.
pub const DMAR_FLAG_INTR_REMAP: u8 = 0x01;

/// Serial log marker emitted after successful REAL_HW Gate C init.
pub const REAL_HW_BOOT_SUCCESS_MARKER: &str = "hypervisor Gate C REAL_HW boot succeeded";
/// Serial log marker emitted when VMXON executes under REAL_HW Gate C.
pub const REAL_HW_VMXON_EXECUTED_MARKER: &str = "REAL_HW: VMXON Executed";
/// Serial log marker emitted when EPT pointer load executes under REAL_HW Gate C.
pub const REAL_HW_EPT_EXECUTED_MARKER: &str = "REAL_HW: EPT pointer Executed";
/// Serial log marker emitted when VMLAUNCH executes under REAL_HW Gate C.
pub const REAL_HW_VMLAUNCH_EXECUTED_MARKER: &str = "REAL_HW: VMLAUNCH Executed";
