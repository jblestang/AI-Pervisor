//! Loader handoff construction from firmware-provided inputs.

use crate::build::{build_boot_info_blob, BootInfoSection};
use hv_acpi_walk::{collect_acpi_tables, FirmwareMemoryImage};
use hv_boot_abi::{validate_rsdp_section, AcpiRsdp, BootInfoView, UEFI_MEMORY_DESCRIPTOR_MIN_SIZE};
use hv_platform_model::{CpuidSnapshot, ObservationInputs};
use hv_types::{PciBdf, SHA256_DIGEST_BYTES};

use crate::constants::{DEFAULT_MEMORY_DESCRIPTOR_SIZE, MEMORY_MAP_KIND, RSDP_KIND};
use crate::error::{LoaderError, LoaderErrorKind};

/// Inputs collected by the loader before hypervisor entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderHandoffInput {
    /// Expected configuration digest embedded in the loader image.
    pub config_digest: [u8; SHA256_DIGEST_BYTES],
    /// Raw UEFI memory map bytes.
    pub memory_map: Vec<u8>,
    /// Size of one memory map descriptor.
    pub memory_descriptor_size: usize,
    /// ACPI RSDP bytes copied from firmware.
    pub rsdp: Vec<u8>,
    /// Firmware physical memory image used to walk ACPI tables.
    pub firmware_memory: FirmwareMemoryImage,
    /// CPUID snapshot collected at boot.
    pub cpuid: CpuidSnapshot,
    /// PCI devices discovered by firmware.
    pub pci_devices: Vec<PciBdf>,
}

/// Loader output handed to the hypervisor entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderHandoff {
    /// Serialized boot info blob.
    pub boot_info_blob: Vec<u8>,
    /// Observation inputs derived from firmware data.
    pub observation: ObservationInputs,
}

impl LoaderHandoffInput {
    /// Creates a handoff input with the default UEFI descriptor size.
    pub fn with_default_descriptor_size(
        config_digest: [u8; SHA256_DIGEST_BYTES],
        memory_map: Vec<u8>,
        rsdp: Vec<u8>,
        firmware_memory: FirmwareMemoryImage,
        cpuid: CpuidSnapshot,
        pci_devices: Vec<PciBdf>,
    ) -> Self {
        Self {
            config_digest,
            memory_map,
            memory_descriptor_size: DEFAULT_MEMORY_DESCRIPTOR_SIZE,
            rsdp,
            firmware_memory,
            cpuid,
            pci_devices,
        }
    }
}

/// Builds the boot info blob and observation bundle for hypervisor entry.
pub fn build_loader_handoff(input: &LoaderHandoffInput) -> Result<LoaderHandoff, LoaderError> {
    if input.memory_descriptor_size == 0 {
        return Err(LoaderError::new(
            LoaderErrorKind::Observation,
            "memory descriptor size must not be zero",
        ));
    }
    if input.memory_descriptor_size < UEFI_MEMORY_DESCRIPTOR_MIN_SIZE {
        return Err(LoaderError::new(
            LoaderErrorKind::Observation,
            "memory descriptor size below UEFI minimum",
        ));
    }

    validate_rsdp_section(&input.rsdp).map_err(LoaderError::from)?;

    let parsed_rsdp = AcpiRsdp::parse(&input.rsdp).map_err(LoaderError::from)?;
    let acpi_tables =
        collect_acpi_tables(&input.firmware_memory, &parsed_rsdp).map_err(LoaderError::from)?;

    let boot_info_blob = build_boot_info_blob(
        input.config_digest,
        &[
            BootInfoSection {
                kind: MEMORY_MAP_KIND,
                data: input.memory_map.clone(),
            },
            BootInfoSection {
                kind: RSDP_KIND,
                data: input.rsdp.clone(),
            },
        ],
    )?;

    BootInfoView::parse(&boot_info_blob).map_err(LoaderError::from)?;

    let observation = ObservationInputs {
        cpuid: input.cpuid.clone(),
        acpi_tables,
        memory_map: input.memory_map.clone(),
        memory_descriptor_size: input.memory_descriptor_size,
        pci_devices: input.pci_devices.clone(),
    };

    Ok(LoaderHandoff {
        boot_info_blob,
        observation,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::firmware::encode_empty_acpi_firmware;
    use hv_platform_model::{
        CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
        CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT, CPUID_80000007_EDX_INVARIANT_TSC_BIT,
    };

    #[test]
    fn build_loader_handoff_produces_parseable_boot_info() {
        let digest = [0x11; SHA256_DIGEST_BYTES];
        let firmware = encode_empty_acpi_firmware();
        let rsdp = firmware
            .bytes
            .get(
                crate::firmware::reference_addresses::RSDP as usize
                    ..crate::firmware::reference_addresses::RSDP as usize + 36,
            )
            .expect("rsdp slice")
            .to_vec();
        let input = LoaderHandoffInput::with_default_descriptor_size(
            digest,
            vec![0u8; 48],
            rsdp,
            firmware,
            CpuidSnapshot {
                leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
                leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
                leaf1_ebx: 1 << 16,
                leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
                leaf80000008_ecx: Some(0),
                leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
                leaf480_ebx: None,
            },
            Vec::new(),
        );
        let handoff = build_loader_handoff(&input).expect("handoff");
        let view = BootInfoView::parse(&handoff.boot_info_blob).expect("parse");
        view.verify_config_digest(&digest).expect("digest");
    }
}
