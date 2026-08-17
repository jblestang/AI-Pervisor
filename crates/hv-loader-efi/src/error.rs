//! Hypervisor loader errors at the UEFI entry boundary.

use core::fmt;
use alloc::string::{String, ToString};

/// Kind of UEFI loader error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderEfiErrorKind {
    /// Loader handoff construction failed.
    Handoff,
}

/// Structured UEFI loader error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderEfiError {
    /// Error category.
    pub kind: LoaderEfiErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl LoaderEfiError {
    /// Creates a new loader EFI error.
    pub fn new(kind: LoaderEfiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for LoaderEfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LoaderEfiError {}

impl fmt::Display for LoaderEfiErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Handoff => write!(f, "uefi loader handoff error"),
        }
    }
}

impl From<hv_loader::LoaderError> for LoaderEfiError {
    fn from(err: hv_loader::LoaderError) -> Self {
        Self::new(LoaderEfiErrorKind::Handoff, err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_efi_error_display_includes_kind() {
        let err = LoaderEfiError::new(LoaderEfiErrorKind::Handoff, "bad handoff");
        assert!(err.to_string().contains("uefi loader handoff error"));
    }
}
