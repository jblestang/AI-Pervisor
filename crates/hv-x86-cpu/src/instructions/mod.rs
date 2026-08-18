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
pub mod vtd;
pub mod vmcs;
pub mod vmx;

pub use environment::{
    current_privilege_level, firmware_live_execution_enabled, live_execution_environment_ready,
    live_execution_runtime_enabled,
};
pub use ept::execute_ept_pointer_load;
pub use msr::{read_vmx_basic_msr, vmx_revision_from_basic_msr, IA32_VMX_BASIC};
pub use vtd::{execute_vtd_enable, last_vtd_enable_intent, VtdEnableIntent};
pub use vmcs::execute_vmcs_prepare;
pub use vmx::execute_vmxon;
