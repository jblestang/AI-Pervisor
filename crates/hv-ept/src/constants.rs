//! EPT-related constants.

/// Default EPT page size (4 KiB).
pub const EPT_PAGE_SIZE_BYTES: u64 = 4096;

/// Bytes reserved for the EPT root table inside the hypervisor private region.
pub const EPT_ROOT_TABLE_BYTES: u64 = 4096;
