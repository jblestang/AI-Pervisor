//! VMX initialization planning and backend abstraction.

#![cfg_attr(not(test), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

mod backend;
mod constants;
mod error;
mod init;
mod launch;
mod launch_constants;
mod plan;
mod program;

pub use backend::{FailingVmxBackend, MockVmxBackend, VmxBackend};
pub use constants::{VMXON_REGION_ALIGNMENT_BYTES, VMXON_REGION_MIN_BYTES};
pub use error::{VmxError, VmxErrorKind};
pub use init::{init_vmx, vmx_init_required};
pub use launch::{
    patch_guest_entry_in_fields, plan_vmx_launch, plan_vmx_launch_all_partitions,
    program_vmcs_fields, VmcsProgrammedField, VmcsProgrammedFields, VmxLaunchPlan,
    DEFAULT_SMOKE_GUEST_PARTITION_ID,
};
pub use launch_constants::{
    VMCS_CPU_BASED_VM_EXEC_CONTROL, VMCS_GUEST_CR3, VMCS_GUEST_RIP, VMCS_GUEST_RSP,
    VMCS_HOST_CR3, VMCS_HOST_RIP, VMCS_HOST_RSP, VMCS_PIN_BASED_VM_EXEC_CONTROL,
    VMCS_SECONDARY_VM_EXEC_CONTROL, VMCS_VM_ENTRY_CONTROLS, VMCS_VM_EXIT_CONTROLS,
};
pub use plan::{plan_vmx_init, VmxInitPlan};
pub use program::{
    program_vmxon_region, ProgrammingVmxBackend, VmxonProgrammedRegion, REFERENCE_VMXON_REVISION,
};
