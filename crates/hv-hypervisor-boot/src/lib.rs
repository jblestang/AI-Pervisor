//! Portable Gate B boot validation and VMX init orchestration.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod boot;
mod error;
mod snapshot;
mod transfer;
mod vmx;

pub use boot::boot_check;
pub use error::{BootCheckError, BootCheckErrorKind};
pub use snapshot::{platform_requirements_from_snapshot, requirements_snapshot_from_platform};
pub use transfer::{boot_from_transfer, boot_from_transfer_snapshot};
pub use vmx::{boot_check_and_init_vmx, boot_from_transfer_and_init_vmx, BootAndVmxResult};
