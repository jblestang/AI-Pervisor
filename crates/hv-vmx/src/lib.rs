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
mod plan;

pub use backend::{FailingVmxBackend, MockVmxBackend, VmxBackend};
pub use constants::{VMXON_REGION_ALIGNMENT_BYTES, VMXON_REGION_MIN_BYTES};
pub use error::{VmxError, VmxErrorKind};
pub use init::{init_vmx, vmx_init_required};
pub use plan::{plan_vmx_init, VmxInitPlan};
