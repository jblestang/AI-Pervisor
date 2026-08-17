//! Canonical normalized configuration representation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, ConfigErrorKind};
use crate::pci::parse_bdf;
use crate::raw::{
    RawConfig, RawDeviceKind, RawDeviceRole, RawFeatureLevel, RawGuestImage, RawIpcChannel,
    RawPartition, RawPlatformRequirements, RawQemu, RawSmtPolicy,
};
use hv_types::{ByteSize, Gibibyte, IpcChannelId, Mebibyte, PciBdf, VmId};

/// Canonical normalized configuration used for hashing and IR generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedConfig {
    /// Schema version.
    pub schema_version: u32,
    /// Platform profile name.
    pub platform_name: String,
    /// Platform requirements in canonical form.
    pub requirements: NormalizedRequirements,
    /// Security policy.
    pub security: NormalizedSecurity,
    /// Canonical partition list sorted by id.
    pub partitions: Vec<NormalizedPartition>,
    /// Canonical IPC channel list sorted by id.
    pub ipc_channels: Vec<NormalizedIpcChannel>,
    /// Boot metadata sorted by partition id.
    pub boot: NormalizedBoot,
    /// QEMU launch plan.
    pub qemu: NormalizedQemu,
}

/// Normalized platform requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedRequirements {
    /// Target architecture.
    pub arch: String,
    /// VMX requirement.
    pub vmx: NormalizedFeatureLevel,
    /// EPT requirement.
    pub ept: NormalizedFeatureLevel,
    /// VT-d requirement.
    pub vtd: NormalizedFeatureLevel,
    /// Minimum physical cores.
    pub min_physical_cores: u32,
    /// Minimum RAM in bytes.
    pub min_ram_bytes: ByteSize,
    /// SMT policy.
    pub smt_policy: NormalizedSmtPolicy,
    /// Interrupt remapping requirement.
    pub interrupt_remapping: NormalizedFeatureLevel,
    /// x2APIC requirement.
    pub x2apic: NormalizedFeatureLevel,
    /// Invariant TSC requirement.
    pub invariant_tsc: NormalizedFeatureLevel,
    /// VPID requirement.
    pub vpid: NormalizedFeatureLevel,
    /// VMX preemption timer requirement.
    pub vmx_preemption_timer: NormalizedFeatureLevel,
    /// NX requirement.
    pub nx: NormalizedFeatureLevel,
    /// Required page sizes in bytes, sorted ascending.
    pub page_sizes: Vec<u64>,
}

/// Normalized feature requirement level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedFeatureLevel {
    /// Required feature.
    Required,
    /// Preferred feature.
    Preferred,
    /// Optional feature.
    Optional,
    /// Disabled feature.
    Disabled,
}

/// Normalized SMT policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedSmtPolicy {
    /// Disabled.
    Disabled,
    /// Exclusive core ownership.
    ExclusiveCore,
    /// Same-partition siblings only.
    SamePartitionSiblings,
    /// Cross-partition siblings allowed.
    AllowCrossPartition,
}

/// Normalized security policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedSecurity {
    /// Require a processing partition on the datapath.
    pub require_mid_in_datapath: bool,
}

/// Normalized partition definition with deterministic VM id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedPartition {
    /// Stable partition identifier from YAML.
    pub id: String,
    /// Deterministic VM identifier.
    pub vm_id: VmId,
    /// Number of vCPUs.
    pub vcpus: u32,
    /// Private RAM size.
    pub memory_bytes: ByteSize,
    /// Reserved physical cores sorted ascending.
    pub physical_cores: Vec<u32>,
    /// Assigned devices sorted by BDF.
    pub devices: Vec<NormalizedDevice>,
}

/// Normalized device assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedDevice {
    /// Device kind.
    pub kind: NormalizedDeviceKind,
    /// Parsed PCI BDF.
    pub bdf: PciBdf,
    /// Optional datapath role.
    pub role: Option<NormalizedDeviceRole>,
}

/// Normalized device kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedDeviceKind {
    /// Intel e1000 NIC.
    NicE1000,
}

impl NormalizedDeviceKind {
    /// Returns the canonical snake_case identifier for this device kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NicE1000 => "nic_e1000",
        }
    }
}

/// Normalized datapath role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedDeviceRole {
    /// Ingress NIC.
    DatapathIn,
    /// Egress NIC.
    DatapathOut,
}

impl NormalizedDeviceRole {
    /// Returns the canonical snake_case identifier for this datapath role.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DatapathIn => "datapath_in",
            Self::DatapathOut => "datapath_out",
        }
    }
}

/// Normalized IPC channel with deterministic ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedIpcChannel {
    /// Stable channel identifier from YAML.
    pub id: String,
    /// Deterministic IPC channel id.
    pub channel_id: IpcChannelId,
    /// Producer partition id.
    pub producer: String,
    /// Producer VM id.
    pub producer_vm_id: VmId,
    /// Consumer partition id.
    pub consumer: String,
    /// Consumer VM id.
    pub consumer_vm_id: VmId,
    /// Queue slot count.
    pub queue_slots: u32,
    /// Slot payload size in bytes.
    pub slot_size_bytes: u32,
}

