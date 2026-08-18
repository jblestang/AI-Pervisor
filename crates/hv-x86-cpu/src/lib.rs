//! x86 CPU instruction seams for Gate C hardware bring-up.
//!
//! Host-only crate: validates CPU capabilities and records instruction disposition.
//! Default builds do not execute privileged VMX/EPT/VT-d instructions (CI-safe).

#![cfg_attr(not(test), no_std)]
#![allow(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod backends;
mod cpuid;
mod error;
mod instructions;
mod seams;

pub use backends::{CpuSeamEptBackend, CpuSeamVmxBackend, CpuSeamVtdBackend};
pub use cpuid::{cpuid_ept_available, cpuid_vmx_available, cpuid_vtd_available};
pub use error::{CpuSeamError, CpuSeamErrorKind};
pub use instructions::{
    current_privilege_level, execute_ept_pointer_load, execute_vmxon, execute_vtd_enable,
    last_vtd_enable_intent, live_execution_environment_ready, live_execution_runtime_enabled,
    read_vmx_basic_msr, vmx_revision_from_basic_msr, VtdEnableIntent, IA32_VMX_BASIC,
};
pub use seams::{
    run_ept_pointer_cpu_seam, run_vmxon_cpu_seam, run_vtd_enable_cpu_seam, CpuInstructionDisposition,
    EptCpuSeamOutcome, VmxCpuSeamOutcome, VtdCpuSeamOutcome,
};
