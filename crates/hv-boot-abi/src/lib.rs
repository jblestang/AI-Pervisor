//! Versioned boot ABI shared by the UEFI loader and hypervisor.
//!
//! The ABI uses plain C layouts only. No heap types, references, or Rust-specific
//! metadata cross this boundary.

#![no_std]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

/// Current boot ABI version.
pub const BOOT_ABI_VERSION: u32 = 1;

/// Magic bytes for `BootInfoHeader`.
pub const BOOT_INFO_MAGIC: [u8; 8] = *b"HVBOOT\0\0";

/// Loader to hypervisor boot information header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootInfoHeader {
    /// Magic identifier (`HVBOOT\0\0`).
    pub magic: [u8; 8],
    /// ABI version number.
    pub version: u32,
    /// Total boot info structure size in bytes.
    pub size: u32,
    /// SHA-256 digest of the normalized configuration.
    pub config_digest: [u8; 32],
    /// Offset to the descriptor table from the start of the boot info blob.
    pub descriptor_table_offset: u32,
    /// Number of descriptors in the table.
    pub descriptor_count: u32,
}

/// Descriptor entry describing one boot info section.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootInfoDescriptor {
    /// Descriptor type identifier.
    pub kind: u32,
    /// Offset from boot info base.
    pub offset: u32,
    /// Section size in bytes.
    pub size: u32,
}

/// Known boot info descriptor kinds.
pub mod descriptor_kind {
    /// Memory map handed over from UEFI.
    pub const MEMORY_MAP: u32 = 1;
    /// ACPI RSDP pointer.
    pub const RSDP: u32 = 2;
    /// Embedded configuration digest metadata.
    pub const CONFIG: u32 = 3;
    /// Hypervisor image hash metadata.
    pub const HYPERVISOR_IMAGE: u32 = 4;
}

/// Returns whether a boot info header matches the supported ABI.
pub fn boot_abi_is_compatible(header: &BootInfoHeader) -> bool {
    header.magic == BOOT_INFO_MAGIC && header.version == BOOT_ABI_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn header_layout_is_stable() {
        assert_eq!(size_of::<BootInfoHeader>(), 56);
        assert_eq!(align_of::<BootInfoHeader>(), 4);
        assert_eq!(size_of::<BootInfoDescriptor>(), 12);
    }

    #[test]
    fn boot_abi_accepts_compatible_header() {
        let header = BootInfoHeader {
            magic: BOOT_INFO_MAGIC,
            version: BOOT_ABI_VERSION,
            size: size_of::<BootInfoHeader>() as u32,
            config_digest: [0; 32],
            descriptor_table_offset: 0,
            descriptor_count: 0,
        };
        assert!(boot_abi_is_compatible(&header));
    }

    #[test]
    fn boot_abi_rejects_incompatible_version() {
        let header = BootInfoHeader {
            magic: BOOT_INFO_MAGIC,
            version: BOOT_ABI_VERSION + 1,
            size: core::mem::size_of::<BootInfoHeader>() as u32,
            config_digest: [0; 32],
            descriptor_table_offset: 0,
            descriptor_count: 0,
        };
        assert!(!boot_abi_is_compatible(&header));
    }

    #[test]
    fn boot_abi_rejects_bad_magic() {
        let header = BootInfoHeader {
            magic: *b"BADMAGIC",
            version: BOOT_ABI_VERSION,
            size: core::mem::size_of::<BootInfoHeader>() as u32,
            config_digest: [0; 32],
            descriptor_table_offset: 0,
            descriptor_count: 0,
        };
        assert!(!boot_abi_is_compatible(&header));
    }
}
