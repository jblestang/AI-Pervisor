//! Portable UEFI loader entry for Gate B.

mod error;

use hv_loader::{build_loader_handoff, LoaderHandoff, LoaderHandoffInput};
use hv_platform_model::CpuidSnapshot;
use hv_types::{PciBdf, SHA256_DIGEST_BYTES};

pub use error::{LoaderEfiError, LoaderEfiErrorKind};
pub use hv_loader::FirmwareMemoryImage;

/// Inputs available to the UEFI loader application entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UefiLoaderParams {
    /// Expected configuration digest embedded in the loader image.
    pub config_digest: [u8; SHA256_DIGEST_BYTES],
    /// Raw UEFI memory map bytes.
    pub memory_map: Vec<u8>,
    /// Size of one memory map descriptor.
    pub memory_descriptor_size: usize,
    /// ACPI RSDP bytes copied from firmware.
    pub rsdp: Vec<u8>,
    /// Firmware physical memory image for ACPI discovery.
    pub firmware_memory: FirmwareMemoryImage,
    /// CPUID snapshot collected at boot.
    pub cpuid: CpuidSnapshot,
    /// PCI devices discovered by firmware.
    pub pci_devices: Vec<PciBdf>,
}

/// Runs the portable loader entry sequence before hypervisor handoff.
pub fn uefi_loader_entry(params: UefiLoaderParams) -> Result<LoaderHandoff, LoaderEfiError> {
    let input = LoaderHandoffInput {
        config_digest: params.config_digest,
        memory_map: params.memory_map,
        memory_descriptor_size: params.memory_descriptor_size,
        rsdp: params.rsdp,
        firmware_memory: params.firmware_memory,
        cpuid: params.cpuid,
        pci_devices: params.pci_devices,
    };
    build_loader_handoff(&input).map_err(LoaderEfiError::from)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_loader::encode_empty_acpi_firmware;

    #[test]
    fn uefi_loader_entry_rejects_invalid_rsdp() {
        let err = uefi_loader_entry(UefiLoaderParams {
            config_digest: [0u8; SHA256_DIGEST_BYTES],
            memory_map: vec![0u8; 48],
            memory_descriptor_size: 48,
            rsdp: b"BAD".to_vec(),
            firmware_memory: encode_empty_acpi_firmware(),
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
        })
        .expect_err("must fail");
        assert_eq!(err.kind, LoaderEfiErrorKind::Handoff);
    }
}
