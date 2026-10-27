//! Minimal VT-x bring-up: VMXON, VMCS, exits (EPT / UART / HLT).
//!
//! All MSR / CR / VMX instruction wrappers are intentional TCB `unsafe`
//! (documented in `docs/TCB.md`). Host tests do not execute these paths.

pub mod controls;
pub mod enter;
pub mod exit;
pub mod guest_pt;
pub mod vmcs;
pub mod vmxon;

pub use controls::{AdjustedControls, ControlCaps, ControlWants};
pub use enter::{
    last_guest_gprs, prepare_launch_image, write_guest_gdt, GuestGprs, GuestLaunchImage,
    LaunchError, LaunchTips,
};
pub use exit::{ExitAction, GuestRunState, VmxExitReason, handle_exit};
pub use guest_pt::{pack_guest_pt, spike_guest_maps, GuestMap, GuestPtPack};
pub use vmcs::{VmcsField, Vcpu, VcpuId};
pub use vmxon::{VmxError, VmxonRegion, vmx_enable_and_on};
