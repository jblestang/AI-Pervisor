//! Hypervisor boot-path validation for Gate B.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

mod boot;
mod error;
mod snapshot;
mod transfer;

pub use boot::boot_check;
pub use error::{BootCheckError, BootCheckErrorKind};
pub use snapshot::{platform_requirements_from_snapshot, requirements_snapshot_from_platform};
pub use transfer::{boot_from_transfer, boot_from_transfer_snapshot};
