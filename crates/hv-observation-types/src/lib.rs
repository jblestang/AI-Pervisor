//! Boot-time observation input types for loader handoff and platform observation.

#![no_std]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

mod cpuid;
mod inputs;

pub use cpuid::{
    CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
    CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
    CPUID_80000007_EDX_INVARIANT_TSC_BIT,
};
pub use inputs::ObservationInputs;
