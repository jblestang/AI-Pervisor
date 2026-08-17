//! ACPI table discovery from firmware physical memory.

#![cfg_attr(not(test), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

mod constants;
mod error;
mod physical;
mod walk;

pub use constants::{
    ACPI_COLLECTED_MAX_BYTES, ACPI_ROOT_MAX_ENTRIES, ACPI_TABLE_HEADER_LENGTH,
    ACPI_TABLE_MAX_LENGTH, RSDT_ENTRY_SIZE, RSDT_SIGNATURE, XSDT_ENTRY_SIZE, XSDT_SIGNATURE,
};
pub use error::{AcpiWalkError, AcpiWalkErrorKind};
pub use physical::{FirmwareMemoryImage, PhysicalMemory};
pub use walk::collect_acpi_tables;
