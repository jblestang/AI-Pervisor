//! Portable hypervisor transfer verification for the UEFI entry path.

#![cfg_attr(not(test), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

mod error;

use hv_boot_abi::{
    decode_observation_transfer, BootInfoView, HypervisorTransferView, RequirementsSnapshot,
};
use hv_types::SHA256_DIGEST_BYTES;

pub use error::{HypervisorEfiError, HypervisorEfiErrorKind};

/// Verifies a loader transfer blob against embedded digest and requirements metadata.
pub fn verify_hypervisor_transfer(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
) -> Result<(), HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }

    let view = HypervisorTransferView::parse(transfer).map_err(HypervisorEfiError::from)?;
    let boot_info = BootInfoView::parse(view.boot_info())
        .map_err(|err| HypervisorEfiError::new(HypervisorEfiErrorKind::BootInfo, err.message))?;
    boot_info
        .verify_config_digest(expected_config_digest)
        .map_err(|err| HypervisorEfiError::new(HypervisorEfiErrorKind::BootInfo, err.message))?;
    decode_observation_transfer(view.observation())
        .map_err(|err| HypervisorEfiError::new(HypervisorEfiErrorKind::Observation, err.message))?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_hypervisor::requirements_snapshot_from_platform;
    use hv_loader::{
        build_hypervisor_transfer, build_loader_handoff, encode_qemu_reference_firmware,
        LoaderHandoffInput,
    };
    use hv_observation_types::{
        CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
        CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
        CPUID_80000007_EDX_INVARIANT_TSC_BIT,
    };
    use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

    #[test]
    fn verify_hypervisor_transfer_accepts_reference_handoff() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let snapshot =
            requirements_snapshot_from_platform(&compiled.requirements, compiled.digest.bytes)
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
        verify_hypervisor_transfer(&transfer, &compiled.digest.bytes, &snapshot).expect("verify");
    }

    #[test]
    fn verify_hypervisor_transfer_rejects_digest_mismatch() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let mut snapshot =
            requirements_snapshot_from_platform(&compiled.requirements, compiled.digest.bytes)
                .expect("snapshot");
        snapshot.config_digest[0] ^= 0xFF;
        let transfer = [0u8; 64];
        let err = verify_hypervisor_transfer(&transfer, &compiled.digest.bytes, &snapshot)
            .expect_err("must fail");
        assert_eq!(err.kind, HypervisorEfiErrorKind::Requirements);
    }

    #[test]
    fn verify_hypervisor_transfer_rejects_invalid_blob() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let snapshot =
            requirements_snapshot_from_platform(&compiled.requirements, compiled.digest.bytes)
                .expect("snapshot");
        let err = verify_hypervisor_transfer(&[0xAA; 16], &compiled.digest.bytes, &snapshot)
            .expect_err("must fail");
        assert_eq!(err.kind, HypervisorEfiErrorKind::Transfer);
    }

    #[test]
    fn hypervisor_efi_error_display_and_boot_error_conversion() {
        let err = HypervisorEfiError::new(HypervisorEfiErrorKind::BootInfo, "bad boot info");
        assert!(err.to_string().contains("bad boot info"));
        let converted = HypervisorEfiError::from(hv_boot_abi::BootError::new(
            hv_boot_abi::BootErrorKind::Bounds,
            "bad bounds",
        ));
        assert_eq!(converted.kind, HypervisorEfiErrorKind::Transfer);
    }
}
