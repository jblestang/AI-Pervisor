//! Live privileged instruction execution for Gate C hardware bring-up.

pub mod environment;
pub mod ept;
pub mod msr;
pub mod vtd;
pub mod vmx;

pub use environment::{
    current_privilege_level, live_execution_environment_ready, live_execution_runtime_enabled,
};
pub use ept::execute_ept_pointer_load;
pub use msr::{read_vmx_basic_msr, vmx_revision_from_basic_msr, IA32_VMX_BASIC};
pub use vtd::{execute_vtd_enable, last_vtd_enable_intent, VtdEnableIntent};
pub use vmx::execute_vmxon;
