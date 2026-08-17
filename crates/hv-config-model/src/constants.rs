//! Configuration model constants derived from platform contracts.

use hv_types::{ByteSize, BYTES_PER_MIB};

/// Supported target architecture string in YAML.
pub const SUPPORTED_ARCH: &str = "x86_64";

/// Default hypervisor memory reserve in mebibytes.
pub const HYPERVISOR_RESERVE_MIB: u64 = 64;

/// Per-slot IPC queue metadata size in bytes.
pub const IPC_SLOT_METADATA_BYTES: u64 = 64;

/// Returns the default hypervisor reserve as a byte size.
pub const fn hypervisor_reserve_bytes() -> ByteSize {
    ByteSize::new(HYPERVISOR_RESERVE_MIB * BYTES_PER_MIB)
}
