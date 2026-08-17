//! Observed platform validation and static platform IR planning (Gate B foundation).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

mod constants;
mod error;
mod observed;
mod planner;
mod platform_ir;
mod validate;
mod validated;

pub use constants::{platform_phys_base, PLATFORM_PHYS_BASE, REGION_ALIGNMENT_BYTES};
pub use error::{PlatformError, PlatformErrorKind, PlatformWarning};
pub use observed::{parse_observed_platform_json, ObservedPlatform};
pub use planner::plan_static_platform_ir;
pub use platform_ir::{
    PlannedGuestMemory, PlannedHypervisorReserve, PlannedIpcMemory, PlannedPciDevice,
    StaticPlatformIR,
};
pub use validate::validate_platform;
pub use validated::ValidatedPlatform;