/// Normalized boot metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedBoot {
    /// Guest images sorted by partition id.
    pub guest_images: Vec<NormalizedGuestImage>,
}

/// Normalized guest image entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedGuestImage {
    /// Target partition id.
    pub partition: String,
    /// Target VM id.
    pub vm_id: VmId,
    /// Image path.
    pub path: String,
    /// Expected SHA-256 hash.
    pub sha256: String,
}

/// Normalized QEMU plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedQemu {
    /// Machine type.
    pub machine: String,
    /// CPU count.
    pub cpus: u32,
    /// Memory in bytes.
    pub memory_bytes: ByteSize,
    /// CPU model.
    pub cpu_model: String,
    /// Accelerator.
    pub accel: String,
    /// Socket count.
    pub smp_sockets: u32,
    /// Core count per socket.
    pub smp_cores: u32,
    /// Thread count per core.
    pub smp_threads: u32,
}

/// Normalizes a validated raw configuration into canonical form.
pub fn normalize(raw: RawConfig) -> Result<NormalizedConfig, ConfigError> {
    let mut partitions = raw.partitions;
    partitions.sort_by(|a, b| a.id.cmp(&b.id));

    let vm_id_map = build_vm_id_map(&partitions)?;

    let normalized_partitions = partitions
        .into_iter()
        .map(|partition| normalize_partition(partition, &vm_id_map))
        .collect::<Result<Vec<_>, _>>()?;

    let mut ipc_channels = raw.ipc_channels;
    ipc_channels.sort_by(|a, b| a.id.cmp(&b.id));
    let normalized_ipc = ipc_channels
        .into_iter()
        .enumerate()
        .map(|(index, channel)| normalize_ipc(channel, index as u32, &vm_id_map))
        .collect::<Result<Vec<_>, _>>()?;

    let mut guest_images = raw.boot.guest_images;
    guest_images.sort_by(|a, b| a.partition.cmp(&b.partition));
    let normalized_guest_images = guest_images
        .into_iter()
        .map(|image| normalize_guest_image(image, &vm_id_map))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(NormalizedConfig {
        schema_version: raw.schema_version,
        platform_name: raw.platform.name,
        requirements: normalize_requirements(&raw.platform.requirements)?,
        security: NormalizedSecurity {
            require_mid_in_datapath: raw.security.require_mid_in_datapath,
        },
        partitions: normalized_partitions,
        ipc_channels: normalized_ipc,
        boot: NormalizedBoot {
            guest_images: normalized_guest_images,
        },
        qemu: normalize_qemu(&raw.qemu)?,
    })
}

fn build_vm_id_map(partitions: &[RawPartition]) -> Result<HashMap<String, VmId>, ConfigError> {
    let mut map = HashMap::new();
    for (index, partition) in partitions.iter().enumerate() {
        map.insert(partition.id.clone(), VmId::new(index as u32));
    }
    Ok(map)
}

fn normalize_partition(
    partition: RawPartition,
    vm_id_map: &HashMap<String, VmId>,
) -> Result<NormalizedPartition, ConfigError> {
    let vm_id = *vm_id_map
        .get(&partition.id)
        .ok_or_else(|| ConfigError::new(ConfigErrorKind::Internal, "missing vm id mapping"))?;

    let memory_bytes = Gibibyte::new(partition.memory_gib)
        .to_bytes()
        .map_err(|_| {
            ConfigError::new(ConfigErrorKind::Arithmetic, "partition memory overflow")
                .with_path(format!("partitions[id={}].memory_gib", partition.id))
        })?;

    let mut physical_cores = partition.cpu_affinity.physical_cores;
    physical_cores.sort_unstable();

    let mut devices = partition
        .devices
        .into_iter()
        .map(|device| {
            let bdf = parse_bdf(&device.bdf).map_err(|err| {
                err.with_path(format!("partitions[id={}].devices.bdf", partition.id))
            })?;
            Ok(NormalizedDevice {
                kind: match device.kind {
                    RawDeviceKind::NicE1000 => NormalizedDeviceKind::NicE1000,
                },
                bdf,
                role: device.role.map(|role| match role {
                    RawDeviceRole::DatapathIn => NormalizedDeviceRole::DatapathIn,
                    RawDeviceRole::DatapathOut => NormalizedDeviceRole::DatapathOut,
                }),
            })
        })
        .collect::<Result<Vec<_>, ConfigError>>()?;
    devices.sort_by_key(|device| {
        (
            device.bdf.segment.raw(),
            device.bdf.bus.raw(),
            device.bdf.device.raw(),
            device.bdf.function.raw(),
        )
    });

    Ok(NormalizedPartition {
        id: partition.id,
        vm_id,
        vcpus: partition.vcpus,
        memory_bytes,
        physical_cores,
        devices,
    })
}

