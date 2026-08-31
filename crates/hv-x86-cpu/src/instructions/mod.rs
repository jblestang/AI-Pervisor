//! Live privileged instruction execution for Gate C hardware bring-up.

pub mod environment;
pub mod ept;
#[cfg(all(
    target_arch = "x86_64",
    feature = "execute-instructions",
    not(test),
    not(coverage)
))]
mod live_asm;
pub mod msr;
pub mod tsc;
pub mod vmcs;
pub mod vmcs_fields;
pub mod vmexit_stub;
pub mod vmlaunch;
pub mod vmx;
pub mod vmx_guest_run;
pub mod vtd;

pub use environment::{
    current_privilege_level, firmware_live_execution_enabled, live_execution_environment_ready,
    live_execution_runtime_enabled,
};
pub use ept::{execute_ept_pointer_load, execute_invept_single_context};
pub use msr::{read_vmx_basic_msr, vmx_revision_from_basic_msr, IA32_VMX_BASIC};
pub use tsc::{hypervisor_elapsed_tsc, read_timestamp_counter, validate_hypervisor_tsc_bracket};
pub use vmcs::execute_vmcs_prepare;
pub use vmcs_fields::execute_vmcs_field_programming;
pub use vmlaunch::execute_vmlaunch;
pub use vmx::execute_vmxon;
pub use vtd::{execute_vtd_enable, last_vtd_enable_intent, VtdEnableIntent};
