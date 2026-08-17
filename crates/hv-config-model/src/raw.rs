//! Raw configuration types mirroring the YAML schema.

use serde::{Deserialize, Serialize};

/// Supported schema version for the configuration file.
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// Top-level raw configuration document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    /// Schema version number.
    pub schema_version: u32,
    /// Platform description and requirements.
    pub platform: RawPlatform,
    /// Security-related policy knobs.
    pub security: RawSecurity,
    /// QEMU-specific launch parameters.
    pub qemu: RawQemu,
    /// Static partition definitions.
    pub partitions: Vec<RawPartition>,
    /// IPC channel definitions.
    pub ipc_channels: Vec<RawIpcChannel>,
    /// Boot image metadata.
    pub boot: RawBoot,
}

/// Platform metadata and requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawPlatform {
    /// Human-readable platform profile name.
    pub name: String,
    /// Required platform capabilities.
    pub requirements: RawPlatformRequirements,
}

/// Platform capability requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawPlatformRequirements {
    /// Target architecture string.
    pub arch: String,
    /// VMX requirement level.
    pub vmx: RawFeatureLevel,
    /// EPT requirement level.
    pub ept: RawFeatureLevel,
    /// VT-d requirement level.
    pub vtd: RawFeatureLevel,
    /// Minimum number of physical cores.
    pub min_physical_cores: u32,
    /// Minimum platform RAM in gibibytes.
    pub min_ram_gib: u64,
    /// SMT policy.
    pub smt_policy: RawSmtPolicy,
    /// Interrupt remapping requirement level.
    pub interrupt_remapping: RawFeatureLevel,
    /// x2APIC requirement level.
    pub x2apic: RawFeatureLevel,
    /// Invariant TSC requirement level.
    pub invariant_tsc: RawFeatureLevel,
    /// VPID requirement level.
    pub vpid: RawFeatureLevel,
    /// VMX preemption timer requirement level.
    pub vmx_preemption_timer: RawFeatureLevel,
    /// NX requirement level.
    pub nx: RawFeatureLevel,
    /// Required guest/host page sizes in bytes.
    pub page_sizes: Vec<u64>,
}

/// Feature requirement level in YAML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawFeatureLevel {
    /// Feature must be present.
    Required,
    /// Feature is preferred but not mandatory.
    Preferred,
    /// Feature is optional.
    Optional,
    /// Feature must be absent/disabled.
    Disabled,
}

/// SMT policy in YAML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawSmtPolicy {
    /// SMT disabled platform-wide.
    Disabled,
    /// Each physical core is exclusively owned by one partition.
    ExclusiveCore,
    /// SMT siblings must remain in the same partition.
    SamePartitionSiblings,
    /// Cross-partition SMT siblings are allowed.
    AllowCrossPartition,
}

/// Security policy section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawSecurity {
    /// Require a processing partition between datapath ingress and egress NICs.
    pub require_mid_in_datapath: bool,
}

/// QEMU launch parameters derived from configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawQemu {
    /// Machine type.
    pub machine: String,
    /// Total vCPU count presented to QEMU.
    pub cpus: u32,
    /// Guest memory in mebibytes.
    pub memory_mib: u32,
    /// CPU model string.
    pub cpu_model: String,
    /// Accelerator backend.
    pub accel: String,
    /// Number of sockets.
    pub smp_sockets: u32,
    /// Number of cores per socket.
    pub smp_cores: u32,
    /// Number of threads per core.
    pub smp_threads: u32,
}

/// Raw partition definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawPartition {
    /// Stable partition identifier.
    pub id: String,
    /// Number of vCPUs for the partition.
    pub vcpus: u32,
    /// Private RAM in gibibytes.
    pub memory_gib: u64,
    /// CPU affinity declaration.
    pub cpu_affinity: RawCpuAffinity,
    /// Devices assigned to the partition.
    pub devices: Vec<RawDevice>,
}

/// CPU affinity declaration for a partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawCpuAffinity {
    /// Physical core indices reserved for the partition.
    pub physical_cores: Vec<u32>,
}

/// Device assigned to a partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawDevice {
    /// Device kind string.
    pub kind: RawDeviceKind,
    /// PCI BDF in `SSSS:BB:DD.F` or `BB:DD.F` form.
    pub bdf: String,
    /// Optional datapath role marker.
    #[serde(default)]
    pub role: Option<RawDeviceRole>,
}

/// Supported device kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawDeviceKind {
    /// Intel e1000 NIC.
    NicE1000,
}

/// Datapath role marker for NIC devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawDeviceRole {
    /// External ingress NIC.
    DatapathIn,
    /// External egress NIC.
    DatapathOut,
}

/// Raw IPC channel definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawIpcChannel {
    /// Stable IPC channel identifier.
    pub id: String,
    /// Producer partition id.
    pub producer: String,
    /// Consumer partition id.
    pub consumer: String,
    /// Number of queue slots.
    pub queue_slots: u32,
    /// Slot payload size in bytes.
    pub slot_size_bytes: u32,
}

/// Boot image metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawBoot {
    /// Guest images indexed by partition id.
    pub guest_images: Vec<RawGuestImage>,
}

/// Guest image entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawGuestImage {
    /// Target partition id.
    pub partition: String,
    /// Relative path to the guest ELF image.
    pub path: String,
    /// Expected SHA-256 hash of the image.
    pub sha256: String,
}
