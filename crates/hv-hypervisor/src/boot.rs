//! Hypervisor boot-path validation orchestration.

use hv_boot_abi::{validate_rsdp_section, BootInfoView};
use hv_config_model::PlatformRequirements;
use hv_platform_model::{
    observe_platform, validate_platform, ObservationInputs, PlatformWarning, ValidatedPlatform,
};
use hv_types::SHA256_DIGEST_BYTES;

use crate::error::{BootCheckError, BootCheckErrorKind};

/// Runs the Gate B boot checks before hypervisor initialization continues.
pub fn boot_check(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &ObservationInputs,
) -> Result<(ValidatedPlatform, Vec<PlatformWarning>), BootCheckError> {
    let boot_info = BootInfoView::parse(boot_info_bytes)?;
    boot_info.verify_config_digest(expected_config_digest)?;

    if let Some(rsdp) = boot_info.rsdp_section()? {
        validate_rsdp_section(rsdp)?;
    }

    if let Some(memory_map) = boot_info.memory_map_section()? {
        if memory_map != observation.memory_map.as_slice() {
            return Err(BootCheckError::new(
                BootCheckErrorKind::BootAbi,
                "boot info memory map does not match observation input",
            ));
        }
    }

    let observed = observe_platform(observation)?;
    let (validated, warnings) = validate_platform(requirements, &observed).map_err(|err| {
        BootCheckError::new(BootCheckErrorKind::Platform, err.to_string())
    })?;
    Ok((validated, warnings))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hv_boot_abi::EFI_MEMORY_CONVENTIONAL;
    use hv_config_model::compile_config_from_str;
    use hv_loader::{build_loader_handoff, encode_qemu_reference_firmware, LoaderHandoffInput};
    use hv_platform_model::{
        CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
        CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
        CPUID_80000007_EDX_INVARIANT_TSC_BIT,
    };
    use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

    #[test]
    fn boot_check_accepts_reference_loader_handoff() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let digest = compiled.digest.bytes;
        let firmware = encode_qemu_reference_firmware();

        let mut memory_map = vec![0u8; 48];
        memory_map[0..4].copy_from_slice(&EFI_MEMORY_CONVENTIONAL.to_le_bytes());
        memory_map[24..32].copy_from_slice(&(2_097_152u64).to_le_bytes());

        let input = LoaderHandoffInput::with_default_descriptor_size(
            digest,
            memory_map,
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
                leaf480_ecx: Some(
                    (1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT),
                ),
                leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
            },
            vec![
                PciBdf {
                    segment: PciSegment::new(0),
                    bus: PciBus::new(0),
                    device: PciDevice::new(3),
                    function: PciFunction::new(0),
                },
                PciBdf {
                    segment: PciSegment::new(0),
                    bus: PciBus::new(0),
                    device: PciDevice::new(4),
                    function: PciFunction::new(0),
                },
            ],
        );
        let handoff = build_loader_handoff(&input, &firmware).expect("handoff");
        let (validated, warnings) = boot_check(
            &handoff.boot_info_blob,
            &digest,
            &compiled.requirements,
            &handoff.observation,
        )
        .expect("boot check");
        assert!(validated.observed.vmx);
        assert!(validated.observed.interrupt_remapping);
        assert!(warnings.is_empty());
    }
}
