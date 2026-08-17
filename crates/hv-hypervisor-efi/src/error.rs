//! Hypervisor UEFI entry errors.

use alloc::string::String;
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
    /// Platform validation or VMX init failed.
    Platform,
}

/// Structured hypervisor UEFI verification error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypervisorEfiError {
    /// Error category.
    pub kind: HypervisorEfiErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl HypervisorEfiError {
    /// Creates a new hypervisor EFI error.
    pub fn new(kind: HypervisorEfiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
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

impl From<hv_hypervisor_boot::BootCheckError> for HypervisorEfiError {
    fn from(err: hv_hypervisor_boot::BootCheckError) -> Self {
        let kind = match err.kind {
            hv_hypervisor_boot::BootCheckErrorKind::BootAbi => HypervisorEfiErrorKind::BootInfo,
            hv_hypervisor_boot::BootCheckErrorKind::Observation => {
                HypervisorEfiErrorKind::Observation
            }
            hv_hypervisor_boot::BootCheckErrorKind::Platform => HypervisorEfiErrorKind::Platform,
        };
        Self::new(kind, err.message)
    }
}
