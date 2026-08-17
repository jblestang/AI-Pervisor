//! Configuration model, validation, normalization, and static intent IR generation.

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

pub use digest::{config_digest, ConfigDigest};
pub use error::{ConfigError, ConfigErrorKind, ConfigWarning, WarningKind};
pub use intent::{
    BootIntent, CpuPlacementIntent, GuestImageIntent, IpcIntent, MemoryLayoutIntent,
    PartitionIntent, PciDeviceIntent, PciOwnershipIntent, QemuPlanIntent, StaticIntentIR,
};
pub use normalize::NormalizedConfig;
pub use parse::{load_raw_from_path, load_raw_from_str};
pub use pipeline::{
    compile_config, compile_config_from_path, compile_config_from_str, CompiledConfig,
};
pub use raw::RawConfig;
pub use requirements::{
    ArchRequirement, ExpectedPciDevice, FeatureRequirement, PageSizeSet, PlatformRequirements,
    SmtPolicy,
};
