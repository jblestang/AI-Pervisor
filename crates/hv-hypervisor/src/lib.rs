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

pub use boot::boot_check;
pub use error::{BootCheckError, BootCheckErrorKind};
