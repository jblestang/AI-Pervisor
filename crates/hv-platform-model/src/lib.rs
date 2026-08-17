//! Observed platform validation and static platform IR planning (Gate B foundation).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

mod constants;
mod cpuid_constants;
mod error;
mod observe;
mod observed;
mod planner;
mod platform_ir;
mod validate;
mod validated;

pub use constants::{platform_phys_base, PLATFORM_PHYS_BASE, REGION_ALIGNMENT_BYTES};
pub use cpuid_constants::DEFAULT_PAGE_SIZES;
pub use error::{PlatformError, PlatformErrorKind, PlatformWarning};
pub use hv_observation_types::{
    CpuidSnapshot, ObservationInputs, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT,
    CPUID_1_EDX_NX_BIT, CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT,
    CPUID_480_ECX_VPID_BIT, CPUID_80000007_EDX_INVARIANT_TSC_BIT,
};
pub use observe::observe_platform;
pub use observed::{parse_observed_platform_json, ObservedPlatform};
pub use planner::plan_static_platform_ir;
pub use platform_ir::{
    PlannedGuestMemory, PlannedHypervisorReserve, PlannedIpcMemory, PlannedPciDevice,
    StaticPlatformIR,
};
pub use validate::validate_platform;
pub use validated::ValidatedPlatform;
