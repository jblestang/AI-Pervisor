//! Hypervisor boot-path errors.

use alloc::string::String;
use core::fmt;

/// Kind of hypervisor boot error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootCheckErrorKind {
    /// Boot ABI validation failed.
    BootAbi,
    /// Platform observation failed.
    Observation,
    /// Platform validation failed.
    Platform,
}

/// Structured hypervisor boot error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootCheckError {
    /// Error category.
    pub kind: BootCheckErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl BootCheckError {
    /// Creates a new boot check error.
    pub fn new(kind: BootCheckErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for BootCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl fmt::Display for BootCheckErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BootAbi => write!(f, "boot abi error"),
            Self::Observation => write!(f, "boot observation error"),
            Self::Platform => write!(f, "boot platform error"),
        }
    }
}

impl From<hv_boot_abi::BootError> for BootCheckError {
    fn from(err: hv_boot_abi::BootError) -> Self {
        Self::new(BootCheckErrorKind::BootAbi, err.message)
    }
}

impl From<hv_platform_model::PlatformError> for BootCheckError {
    fn from(err: hv_platform_model::PlatformError) -> Self {
        let kind = match err.kind {
            hv_platform_model::PlatformErrorKind::Observation => BootCheckErrorKind::Observation,
            _ => BootCheckErrorKind::Platform,
        };
        Self::new(kind, err.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_check_error_display_includes_kind() {
        let err = BootCheckError::new(BootCheckErrorKind::BootAbi, "bad digest");
        assert!(err.to_string().contains("boot abi error"));
    }
}
