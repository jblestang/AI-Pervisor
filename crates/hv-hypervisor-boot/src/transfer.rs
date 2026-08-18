//! Hypervisor boot orchestration from loader transfer blobs.

use alloc::vec::Vec;

use hv_boot_abi::{
    decode_observation_transfer, HypervisorTransferView, ObservationTransferPartsOwned,
    RequirementsSnapshot,
};
use hv_config_model::PlatformRequirements;
use hv_observation_types::{CpuidSnapshot, ObservationInputs};
use hv_platform_model::{PlatformWarning, ValidatedPlatform};
use hv_types::SHA256_DIGEST_BYTES;

use crate::boot::boot_check;
use crate::error::{BootCheckError, BootCheckErrorKind};
use crate::snapshot::platform_requirements_from_snapshot;

/// Runs Gate B boot checks from a loader transfer blob and embedded requirements snapshot.
pub fn boot_from_transfer_snapshot(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
) -> Result<(ValidatedPlatform, Vec<PlatformWarning>), BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    boot_from_transfer(transfer, &snapshot.config_digest, &requirements)
}

/// Runs Gate B boot checks from a loader transfer blob.
pub fn boot_from_transfer(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
) -> Result<(ValidatedPlatform, Vec<PlatformWarning>), BootCheckError> {
    let view = HypervisorTransferView::parse(transfer).map_err(map_boot_error)?;
    let observation = decode_observation_transfer(view.observation()).map_err(map_boot_error)?;
    boot_check(
        view.boot_info(),
        expected_config_digest,
        requirements,
        &observation_inputs_from_transfer(observation)?,
    )
}

fn observation_inputs_from_transfer(
    observation: ObservationTransferPartsOwned,
) -> Result<ObservationInputs, BootCheckError> {
    if observation.memory_descriptor_size == 0 {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Observation,
            "memory descriptor size must not be zero",
        ));
    }
    Ok(ObservationInputs {
        cpuid: cpuid_from_transfer(observation.cpuid),
        acpi_tables: observation.acpi_tables,
        memory_map: observation.memory_map,
        memory_descriptor_size: observation.memory_descriptor_size,
        pci_devices: observation.pci_devices,
    })
}

fn cpuid_from_transfer(snapshot: hv_boot_abi::CpuidTransferSnapshot) -> CpuidSnapshot {
    CpuidSnapshot {
        leaf1_ecx: snapshot.leaf1_ecx,
        leaf1_edx: snapshot.leaf1_edx,
        leaf1_ebx: snapshot.leaf1_ebx,
        leaf80000007_edx: snapshot.leaf80000007_edx,
        leaf80000008_ecx: snapshot.leaf80000008_ecx,
        leaf480_ecx: snapshot.leaf480_ecx,
        leaf480_ebx: snapshot.leaf480_ebx,
    }
}

fn map_boot_error(err: hv_boot_abi::BootError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::BootAbi, err.message)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_loader::{
        build_hypervisor_transfer, build_loader_handoff, encode_qemu_reference_firmware,
        LoaderHandoffInput,
    };
    use hv_platform_model::plan_static_platform_ir;
    use hv_observation_types::{
        CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
        CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
        CPUID_80000007_EDX_INVARIANT_TSC_BIT,
    };
    use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

    #[test]
    fn boot_from_transfer_accepts_reference_handoff() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let firmware = encode_qemu_reference_firmware();
        let handoff = build_loader_handoff(
            &LoaderHandoffInput::with_default_descriptor_size(
                compiled.digest.bytes,
                {
                    let mut memory_map = vec![0u8; 48];
                    memory_map[0..4]
                        .copy_from_slice(&hv_boot_abi::EFI_MEMORY_CONVENTIONAL.to_le_bytes());
                    memory_map[24..32].copy_from_slice(&(2_097_152u64).to_le_bytes());
                    memory_map
                },
                firmware
                    .bytes
                    .get(0x1000..0x1000 + 36)
                    .expect("rsdp")
                    .to_vec(),
                CpuidSnapshot {
                    leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
                    leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
                    leaf1_ebx: (4 << 16) | 4,
                    leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
                    leaf80000008_ecx: Some(3),
                    leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
                    leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
                },
                vec![
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(3),
                        PciFunction::new(0),
                    ),
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(4),
                        PciFunction::new(0),
                    ),
                ],
            ),
            &firmware,
        )
        .expect("handoff");
        let transfer = build_hypervisor_transfer(&handoff).expect("transfer");
        let (validated, warnings) =
            boot_from_transfer(&transfer, &compiled.digest.bytes, &compiled.requirements)
                .expect("boot from transfer");
        assert!(validated.observed.vmx);
        assert!(warnings.is_empty());
    }

    #[test]
    fn boot_from_transfer_snapshot_accepts_reference_handoff() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let snapshot = crate::snapshot::requirements_snapshot_from_platform(
            &compiled.requirements,
            compiled.digest.bytes,
            layout.hypervisor_reserve.host_phys.raw(),
            layout.hypervisor_reserve.size.bytes(),
        )
        .expect("snapshot");
        let firmware = encode_qemu_reference_firmware();
        let handoff = build_loader_handoff(
            &LoaderHandoffInput::with_default_descriptor_size(
                compiled.digest.bytes,
                {
                    let mut memory_map = vec![0u8; 48];
                    memory_map[0..4]
                        .copy_from_slice(&hv_boot_abi::EFI_MEMORY_CONVENTIONAL.to_le_bytes());
                    memory_map[24..32].copy_from_slice(&(2_097_152u64).to_le_bytes());
                    memory_map
                },
                firmware
                    .bytes
                    .get(0x1000..0x1000 + 36)
                    .expect("rsdp")
                    .to_vec(),
                CpuidSnapshot {
                    leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
                    leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
                    leaf1_ebx: (4 << 16) | 4,
                    leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
                    leaf80000008_ecx: Some(3),
                    leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
                    leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
                },
                vec![
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(3),
                        PciFunction::new(0),
                    ),
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(4),
                        PciFunction::new(0),
                    ),
                ],
            ),
            &firmware,
        )
        .expect("handoff");
        let transfer = build_hypervisor_transfer(&handoff).expect("transfer");
        let (validated, warnings) =
            boot_from_transfer_snapshot(&transfer, &snapshot).expect("boot from snapshot");
        assert!(validated.observed.vmx);
        assert!(warnings.is_empty());
    }

    #[test]
    fn boot_from_transfer_rejects_zero_memory_descriptor_size() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let err = boot_from_transfer(&[0u8; 32], &compiled.digest.bytes, &compiled.requirements)
            .expect_err("must fail");
        assert_eq!(err.kind, BootCheckErrorKind::BootAbi);
    }
}
