//! Hypervisor transfer helpers for the portable UEFI loader entry.

use alloc::vec::Vec;

use hv_loader::{build_hypervisor_transfer, LoaderHandoff};

use crate::error::{LoaderEfiError, LoaderEfiErrorKind};
use crate::uefi_loader_entry;

use crate::UefiLoaderParams;
use hv_acpi_walk::PhysicalMemory;

/// Builds the loader handoff and serializes it into a hypervisor transfer blob.
pub fn build_hypervisor_transfer_from_entry(
    params: UefiLoaderParams,
    firmware_memory: &impl PhysicalMemory,
) -> Result<(LoaderHandoff, Vec<u8>), LoaderEfiError> {
    let handoff = uefi_loader_entry(params, firmware_memory)?;
    let transfer = build_hypervisor_transfer(&handoff)
        .map_err(|err| LoaderEfiError::new(LoaderEfiErrorKind::Handoff, err.message))?;
    Ok((handoff, transfer))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_loader::encode_qemu_reference_firmware;

    #[test]
    fn build_hypervisor_transfer_from_entry_produces_parseable_blob() {
        let firmware = encode_qemu_reference_firmware();
        let rsdp = firmware
            .bytes
            .get(0x1000..0x1000 + 36)
            .expect("rsdp")
            .to_vec();
        let (_handoff, transfer) = build_hypervisor_transfer_from_entry(
            UefiLoaderParams {
                config_digest: [0x11; hv_types::SHA256_DIGEST_BYTES],
                memory_map: vec![0u8; 48],
                memory_descriptor_size: 48,
                rsdp,
                cpuid: hv_observation_types::CpuidSnapshot {
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
        .expect("transfer");
        assert!(hv_boot_abi::HypervisorTransferView::parse(&transfer).is_ok());
    }

    #[test]
    fn build_hypervisor_transfer_from_entry_maps_loader_errors() {
        let firmware = encode_qemu_reference_firmware();
        let err = build_hypervisor_transfer_from_entry(
            UefiLoaderParams {
                config_digest: [0x11; hv_types::SHA256_DIGEST_BYTES],
                memory_map: vec![0u8; 48],
                memory_descriptor_size: 48,
                rsdp: b"BAD".to_vec(),
                cpuid: hv_observation_types::CpuidSnapshot {
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
