//! Boot ABI parse and validation errors.

/// Kind of boot ABI error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootErrorKind {
    /// Boot info blob is truncated or malformed.
    Parse,
    /// Header magic, version, or size check failed.
    Incompatible,
    /// Descriptor table or section bounds are invalid.
    Bounds,
    /// Configuration digest mismatch.
    DigestMismatch,
}

/// Structured boot ABI error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootError {
    /// Error category.
    pub kind: BootErrorKind,
    /// Human-readable message.
    pub message: &'static str,
}

impl BootError {
    /// Creates a new boot error.
    pub const fn new(kind: BootErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }
}

impl core::fmt::Display for BootError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl core::fmt::Display for BootErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse => write!(f, "boot parse error"),
            Self::Incompatible => write!(f, "boot incompatible"),
            Self::Bounds => write!(f, "boot bounds error"),
            Self::DigestMismatch => write!(f, "boot digest mismatch"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_error_display_includes_kind() {
        let err = BootError::new(BootErrorKind::Parse, "truncated header");
        assert!(err.to_string().contains("boot parse error"));
        assert!(err.to_string().contains("truncated header"));
        assert_eq!(BootErrorKind::Incompatible.to_string(), "boot incompatible");
        assert_eq!(BootErrorKind::Bounds.to_string(), "boot bounds error");
        assert_eq!(
            BootErrorKind::DigestMismatch.to_string(),
            "boot digest mismatch"
        );
    }
}