fn normalize_ipc(
    channel: RawIpcChannel,
    index: u32,
    vm_id_map: &HashMap<String, VmId>,
) -> Result<NormalizedIpcChannel, ConfigError> {
    let producer_vm_id = *vm_id_map
        .get(&channel.producer)
        .ok_or_else(|| ConfigError::new(ConfigErrorKind::Internal, "missing producer vm id"))?;
    let consumer_vm_id = *vm_id_map
        .get(&channel.consumer)
        .ok_or_else(|| ConfigError::new(ConfigErrorKind::Internal, "missing consumer vm id"))?;

    Ok(NormalizedIpcChannel {
        id: channel.id,
        channel_id: IpcChannelId::new(index),
        producer: channel.producer,
        producer_vm_id,
        consumer: channel.consumer,
        consumer_vm_id,
        queue_slots: channel.queue_slots,
        slot_size_bytes: channel.slot_size_bytes,
    })
}

fn normalize_guest_image(
    image: RawGuestImage,
    vm_id_map: &HashMap<String, VmId>,
) -> Result<NormalizedGuestImage, ConfigError> {
    let vm_id = *vm_id_map
        .get(&image.partition)
        .ok_or_else(|| ConfigError::new(ConfigErrorKind::Internal, "missing guest image vm id"))?;
    Ok(NormalizedGuestImage {
        partition: image.partition,
        vm_id,
        path: image.path,
        sha256: image.sha256.to_ascii_lowercase(),
    })
}

fn normalize_requirements(
    raw: &RawPlatformRequirements,
) -> Result<NormalizedRequirements, ConfigError> {
    let min_ram_bytes = Gibibyte::new(raw.min_ram_gib).to_bytes().map_err(|_| {
        ConfigError::new(ConfigErrorKind::Arithmetic, "min_ram_gib overflow")
            .with_path("platform.requirements.min_ram_gib")
    })?;

    let mut page_sizes = raw.page_sizes.clone();
    page_sizes.sort_unstable();
    page_sizes.dedup();

    Ok(NormalizedRequirements {
        arch: raw.arch.clone(),
        vmx: convert_feature(raw.vmx),
        ept: convert_feature(raw.ept),
        vtd: convert_feature(raw.vtd),
        min_physical_cores: raw.min_physical_cores,
        min_ram_bytes,
        smt_policy: convert_smt(raw.smt_policy),
        interrupt_remapping: convert_feature(raw.interrupt_remapping),
        x2apic: convert_feature(raw.x2apic),
        invariant_tsc: convert_feature(raw.invariant_tsc),
        vpid: convert_feature(raw.vpid),
        vmx_preemption_timer: convert_feature(raw.vmx_preemption_timer),
        nx: convert_feature(raw.nx),
        page_sizes,
    })
}

fn normalize_qemu(raw: &RawQemu) -> Result<NormalizedQemu, ConfigError> {
    let memory_bytes = Mebibyte::new(u64::from(raw.memory_mib)).to_bytes();
    Ok(NormalizedQemu {
        machine: raw.machine.clone(),
        cpus: raw.cpus,
        memory_bytes,
        cpu_model: raw.cpu_model.clone(),
        accel: raw.accel.clone(),
        smp_sockets: raw.smp_sockets,
        smp_cores: raw.smp_cores,
        smp_threads: raw.smp_threads,
    })
}

const fn convert_feature(level: RawFeatureLevel) -> NormalizedFeatureLevel {
    match level {
        RawFeatureLevel::Required => NormalizedFeatureLevel::Required,
        RawFeatureLevel::Preferred => NormalizedFeatureLevel::Preferred,
        RawFeatureLevel::Optional => NormalizedFeatureLevel::Optional,
        RawFeatureLevel::Disabled => NormalizedFeatureLevel::Disabled,
    }
}

const fn convert_smt(policy: RawSmtPolicy) -> NormalizedSmtPolicy {
    match policy {
        RawSmtPolicy::Disabled => NormalizedSmtPolicy::Disabled,
        RawSmtPolicy::ExclusiveCore => NormalizedSmtPolicy::ExclusiveCore,
        RawSmtPolicy::SamePartitionSiblings => NormalizedSmtPolicy::SamePartitionSiblings,
        RawSmtPolicy::AllowCrossPartition => NormalizedSmtPolicy::AllowCrossPartition,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::parse::load_raw_from_str;
    use crate::semantic::validate_semantics;
    use crate::syntax::validate_syntax;

    #[test]
    fn normalization_is_deterministic() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let raw_a = load_raw_from_str(yaml).expect("parse");
        validate_syntax(&raw_a).expect("syntax");
        validate_semantics(&raw_a).expect("semantic");
        let norm_a = normalize(raw_a.clone()).expect("normalize");

        let raw_b = load_raw_from_str(yaml).expect("parse");
        let norm_b = normalize(raw_b).expect("normalize");
        assert_eq!(norm_a, norm_b);
    }
}
