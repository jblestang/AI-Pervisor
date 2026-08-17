//! Shared numeric constants for cryptography and memory units.

/// Number of bytes in a SHA-256 digest.
pub const SHA256_DIGEST_BYTES: usize = 32;

/// Length of a lowercase hex-encoded SHA-256 digest.
pub const SHA256_HEX_LEN: usize = SHA256_DIGEST_BYTES * 2;

/// Bytes in one mebibyte (MiB).
pub const BYTES_PER_MIB: u64 = 1024 * 1024;

/// Bytes in one gibibyte (GiB).
pub const BYTES_PER_GIB: u64 = 1024 * 1024 * 1024;
