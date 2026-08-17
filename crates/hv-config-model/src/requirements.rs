//! Platform requirement extraction from normalized configuration.

use crate::normalize::{NormalizedConfig, NormalizedFeatureLevel, NormalizedSmtPolicy};
use hv_types::{ByteSize, PciBdf, VmId};

/// Supported architecture requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchRequirement {
    /// 64-bit x86.
    X86_64,
}

/// Feature requirement level for platform contract validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureRequirement {
    /// Feature must be present.
    Required,
    /// Feature is preferred.
    Preferred,
    /// Feature is optional.
    Optional,
    /// Feature must be absent.
    Disabled,
}

/// SMT policy for CPU placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtPolicy {
    /// SMT disabled.
    Disabled,
    /// Exclusive physical core ownership.
    ExclusiveCore,
    /// SMT siblings remain within one partition.
    SamePartitionSiblings,
    /// Cross-partition siblings allowed.
    AllowCrossPartition,
}

/// Required page sizes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSizeSet {
    /// Page sizes in bytes, sorted ascending.
    pub sizes: Vec<u64>,
}

/// PCI device expected by the configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedPciDevice {
    /// Owning VM id.
    pub vm_id: VmId,
    /// Partition identifier.
    pub partition_id: String,
    /// Parsed BDF.
    pub bdf: PciBdf,
    /// Device kind string.
    pub kind: String,
}

/// Formal platform requirements derived from desired configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformRequirements {
    /// Required architecture.
    pub arch: ArchRequirement,
    /// VMX requirement.
    pub vmx: FeatureRequirement,
    /// EPT requirement.
    pub ept: FeatureRequirement,
    /// VT-d requirement.
    pub vtd: FeatureRequirement,
    /// Minimum physical core count.
    pub min_physical_cores: u32,
    /// SMT policy.
    pub smt_policy: SmtPolicy,
    /// Minimum platform RAM.
    pub min_ram_bytes: ByteSize,
    /// Interrupt remapping requirement.
    pub interrupt_remapping: FeatureRequirement,
    /// x2APIC requirement.
    pub x2apic: FeatureRequirement,
    /// Invariant TSC requirement.
    pub invariant_tsc: FeatureRequirement,
    /// VPID requirement.
    pub vpid: FeatureRequirement,
    /// VMX preemption timer requirement.
    pub vmx_preemption_timer: FeatureRequirement,
    /// NX requirement.
    pub nx: FeatureRequirement,
    /// Required page sizes.
    pub page_sizes: PageSizeSet,
    /// Expected PCI devices from configuration.
    pub expected_pci_devices: Vec<ExpectedPciDevice>,
}

impl ArchRequirement {
    /// Returns the canonical architecture string for this requirement.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => crate::constants::SUPPORTED_ARCH,
        }
    }
}

/// Builds platform requirements from a normalized configuration.
pub fn platform_requirements(config: &NormalizedConfig) -> PlatformRequirements {
    let mut expected_pci_devices = Vec::new();
    for partition in &config.partitions {
        for device in &partition.devices {
            expected_pci_devices.push(ExpectedPciDevice {
                vm_id: partition.vm_id,
                partition_id: partition.id.clone(),
                bdf: device.bdf,
                kind: device.kind.as_str().to_string(),
            });
        }
    }
    expected_pci_devices.sort_by_key(|device| {
        (
            device.bdf.segment.raw(),
            device.bdf.bus.raw(),
            device.bdf.device.raw(),
            device.bdf.function.raw(),
        )
    });

    PlatformRequirements {
        arch: ArchRequirement::X86_64,
        vmx: convert_feature(config.requirements.vmx),
        ept: convert_feature(config.requirements.ept),
        vtd: convert_feature(config.requirements.vtd),
        min_physical_cores: config.requirements.min_physical_cores,
        smt_policy: convert_smt(config.requirements.smt_policy),
        min_ram_bytes: config.requirements.min_ram_bytes,
        interrupt_remapping: convert_feature(config.requirements.interrupt_remapping),
        x2apic: convert_feature(config.requirements.x2apic),
        invariant_tsc: convert_feature(config.requirements.invariant_tsc),
        vpid: convert_feature(config.requirements.vpid),
        vmx_preemption_timer: convert_feature(config.requirements.vmx_preemption_timer),
        nx: convert_feature(config.requirements.nx),
        page_sizes: PageSizeSet {
            sizes: config.requirements.page_sizes.clone(),
        },
        expected_pci_devices,
    }
}

const fn convert_feature(level: NormalizedFeatureLevel) -> FeatureRequirement {
    match level {
        NormalizedFeatureLevel::Required => FeatureRequirement::Required,
        NormalizedFeatureLevel::Preferred => FeatureRequirement::Preferred,
        NormalizedFeatureLevel::Optional => FeatureRequirement::Optional,
        NormalizedFeatureLevel::Disabled => FeatureRequirement::Disabled,
    }
}

const fn convert_smt(policy: NormalizedSmtPolicy) -> SmtPolicy {
    match policy {
        NormalizedSmtPolicy::Disabled => SmtPolicy::Disabled,
        NormalizedSmtPolicy::ExclusiveCore => SmtPolicy::ExclusiveCore,
        NormalizedSmtPolicy::SamePartitionSiblings => SmtPolicy::SamePartitionSiblings,
        NormalizedSmtPolicy::AllowCrossPartition => SmtPolicy::AllowCrossPartition,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::compile_config_from_str;

    #[test]
    fn platform_requirements_match_reference_config() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let req = platform_requirements(&compiled.normalized);
        assert_eq!(req.arch, ArchRequirement::X86_64);
        assert_eq!(req.min_physical_cores, 3);
        assert_eq!(req.expected_pci_devices.len(), 2);
        assert!(!req.page_sizes.sizes.is_empty());
    }

    #[test]
    fn platform_requirements_cover_all_feature_and_smt_variants() {
        let yaml = include_str!("../tests/fixtures/valid/all_feature_levels.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let req = platform_requirements(&compiled.normalized);
        assert_eq!(req.vmx, FeatureRequirement::Disabled);
        assert_eq!(req.ept, FeatureRequirement::Optional);
        assert_eq!(req.vtd, FeatureRequirement::Preferred);
        assert_eq!(req.smt_policy, SmtPolicy::SamePartitionSiblings);
        assert_eq!(req.invariant_tsc, FeatureRequirement::Optional);
        assert_eq!(req.vpid, FeatureRequirement::Disabled);
    }

    #[test]
    fn platform_requirements_cover_allow_cross_partition_smt() {
        let yaml = include_str!("../tests/fixtures/valid/allow_cross_partition.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let req = platform_requirements(&compiled.normalized);
        assert_eq!(req.smt_policy, SmtPolicy::AllowCrossPartition);
    }

    #[test]
    fn platform_requirements_cover_disabled_smt() {
        let yaml = include_str!("../tests/fixtures/valid/smt_disabled.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let req = platform_requirements(&compiled.normalized);
        assert_eq!(req.smt_policy, SmtPolicy::Disabled);
        assert_eq!(
            compiled.normalized.requirements.smt_policy,
            crate::normalize::NormalizedSmtPolicy::Disabled
        );
    }
}
