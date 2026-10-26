//! Minimal VT-x bring-up: VMXON, VMCS, exits (EPT / UART / HLT).
//!
//! All MSR / CR / VMX instruction wrappers are intentional TCB `unsafe`
//! (documented in `docs/TCB.md`). Host tests do not execute these paths.

pub mod exit;
pub mod vmcs;
pub mod vmxon;

pub use exit::{ExitAction, GuestRunState, VmxExitReason, handle_exit};
pub use vmcs::{VmcsField, Vcpu, VcpuId};
pub use vmxon::{VmxError, VmxonRegion, vmx_enable_and_on};
