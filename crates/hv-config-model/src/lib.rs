//! Configuration model, validation, normalization, and static intent IR generation.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

mod constants;
mod digest;
mod error;
mod intent;
mod normalize;
mod parse;
mod pci;
mod pipeline;
mod raw;
mod requirements;
mod semantic;
mod syntax;

pub use constants::{
    hypervisor_reserve_bytes, HYPERVISOR_RESERVE_MIB, IPC_SLOT_METADATA_BYTES, SUPPORTED_ARCH,
};
pub use digest::{config_digest, ConfigDigest};
pub use error::{ConfigError, ConfigErrorKind, ConfigWarning, WarningKind};
pub use intent::{
    BootIntent, CpuPlacementIntent, GuestImageIntent, IpcIntent, MemoryLayoutIntent,
    PartitionIntent, PciDeviceIntent, PciOwnershipIntent, QemuPlanIntent, StaticIntentIR,
};
pub use normalize::{normalize, NormalizedConfig, NormalizedSmtPolicy};
pub use parse::{load_raw_from_path, load_raw_from_str};
pub use pipeline::{
    compile_config, compile_config_from_path, compile_config_from_str, CompiledConfig,
};
pub use raw::RawConfig;
pub use requirements::{
    platform_requirements, ArchRequirement, ExpectedPciDevice, FeatureRequirement, PageSizeSet,
    PlatformRequirements, SmtPolicy,
};
pub use semantic::validate_semantics;
pub use syntax::validate_syntax;
