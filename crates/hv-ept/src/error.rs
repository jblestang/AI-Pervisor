//! EPT initialization errors.

use alloc::string::String;

/// Category of EPT initialization failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EptErrorKind {
    /// Planning or layout validation failed.
    Planning,
    /// Platform requirements were not satisfied for EPT bring-up.
    Requirements,
    /// Backend rejected the init sequence.
    Backend,
}

/// Structured EPT initialization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EptError {
    /// Error category.
    pub kind: EptErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl EptError {
    /// Creates a new EPT error.
    pub fn new(kind: EptErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl core::fmt::Display for EptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl core::fmt::Display for EptErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Planning => write!(f, "ept planning error"),
            Self::Requirements => write!(f, "ept requirements error"),
            Self::Backend => write!(f, "ept backend error"),
        }
    }
}
