//! CPU seam errors.

extern crate alloc;

use alloc::string::String;

/// Category of CPU seam failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuSeamErrorKind {
    /// CPU capability was not present for the requested seam.
    Unavailable,
    /// Seam input was invalid.
    InvalidInput,
    /// Live instruction execution was attempted and failed.
    ExecutionFailed,
}

/// Structured CPU seam error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuSeamError {
    /// Error category.
    pub kind: CpuSeamErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl CpuSeamError {
    /// Creates a new CPU seam error.
    pub fn new(kind: CpuSeamErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl core::fmt::Display for CpuSeamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl core::fmt::Display for CpuSeamErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "cpu seam unavailable"),
            Self::InvalidInput => write!(f, "cpu seam invalid input"),
            Self::ExecutionFailed => write!(f, "cpu seam execution failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_seam_error_display_includes_kind_and_message() {
        let err = CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "bad input");
        assert!(format!("{err}").contains("invalid input"));
        assert!(format!("{err}").contains("bad input"));
        assert!(format!("{}", CpuSeamErrorKind::Unavailable).contains("unavailable"));
        assert!(format!("{}", CpuSeamErrorKind::ExecutionFailed).contains("execution failed"));
    }
}

