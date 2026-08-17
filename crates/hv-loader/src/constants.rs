//! Loader-side constants for boot handoff construction.

use hv_boot_abi::descriptor_kind;

/// Default UEFI memory map descriptor size used by OVMF on x86-64.
pub const DEFAULT_MEMORY_DESCRIPTOR_SIZE: usize = 48;

/// Descriptor kinds re-exported for loader callers.
pub const MEMORY_MAP_KIND: u32 = descriptor_kind::MEMORY_MAP;
/// Boot info descriptor kind for the ACPI RSDP section.
pub const RSDP_KIND: u32 = descriptor_kind::RSDP;
