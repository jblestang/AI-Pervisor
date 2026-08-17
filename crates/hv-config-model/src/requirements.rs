//! Platform requirement extraction from normalized configuration.

use crate::normalize::{
    NormalizedConfig, NormalizedDeviceKind, NormalizedFeatureLevel, NormalizedSmtPolicy,
};
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

/// Builds platform requirements from a normalized configuration.
pub fn platform_requirements(config: &NormalizedConfig) -> PlatformRequirements {
    let mut expected_pci_devices = Vec::new();
    for partition in &config.partitions {
        for device in &partition.devices {
            expected_pci_devices.push(ExpectedPciDevice {
                vm_id: partition.vm_id,
                partition_id: partition.id.clone(),
                bdf: device.bdf,
                kind: match device.kind {
                    NormalizedDeviceKind::NicE1000 => "nic_e1000".to_string(),
                },
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
