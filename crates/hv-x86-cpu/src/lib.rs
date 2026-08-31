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
mod constants;
mod cpuid;
mod error;
mod instructions;
mod resident;
mod resident_backends;
mod seams;

pub use backends::{CpuSeamEptBackend, CpuSeamVmxBackend, CpuSeamVtdBackend};
pub use constants::{
    CR4_VMXE_BIT, HV_X86_LIVE_INSTRUCTIONS_DISABLED, HV_X86_LIVE_INSTRUCTIONS_ENABLED,
    HV_X86_LIVE_INSTRUCTIONS_ENV, HV_X86_LIVE_VMXON_UNAVAILABLE, HV_X86_LIVE_VTD_UNAVAILABLE,
    VMCS_EPT_POINTER_FIELD, VMXON_REVISION_PREFIX_BYTES, X86_CPL_MASK, X86_RING_0,
};
pub use error::{CpuSeamError, CpuSeamErrorKind};
pub use instructions::{
    current_privilege_level, execute_ept_pointer_load, execute_vmxon, execute_vtd_enable,
    execute_vmcs_prepare, execute_vmcs_field_programming, execute_vmlaunch,
    firmware_live_execution_enabled, last_vtd_enable_intent,
    live_execution_environment_ready, live_execution_runtime_enabled, read_vmx_basic_msr,
    vmx_revision_from_basic_msr, VtdEnableIntent, IA32_VMX_BASIC,
};
pub use resident::{
    install_ept_tables, install_guest_image, install_vmxon_region, install_vmcs_region,
    resolve_vmxon_revision, MockPageAllocator, PageAllocator, VMCS_REGION_BYTES,
};
#[cfg(feature = "datapath-guests")]
pub use resident::install_guest_elf;
pub use resident_backends::{
    ResidentCpuSeamEptBackend, ResidentCpuSeamVmxBackend, ResidentCpuSeamVtdBackend,
};
pub use seams::{
    run_ept_pointer_cpu_seam, run_vmxon_cpu_seam, run_vtd_enable_cpu_seam,
    run_vmx_launch_cpu_seam, CpuInstructionDisposition, EptCpuSeamOutcome, VmxCpuSeamOutcome,
    VmxLaunchCpuSeamOutcome, VtdCpuSeamOutcome,
};
#[cfg(feature = "datapath-guests")]
pub use seams::{run_multi_vmx_launch_cpu_seam, MultiVmxLaunchCpuSeamOutcome};
#[cfg(feature = "datapath-live")]
pub use seams::{run_datapath_live_cpu_seam, DatapathLiveCpuSeamOutcome};
#[cfg(feature = "datapath-runtime")]
pub use seams::{run_datapath_runtime_cpu_seam, DatapathRuntimeCpuSeamOutcome};
