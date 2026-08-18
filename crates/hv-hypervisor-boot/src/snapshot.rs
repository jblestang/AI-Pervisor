//! Requirements snapshot conversion for embedded hypervisor images.

use alloc::string::String;

use hv_boot_abi::{
    ExpectedPciSnapshot, RequirementsSnapshot, FEATURE_DISABLED, FEATURE_OPTIONAL,
    FEATURE_PREFERRED, FEATURE_REQUIRED, MAX_REQUIREMENTS_PAGE_SIZES, MAX_REQUIREMENTS_PCI_DEVICES,
    REQUIREMENTS_ARCH_X86_64, SMT_POLICY_ALLOW_CROSS_PARTITION, SMT_POLICY_DISABLED,
    SMT_POLICY_EXCLUSIVE_CORE, SMT_POLICY_SAME_PARTITION_SIBLINGS,
};
use hv_config_model::{
    ExpectedPciDevice, FeatureRequirement, PageSizeSet, PlatformRequirements, SmtPolicy,
};
use hv_types::{
    ByteSize, PciBdf, PciBus, PciDevice, PciFunction, PciSegment, VmId, SHA256_DIGEST_BYTES,
};

use crate::error::{BootCheckError, BootCheckErrorKind};

/// Converts an embedded requirements snapshot into runtime requirements.
pub fn platform_requirements_from_snapshot(
    snapshot: &RequirementsSnapshot,
) -> Result<PlatformRequirements, BootCheckError> {
    if snapshot.arch != REQUIREMENTS_ARCH_X86_64 {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "unsupported requirements snapshot architecture",
        ));
    }
    if snapshot.page_size_count as usize > MAX_REQUIREMENTS_PAGE_SIZES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "requirements snapshot page size count exceeds maximum",
        ));
    }
    if snapshot.expected_pci_count as usize > MAX_REQUIREMENTS_PCI_DEVICES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "requirements snapshot pci device count exceeds maximum",
        ));
    }

    let page_sizes = snapshot
        .page_sizes
        .get(0..snapshot.page_size_count as usize)
        .ok_or(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "requirements snapshot page sizes out of bounds",
        ))?
        .to_vec();
    let expected_pci_devices = snapshot
        .expected_pci
        .get(0..snapshot.expected_pci_count as usize)
        .ok_or(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "requirements snapshot pci devices out of bounds",
        ))?
        .iter()
        .map(expected_pci_from_snapshot)
        .collect();

    Ok(PlatformRequirements {
        arch: hv_config_model::ArchRequirement::X86_64,
        vmx: feature_from_snapshot(snapshot.vmx)?,
        ept: feature_from_snapshot(snapshot.ept)?,
        vtd: feature_from_snapshot(snapshot.vtd)?,
        min_physical_cores: snapshot.min_physical_cores,
        smt_policy: smt_policy_from_snapshot(snapshot.smt_policy)?,
        min_ram_bytes: ByteSize::new(snapshot.min_ram_bytes),
        interrupt_remapping: feature_from_snapshot(snapshot.interrupt_remapping)?,
        x2apic: feature_from_snapshot(snapshot.x2apic)?,
        invariant_tsc: feature_from_snapshot(snapshot.invariant_tsc)?,
        vpid: feature_from_snapshot(snapshot.vpid)?,
        vmx_preemption_timer: feature_from_snapshot(snapshot.vmx_preemption_timer)?,
        nx: feature_from_snapshot(snapshot.nx)?,
        page_sizes: PageSizeSet { sizes: page_sizes },
        expected_pci_devices,
    })
}

