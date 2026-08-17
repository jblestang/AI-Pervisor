//! Hypervisor transfer blob construction from loader handoff output.

use alloc::vec::Vec;

use hv_boot_abi::{
    build_hypervisor_transfer_blob, CpuidTransferSnapshot, ObservationTransferParts,
};
use hv_observation_types::CpuidSnapshot;

use crate::error::{LoaderError, LoaderErrorKind};
use crate::handoff::LoaderHandoff;

/// Builds the loader-to-hypervisor transfer blob from a completed handoff.
pub fn build_hypervisor_transfer(handoff: &LoaderHandoff) -> Result<Vec<u8>, LoaderError> {
    let observation = ObservationTransferParts {
        cpuid: cpuid_transfer_from_snapshot(&handoff.observation.cpuid),
        memory_map: handoff.observation.memory_map.as_slice(),
        memory_descriptor_size: handoff.observation.memory_descriptor_size,
        acpi_tables: handoff.observation.acpi_tables.as_slice(),
        pci_devices: handoff.observation.pci_devices.as_slice(),
    };
    build_hypervisor_transfer_blob(&handoff.boot_info_blob, &observation).map_err(map_boot_error)
}

fn cpuid_transfer_from_snapshot(snapshot: &CpuidSnapshot) -> CpuidTransferSnapshot {
    CpuidTransferSnapshot {
        leaf1_ecx: snapshot.leaf1_ecx,
        leaf1_edx: snapshot.leaf1_edx,
        leaf1_ebx: snapshot.leaf1_ebx,
        leaf80000007_edx: snapshot.leaf80000007_edx,
        leaf80000008_ecx: snapshot.leaf80000008_ecx,
        leaf480_ecx: snapshot.leaf480_ecx,
        leaf480_ebx: snapshot.leaf480_ebx,
    }
}

fn map_boot_error(err: hv_boot_abi::BootError) -> LoaderError {
    LoaderError::new(LoaderErrorKind::BootInfo, err.message)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::firmware::encode_empty_acpi_firmware;
    use crate::handoff::{build_loader_handoff, LoaderHandoffInput};
    use hv_boot_abi::HypervisorTransferView;
    use hv_observation_types::{CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_EDX_NX_BIT};
    use hv_types::SHA256_DIGEST_BYTES;

    #[test]
    fn build_hypervisor_transfer_produces_parseable_blob() {
        let digest = [0x33; SHA256_DIGEST_BYTES];
        let firmware = encode_empty_acpi_firmware();
        let rsdp = firmware
            .bytes
            .get(
                crate::firmware::reference_addresses::RSDP as usize
                    ..crate::firmware::reference_addresses::RSDP as usize + 36,
            )
            .expect("rsdp")
            .to_vec();
        let handoff = build_loader_handoff(
            &LoaderHandoffInput::with_default_descriptor_size(
                digest,
                vec![0u8; 48],
                rsdp,
                CpuidSnapshot {
                    leaf1_ecx: 1 << CPUID_1_ECX_VMX_BIT,
                    leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
                    leaf1_ebx: 1 << 16,
                    leaf80000007_edx: None,
                    leaf80000008_ecx: None,
                    leaf480_ecx: None,
                    leaf480_ebx: None,
                },
                Vec::new(),
            ),
            &firmware,
        )
        .expect("handoff");
        let transfer = build_hypervisor_transfer(&handoff).expect("transfer");
        let view = HypervisorTransferView::parse(&transfer).expect("parse");
        assert_eq!(view.boot_info(), handoff.boot_info_blob.as_slice());
    }
}
