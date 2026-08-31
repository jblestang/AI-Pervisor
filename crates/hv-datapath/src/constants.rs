//! Datapath layout constants.

/// Reference IPC shared mapping size for `configs/qemu.yaml` (slot metadata + payload per slot).
pub const REFERENCE_IPC_SHARED_BYTES: u64 = 0x840_000;

/// Guest physical base for `chan_a` in the reference planner output.
pub const REFERENCE_IPC_CHAN_A_GUEST_PHYS: u64 = 0x0000_0002_0000_0000;

/// Guest physical base for `chan_b` in the reference planner output.
pub const REFERENCE_IPC_CHAN_B_GUEST_PHYS: u64 = 0x0000_0002_0084_0000;

/// Intel e1000 MMIO window size in bytes (128 KiB BAR).
pub const E1000_MMIO_SIZE_BYTES: u64 = 0x20_000;

/// Base guest physical address for the first e1000 MMIO window.
pub const E1000_MMIO_GUEST_PHYS_BASE: u64 = 0xFEB0_0000;

/// Stride between per-partition e1000 MMIO guest physical bases.
pub const E1000_MMIO_GUEST_PHYS_STRIDE: u64 = 0x1_0000;
