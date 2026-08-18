//! EPT-related constants.

/// Default EPT page size (4 KiB).
pub const EPT_PAGE_SIZE_BYTES: u64 = 4096;

/// Low bits of a page-aligned guest physical address.
pub const EPT_PAGE_OFFSET_MASK: u64 = EPT_PAGE_SIZE_BYTES - 1;

/// Bytes reserved for the EPT root table inside the hypervisor private region.
pub const EPT_ROOT_TABLE_BYTES: u64 = 4096;

/// EPT pointer memory type WB (write-back), encoded in bits 5:3.
pub const EPT_POINTER_MEMORY_TYPE_WB: u64 = 6;

/// Bit shift for the EPT pointer memory type field.
pub const EPT_POINTER_MEMORY_TYPE_SHIFT: u32 = 3;

/// EPT pointer page-walk length minus one (4-level walk), encoded in bits 8:6.
pub const EPT_POINTER_PAGE_WALK_LENGTH: u64 = 2;

/// Bit shift for the EPT pointer page-walk length field.
pub const EPT_POINTER_PAGE_WALK_LENGTH_SHIFT: u32 = 6;