/// Builds a requirements snapshot from compiled platform requirements and layout metadata.
pub fn requirements_snapshot_from_platform(
    requirements: &PlatformRequirements,
    config_digest: [u8; SHA256_DIGEST_BYTES],
    hypervisor_reserve_phys: u64,
    hypervisor_reserve_bytes: u64,
) -> Result<RequirementsSnapshot, BootCheckError> {
    if requirements.page_sizes.sizes.len() > MAX_REQUIREMENTS_PAGE_SIZES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "page size count exceeds snapshot capacity",
        ));
    }
    if requirements.expected_pci_devices.len() > MAX_REQUIREMENTS_PCI_DEVICES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "expected pci device count exceeds snapshot capacity",
        ));
    }

    let mut page_sizes = [0u64; MAX_REQUIREMENTS_PAGE_SIZES];
    for (index, size) in requirements.page_sizes.sizes.iter().enumerate() {
        if let Some(slot) = page_sizes.get_mut(index) {
            *slot = *size;
        }
    }

    let mut expected_pci = [ExpectedPciSnapshot {
        vm_id: 0,
        segment: 0,
        bus: 0,
        device: 0,
        function: 0,
        reserved: [0; 3],
    }; MAX_REQUIREMENTS_PCI_DEVICES];
    for (index, device) in requirements.expected_pci_devices.iter().enumerate() {
        if let Some(slot) = expected_pci.get_mut(index) {
            *slot = expected_pci_to_snapshot(device);
        }
    }

    Ok(RequirementsSnapshot {
        arch: REQUIREMENTS_ARCH_X86_64,
        vmx: feature_to_snapshot(requirements.vmx),
        ept: feature_to_snapshot(requirements.ept),
        vtd: feature_to_snapshot(requirements.vtd),
        min_physical_cores: requirements.min_physical_cores,
        smt_policy: smt_policy_to_snapshot(requirements.smt_policy),
        min_ram_bytes: requirements.min_ram_bytes.bytes(),
        interrupt_remapping: feature_to_snapshot(requirements.interrupt_remapping),
        x2apic: feature_to_snapshot(requirements.x2apic),
        invariant_tsc: feature_to_snapshot(requirements.invariant_tsc),
        vpid: feature_to_snapshot(requirements.vpid),
        vmx_preemption_timer: feature_to_snapshot(requirements.vmx_preemption_timer),
        nx: feature_to_snapshot(requirements.nx),
        page_size_count: requirements.page_sizes.sizes.len() as u32,
        page_sizes,
        expected_pci_count: requirements.expected_pci_devices.len() as u32,
        expected_pci,
        hypervisor_reserve_phys,
        hypervisor_reserve_bytes,
        config_digest,
    })
}

fn expected_pci_from_snapshot(snapshot: &ExpectedPciSnapshot) -> ExpectedPciDevice {
    ExpectedPciDevice {
        vm_id: VmId::new(snapshot.vm_id),
        partition_id: String::new(),
        bdf: PciBdf::new(
            PciSegment::new(snapshot.segment),
            PciBus::new(snapshot.bus),
            PciDevice::new(snapshot.device),
            PciFunction::new(snapshot.function),
        ),
        kind: String::new(),
    }
}

fn expected_pci_to_snapshot(device: &ExpectedPciDevice) -> ExpectedPciSnapshot {
    ExpectedPciSnapshot {
        vm_id: device.vm_id.raw(),
        segment: device.bdf.segment.raw(),
        bus: device.bdf.bus.raw(),
        device: device.bdf.device.raw(),
        function: device.bdf.function.raw(),
        reserved: [0; 3],
    }
}

fn feature_from_snapshot(value: u32) -> Result<FeatureRequirement, BootCheckError> {
    match value {
        FEATURE_REQUIRED => Ok(FeatureRequirement::Required),
        FEATURE_PREFERRED => Ok(FeatureRequirement::Preferred),
        FEATURE_OPTIONAL => Ok(FeatureRequirement::Optional),
        FEATURE_DISABLED => Ok(FeatureRequirement::Disabled),
        _ => Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "invalid feature requirement discriminator",
        )),
    }
}

fn feature_to_snapshot(value: FeatureRequirement) -> u32 {
    match value {
        FeatureRequirement::Required => FEATURE_REQUIRED,
        FeatureRequirement::Preferred => FEATURE_PREFERRED,
        FeatureRequirement::Optional => FEATURE_OPTIONAL,
        FeatureRequirement::Disabled => FEATURE_DISABLED,
    }
}

fn smt_policy_from_snapshot(value: u32) -> Result<SmtPolicy, BootCheckError> {
    match value {
        SMT_POLICY_DISABLED => Ok(SmtPolicy::Disabled),
        SMT_POLICY_EXCLUSIVE_CORE => Ok(SmtPolicy::ExclusiveCore),
        SMT_POLICY_SAME_PARTITION_SIBLINGS => Ok(SmtPolicy::SamePartitionSiblings),
        SMT_POLICY_ALLOW_CROSS_PARTITION => Ok(SmtPolicy::AllowCrossPartition),
        _ => Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "invalid smt policy discriminator",
        )),
    }
}

