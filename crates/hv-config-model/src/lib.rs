//! Configuration model, validation, normalization, and static intent IR generation.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

mod constants;
mod requirements;

#[cfg(feature = "std")]
mod digest;
#[cfg(feature = "std")]
mod error;
#[cfg(feature = "std")]
mod intent;
#[cfg(feature = "std")]
mod normalize;
#[cfg(feature = "std")]
mod parse;
#[cfg(feature = "std")]
mod pci;
#[cfg(feature = "std")]
mod pipeline;
#[cfg(feature = "std")]
mod raw;
#[cfg(feature = "std")]
mod semantic;
#[cfg(feature = "std")]
mod syntax;

pub use constants::{
    hypervisor_reserve_bytes, HYPERVISOR_RESERVE_MIB, IPC_SLOT_METADATA_BYTES, SUPPORTED_ARCH,
};
pub use requirements::{
    ArchRequirement, ExpectedPciDevice, FeatureRequirement, PageSizeSet, PlatformRequirements,
    SmtPolicy,
};

#[cfg(feature = "std")]
pub use digest::{config_digest, ConfigDigest};
#[cfg(feature = "std")]
pub use error::{ConfigError, ConfigErrorKind, ConfigWarning, WarningKind};
#[cfg(feature = "std")]
pub use intent::{
    BootIntent, CpuPlacementIntent, GuestImageIntent, IpcIntent, MemoryLayoutIntent,
    PartitionIntent, PciDeviceIntent, PciOwnershipIntent, QemuNetworkInterfaceIntent,
    QemuNetworkPlanIntent, QemuPlanIntent, StaticIntentIR,
};
#[cfg(feature = "std")]
pub use normalize::{normalize, NormalizedConfig, NormalizedSmtPolicy};
#[cfg(feature = "std")]
pub use parse::{load_raw_from_path, load_raw_from_str};
#[cfg(feature = "std")]
pub use pci::{parse_bdf, parse_guest_phys};
#[cfg(feature = "std")]
pub use pipeline::{
    compile_config, compile_config_from_path, compile_config_from_str, CompiledConfig,
};
#[cfg(feature = "std")]
pub use raw::RawConfig;
#[cfg(feature = "std")]
pub use requirements::platform_requirements;
#[cfg(feature = "std")]
pub use semantic::validate_semantics;
#[cfg(feature = "std")]
pub use syntax::validate_syntax;
