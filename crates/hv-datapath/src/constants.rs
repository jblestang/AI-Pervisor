//! Datapath layout constants.

/// Intel e1000 MMIO window size in bytes (128 KiB BAR).
pub const E1000_MMIO_SIZE_BYTES: u64 = 0x20_000;

/// Base guest physical address for the first e1000 MMIO window.
pub const E1000_MMIO_GUEST_PHYS_BASE: u64 = 0xFEB0_0000;

/// Stride between per-partition e1000 MMIO guest physical bases.
pub const E1000_MMIO_GUEST_PHYS_STRIDE: u64 = 0x1_0000;
