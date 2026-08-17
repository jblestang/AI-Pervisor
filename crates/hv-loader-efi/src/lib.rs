//! Portable UEFI loader entry for Gate B.

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

mod error;
mod transfer;

use hv_acpi_walk::PhysicalMemory;
use hv_loader::{build_loader_handoff, LoaderHandoff, LoaderHandoffInput};
use hv_observation_types::CpuidSnapshot;
use hv_types::{PciBdf, SHA256_DIGEST_BYTES};

pub use error::{LoaderEfiError, LoaderEfiErrorKind};
#[cfg(any(test, feature = "std"))]
pub use hv_loader::FirmwareMemoryImage;
pub use transfer::build_hypervisor_transfer_from_entry;

/// Inputs available to the UEFI loader application entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UefiLoaderParams {
    /// Expected configuration digest embedded in the loader image.
    pub config_digest: [u8; SHA256_DIGEST_BYTES],
    /// Raw UEFI memory map bytes.
    pub memory_map: alloc::vec::Vec<u8>,
    /// Size of one memory map descriptor.
    pub memory_descriptor_size: usize,
    /// ACPI RSDP bytes copied from firmware.
    pub rsdp: alloc::vec::Vec<u8>,
    /// CPUID snapshot collected at boot.
    pub cpuid: CpuidSnapshot,
    /// PCI devices discovered by firmware.
    pub pci_devices: alloc::vec::Vec<PciBdf>,
}

/// Runs the portable loader entry sequence before hypervisor handoff.
pub fn uefi_loader_entry(
    params: UefiLoaderParams,
    firmware_memory: &impl PhysicalMemory,
) -> Result<LoaderHandoff, LoaderEfiError> {
    let input = LoaderHandoffInput {
        config_digest: params.config_digest,
        memory_map: params.memory_map,
        memory_descriptor_size: params.memory_descriptor_size,
        rsdp: params.rsdp,
        cpuid: params.cpuid,
        pci_devices: params.pci_devices,
    };
    build_loader_handoff(&input, firmware_memory).map_err(LoaderEfiError::from)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_loader::encode_empty_acpi_firmware;

    #[test]
    fn uefi_loader_entry_rejects_invalid_rsdp() {
        let firmware = encode_empty_acpi_firmware();
        let err = uefi_loader_entry(
            UefiLoaderParams {
                config_digest: [0u8; SHA256_DIGEST_BYTES],
                memory_map: vec![0u8; 48],
                memory_descriptor_size: 48,
                rsdp: b"BAD".to_vec(),
                cpuid: CpuidSnapshot {
                    leaf1_ecx: 0,
                    leaf1_edx: 0,
                    leaf1_ebx: 0,
                    leaf80000007_edx: None,
                    leaf80000008_ecx: None,
                    leaf480_ecx: None,
                    leaf480_ebx: None,
                },
                pci_devices: Vec::new(),
            },
            &firmware,
        )
        .expect_err("must fail");
        assert_eq!(err.kind, LoaderEfiErrorKind::Handoff);
    }
}
