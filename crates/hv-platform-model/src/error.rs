//! Platform validation and planning errors.

/// Kind of platform error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformErrorKind {
    /// Observed platform does not satisfy requirements.
    Validation,
    /// Address planning overflow or alignment failure.
    Planning,
    /// JSON or fixture parse failure.
    Parse,
    /// Runtime observation from firmware inputs failed.
    Observation,
}

/// Structured platform error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformError {
    /// Error category.
    pub kind: PlatformErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl PlatformError {
    /// Creates a new platform error.
    pub fn new(kind: PlatformErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for PlatformError {}

impl std::fmt::Display for PlatformErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation => write!(f, "platform validation error"),
            Self::Planning => write!(f, "platform planning error"),
            Self::Parse => write!(f, "platform parse error"),
            Self::Observation => write!(f, "platform observation error"),
        }
    }
}

/// Non-fatal platform validation warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformWarning {
    /// Human-readable warning message.
    pub message: String,
}

impl PlatformWarning {
    /// Creates a new platform warning.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PlatformWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "warning: {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_error_display_includes_kind() {
        let err = PlatformError::new(PlatformErrorKind::Validation, "vmx missing");
        assert!(err.to_string().contains("platform validation error"));
        assert!(err.to_string().contains("vmx missing"));
    }

    #[test]
    fn platform_warning_display() {
        let warning = PlatformWarning::new("x2apic preferred but absent");
        assert!(warning.to_string().contains("x2apic preferred but absent"));
    }
}
