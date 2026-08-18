//! Portable hypervisor transfer verification for the UEFI entry path.

#![cfg_attr(not(test), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

#[cfg(feature = "real-hw-execution")]
mod allocator;

mod error;

use hv_boot_abi::{LayoutSnapshot, RequirementsSnapshot};
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_c_from_snapshots;
use hv_types::SHA256_DIGEST_BYTES;

#[cfg(feature = "real-hw-execution")]
use hv_x86_cpu::{CpuInstructionDisposition, PageAllocator};
#[cfg(feature = "real-hw-execution")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_c_real_hw_from_snapshots;

pub use error::{HypervisorEfiError, HypervisorEfiErrorKind};
pub use hv_hypervisor_boot::{
    REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER,
};

#[cfg(feature = "real-hw-execution")]
pub use allocator::UefiPageAllocator;

/// REAL_HW boot outcome markers for serial-log verification.
#[cfg(feature = "real-hw-execution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealHwBootMarkers {
    /// Whether VMXON was executed live.
    pub vmxon_executed: bool,
    /// Whether the EPT pointer was loaded live.
    pub ept_executed: bool,
}

/// Runs full Gate B validation and mock-backed Gate C init from a transfer blob.
pub fn boot_hypervisor_from_transfer(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
) -> Result<(), HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    boot_from_transfer_and_init_gate_c_from_snapshots(transfer, requirements, layout)
        .map(|_| ())
        .map_err(HypervisorEfiError::from)
}

/// Runs Gate B validation and REAL_HW Gate C init with resident page installation.
#[cfg(feature = "real-hw-execution")]
pub fn boot_hypervisor_from_transfer_real_hw<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<RealHwBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_c_real_hw_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    let vmxon_executed = result
        .live
        .cpu_seam
        .vmx_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    let ept_executed = result
        .live
        .cpu_seam
        .ept_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    Ok(RealHwBootMarkers {
        vmxon_executed,
        ept_executed,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_hypervisor_boot::{
        layout_snapshot_from_platform_ir, requirements_snapshot_from_platform,
    };
    use hv_loader::{
        build_hypervisor_transfer, build_loader_handoff, encode_qemu_reference_firmware,
        LoaderHandoffInput,
    };
    use hv_observation_types::{
        CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
        CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
        CPUID_80000007_EDX_INVARIANT_TSC_BIT,
    };
    use hv_platform_model::plan_static_platform_ir;
    use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

    fn reference_snapshots() -> (
        hv_boot_abi::RequirementsSnapshot,
        hv_boot_abi::LayoutSnapshot,
        [u8; SHA256_DIGEST_BYTES],
    ) {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let requirements = requirements_snapshot_from_platform(
            &compiled.requirements,
            compiled.digest.bytes,
            layout.hypervisor_reserve.host_phys.raw(),
            layout.hypervisor_reserve.size.bytes(),
        )
        .expect("snapshot");
        let layout_snapshot = layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
        (requirements, layout_snapshot, compiled.digest.bytes)
    }

    #[test]
    fn boot_hypervisor_from_transfer_accepts_reference_handoff() {
        let (requirements, layout, digest) = reference_snapshots();
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
        boot_hypervisor_from_transfer(&transfer, &digest, &requirements, &layout).expect("boot");
    }

    #[test]
    fn boot_hypervisor_from_transfer_rejects_digest_mismatch() {
        let (mut requirements, layout, digest) = reference_snapshots();
        requirements.config_digest[0] ^= 0xFF;
        let err = boot_hypervisor_from_transfer(&[0u8; 64], &digest, &requirements, &layout)
            .expect_err("must fail");
        assert_eq!(err.kind, HypervisorEfiErrorKind::Requirements);
    }

    #[test]
    fn boot_hypervisor_from_transfer_rejects_invalid_blob() {
        let (requirements, layout, digest) = reference_snapshots();
        let err = boot_hypervisor_from_transfer(&[0xAA; 16], &digest, &requirements, &layout)
            .expect_err("must fail");
        assert_eq!(err.kind, HypervisorEfiErrorKind::BootInfo);
    }

    #[test]
    fn boot_hypervisor_from_transfer_rejects_layout_reserve_mismatch() {
        let (requirements, mut layout, digest) = reference_snapshots();
        layout.hypervisor_reserve_bytes ^= 1;
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
        let err = boot_hypervisor_from_transfer(&transfer, &digest, &requirements, &layout)
            .expect_err("must fail");
        assert_eq!(err.kind, HypervisorEfiErrorKind::Platform);
    }

    #[test]
    fn hypervisor_efi_error_from_boot_check_maps_all_kinds() {
        use hv_hypervisor_boot::{BootCheckError, BootCheckErrorKind};
        let boot_abi: HypervisorEfiError =
            BootCheckError::new(BootCheckErrorKind::BootAbi, "boot").into();
        assert_eq!(boot_abi.kind, HypervisorEfiErrorKind::BootInfo);
        let observation: HypervisorEfiError =
            BootCheckError::new(BootCheckErrorKind::Observation, "obs").into();
        assert_eq!(observation.kind, HypervisorEfiErrorKind::Observation);
        let platform: HypervisorEfiError =
            BootCheckError::new(BootCheckErrorKind::Platform, "plat").into();
        assert_eq!(platform.kind, HypervisorEfiErrorKind::Platform);
        assert!(platform.to_string().contains("Platform"));
    }

    #[test]
    fn hypervisor_efi_error_from_boot_abi_error() {
        let err: HypervisorEfiError = hv_boot_abi::BootError::new(
            hv_boot_abi::BootErrorKind::Parse,
            "bad transfer",
        )
        .into();
        assert_eq!(err.kind, HypervisorEfiErrorKind::Transfer);
    }
}