fn smt_policy_to_snapshot(value: SmtPolicy) -> u32 {
    match value {
        SmtPolicy::Disabled => SMT_POLICY_DISABLED,
        SmtPolicy::ExclusiveCore => SMT_POLICY_EXCLUSIVE_CORE,
        SmtPolicy::SamePartitionSiblings => SMT_POLICY_SAME_PARTITION_SIBLINGS,
        SmtPolicy::AllowCrossPartition => SMT_POLICY_ALLOW_CROSS_PARTITION,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    fn reference_reserve() -> (u64, u64) {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        (
            layout.hypervisor_reserve.host_phys.raw(),
            layout.hypervisor_reserve.size.bytes(),
        )
    }

    #[test]
    fn requirements_snapshot_roundtrip_from_reference_config() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let (reserve_phys, reserve_bytes) = reference_reserve();
        let snapshot = requirements_snapshot_from_platform(
            &compiled.requirements,
            compiled.digest.bytes,
            reserve_phys,
            reserve_bytes,
        )
        .expect("snapshot");
        let restored = platform_requirements_from_snapshot(&snapshot).expect("restore");
        assert_eq!(restored.arch, compiled.requirements.arch);
        assert_eq!(restored.vmx, compiled.requirements.vmx);
        assert_eq!(
            restored.min_physical_cores,
            compiled.requirements.min_physical_cores
        );
        assert_eq!(restored.page_sizes, compiled.requirements.page_sizes);
        assert_eq!(
            restored
                .expected_pci_devices
                .iter()
                .map(|device| (device.vm_id, device.bdf))
                .collect::<Vec<_>>(),
            compiled
                .requirements
                .expected_pci_devices
                .iter()
                .map(|device| (device.vm_id, device.bdf))
                .collect::<Vec<_>>()
        );
        assert_eq!(snapshot.config_digest, compiled.digest.bytes);
    }

    #[test]
    fn platform_requirements_from_snapshot_rejects_invalid_metadata() {
        let mut snapshot = RequirementsSnapshot {
            arch: REQUIREMENTS_ARCH_X86_64,
            vmx: FEATURE_REQUIRED,
            ept: FEATURE_REQUIRED,
            vtd: FEATURE_REQUIRED,
            min_physical_cores: 1,
            smt_policy: SMT_POLICY_DISABLED,
            min_ram_bytes: 1,
            interrupt_remapping: FEATURE_REQUIRED,
            x2apic: FEATURE_REQUIRED,
            invariant_tsc: FEATURE_REQUIRED,
            vpid: FEATURE_REQUIRED,
            vmx_preemption_timer: FEATURE_REQUIRED,
            nx: FEATURE_REQUIRED,
            page_size_count: 1,
            page_sizes: [4096, 0, 0, 0],
            expected_pci_count: 0,
            expected_pci: [ExpectedPciSnapshot {
                vm_id: 0,
                segment: 0,
                bus: 0,
                device: 0,
                function: 0,
                reserved: [0; 3],
            }; MAX_REQUIREMENTS_PCI_DEVICES],
            hypervisor_reserve_phys: 0,
            hypervisor_reserve_bytes: 4096,
            config_digest: [0; SHA256_DIGEST_BYTES],
        };
        snapshot.arch = 99;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());

        snapshot.arch = REQUIREMENTS_ARCH_X86_64;
        snapshot.page_size_count = MAX_REQUIREMENTS_PAGE_SIZES as u32 + 1;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());

        snapshot.page_size_count = 1;
        snapshot.expected_pci_count = MAX_REQUIREMENTS_PCI_DEVICES as u32 + 1;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());

        snapshot.expected_pci_count = 0;
        snapshot.vmx = 99;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());

        snapshot.vmx = FEATURE_REQUIRED;
        snapshot.smt_policy = 99;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());
    }

    #[test]
    fn requirements_snapshot_from_platform_rejects_oversized_inputs() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let (reserve_phys, reserve_bytes) = reference_reserve();
        let mut requirements = compiled.requirements.clone();
        requirements.page_sizes.sizes = vec![4096; MAX_REQUIREMENTS_PAGE_SIZES + 1];
        assert!(requirements_snapshot_from_platform(
            &requirements,
            compiled.digest.bytes,
            reserve_phys,
            reserve_bytes,
        )
        .is_err());

        requirements = compiled.requirements.clone();
        let device = compiled
            .requirements
            .expected_pci_devices
            .first()
            .expect("device")
            .clone();
        requirements.expected_pci_devices = (0..=MAX_REQUIREMENTS_PCI_DEVICES)
            .map(|_| device.clone())
            .collect();
        assert!(requirements_snapshot_from_platform(
            &requirements,
            compiled.digest.bytes,
            reserve_phys,
            reserve_bytes,
        )
        .is_err());
    }
}
