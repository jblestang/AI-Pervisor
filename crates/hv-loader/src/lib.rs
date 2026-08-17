//! UEFI loader handoff builder for the static hypervisor boot path.

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

mod build;
mod constants;
mod error;
#[cfg(any(test, feature = "std"))]
mod firmware;
mod handoff;

pub use build::{build_boot_info_blob, BootInfoSection};

pub use constants::{
    DEFAULT_MEMORY_DESCRIPTOR_SIZE, MEMORY_MAP_KIND, RSDP_KIND,
};
pub use error::{LoaderError, LoaderErrorKind};
#[cfg(any(test, feature = "std"))]
pub use firmware::{encode_empty_acpi_firmware, encode_qemu_reference_firmware};
pub use handoff::{build_loader_handoff, LoaderHandoff, LoaderHandoffInput};

pub use hv_acpi_walk::{collect_acpi_tables, FirmwareMemoryImage, PhysicalMemory};
