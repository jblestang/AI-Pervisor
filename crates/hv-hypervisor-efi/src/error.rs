//! Hypervisor UEFI entry errors.

use core::fmt;

/// Kind of hypervisor UEFI verification error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypervisorEfiErrorKind {
    /// Transfer blob parsing failed.
    Transfer,
    /// Boot info parsing or digest verification failed.
    BootInfo,
    /// Observation payload decoding failed.
    Observation,
    /// Embedded requirements snapshot mismatch.
    Requirements,
}

/// Structured hypervisor UEFI verification error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypervisorEfiError {
    /// Error category.
    pub kind: HypervisorEfiErrorKind,
    /// Human-readable message.
    pub message: &'static str,
}

impl HypervisorEfiError {
    /// Creates a new hypervisor EFI error.
    pub const fn new(kind: HypervisorEfiErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }
}

impl fmt::Display for HypervisorEfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl From<hv_boot_abi::BootError> for HypervisorEfiError {
    fn from(err: hv_boot_abi::BootError) -> Self {
        Self::new(HypervisorEfiErrorKind::Transfer, err.message)
    }
}
