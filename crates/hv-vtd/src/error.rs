//! VT-d initialization errors.

use alloc::string::String;

/// Category of VT-d initialization failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VtdErrorKind {
    /// Planning or layout validation failed.
    Planning,
    /// Platform requirements were not satisfied for VT-d bring-up.
    Requirements,
    /// Backend rejected the init sequence.
    Backend,
}

/// Structured VT-d initialization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtdError {
    /// Error category.
    pub kind: VtdErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl VtdError {
    /// Creates a new VT-d error.
    pub fn new(kind: VtdErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl core::fmt::Display for VtdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl core::fmt::Display for VtdErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Planning => write!(f, "vtd planning error"),
            Self::Requirements => write!(f, "vtd requirements error"),
            Self::Backend => write!(f, "vtd backend error"),
        }
    }
}
