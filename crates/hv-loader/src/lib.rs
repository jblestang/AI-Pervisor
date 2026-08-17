//! UEFI loader handoff builder for the static hypervisor boot path.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

mod build;
mod constants;
mod error;
mod handoff;

pub use build::{build_boot_info_blob, BootInfoSection};

pub use constants::{
    DEFAULT_MEMORY_DESCRIPTOR_SIZE, MEMORY_MAP_KIND, RSDP_KIND,
};
pub use error::{LoaderError, LoaderErrorKind};
pub use handoff::{build_loader_handoff, LoaderHandoff, LoaderHandoffInput};
