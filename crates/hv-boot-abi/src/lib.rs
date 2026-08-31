//! Versioned boot ABI shared by the UEFI loader and hypervisor.
//!
//! The ABI uses plain C layouts only. No heap types, references, or Rust-specific
//! metadata cross this boundary.

#![cfg_attr(not(test), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

use hv_types::SHA256_DIGEST_BYTES;

mod acpi;
mod boot_info;
mod constants;
mod error;
mod layout_snapshot;
mod requirements_snapshot;
mod transfer;
mod uefi;

pub use acpi::{
    encode_reference_dmar_with_intr_remap, finalize_acpi_table_checksum, AcpiRsdp, AcpiTableHeader,
};
pub use boot_info::{validate_rsdp_section, BootInfoView};
pub use constants::{
    DMAR_FLAGS_OFFSET, DMAR_FLAG_INTR_REMAP, DMAR_HOST_ADDRESS_WIDTH_OFFSET, DMAR_MIN_LENGTH,
    DMAR_SIGNATURE, EFI_MEMORY_CONVENTIONAL, GATE_D_BENCHMARK_TARGET_MET_MARKER,
    GATE_D_BOOT_INFO_BUILT_MARKER, GATE_D_DATAPATH_BENCHMARK_MARKER,
    GATE_D_DATAPATH_FOUNDATION_MARKER, GATE_D_DATAPATH_GUESTS_MARKER, GATE_D_DATAPATH_LIVE_MARKER,
    GATE_D_DATAPATH_MALICIOUS_MARKER, GATE_D_DATAPATH_RUNTIME_MARKER, GATE_D_E1000_MMIO_MARKER,
    GATE_D_GUEST_BOOT_INFO_INSTALLED_MARKER, GATE_D_GUEST_DATAPATH_FRAME_MARKER,
    GATE_D_GUEST_ELF_INSTALLED_MARKER, GATE_D_GUEST_EXECUTION_MARKER,
    GATE_D_GUEST_SOURCE_ELF_MARKER, GATE_D_GUEST_THROUGHPUT_EXECUTED_MARKER,
    GATE_D_GUEST_THROUGHPUT_MARKER, GATE_D_GUEST_THROUGHPUT_TARGET_MET_MARKER,
    GATE_D_IPC_FORWARD_MARKER, GATE_D_IPC_INTEGRITY_MARKER, GATE_D_MULTI_VMLAUNCH_MARKER,
    REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER, REAL_HW_VMLAUNCH_EXECUTED_MARKER,
    REAL_HW_VMXON_EXECUTED_MARKER, RSDP_REVISION_ACPI2, RSDP_SIGNATURE,
    UEFI_MEMORY_DESCRIPTOR_MIN_SIZE, UEFI_MEMORY_DESCRIPTOR_OVMF_SIZE, UEFI_PAGE_SIZE,
};
pub use error::{BootError, BootErrorKind};
pub use layout_snapshot::{
    LayoutGuestRegionSnapshot, LayoutIpcRegionSnapshot, LayoutPciSnapshot, LayoutSnapshot,
    PlannedRegionSnapshot, LAYOUT_DEVICE_KIND_NIC_E1000, MAX_LAYOUT_GUEST_REGIONS,
    MAX_LAYOUT_IPC_REGIONS, MAX_LAYOUT_PCI_DEVICES,
};
pub use requirements_snapshot::{
    ExpectedPciSnapshot, RequirementsSnapshot, FEATURE_DISABLED, FEATURE_OPTIONAL,
    FEATURE_PREFERRED, FEATURE_REQUIRED, MAX_REQUIREMENTS_PAGE_SIZES, MAX_REQUIREMENTS_PCI_DEVICES,
    REQUIREMENTS_ARCH_X86_64, SMT_POLICY_ALLOW_CROSS_PARTITION, SMT_POLICY_DISABLED,
    SMT_POLICY_EXCLUSIVE_CORE, SMT_POLICY_SAME_PARTITION_SIBLINGS,
};
pub use transfer::{
    build_hypervisor_transfer_blob, decode_observation_transfer, patch_published_alloc_size,
    validate_transfer_bounds, CpuidTransferSnapshot, Guid, HypervisorTransferHeader,
    HypervisorTransferView, ObservationTransferHeader, ObservationTransferParts,
    ObservationTransferPartsOwned, PciBdfTransfer, HV_TRANSFER_TABLE_GUID, TRANSFER_ABI_VERSION,
    TRANSFER_MAGIC,
};
pub use uefi::UefiMemoryDescriptor;

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
    pub config_digest: [u8; SHA256_DIGEST_BYTES],
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
            config_digest: [0; SHA256_DIGEST_BYTES],
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
            config_digest: [0; SHA256_DIGEST_BYTES],
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
            config_digest: [0; SHA256_DIGEST_BYTES],
            descriptor_table_offset: 0,
            descriptor_count: 0,
        };
        assert!(!boot_abi_is_compatible(&header));
    }
}
