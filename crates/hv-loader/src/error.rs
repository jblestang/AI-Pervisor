//! Loader handoff errors.

/// Kind of loader error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderErrorKind {
    /// Boot info construction failed.
    BootInfo,
    /// Observation input assembly failed.
    Observation,
}

/// Structured loader error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderError {
    /// Error category.
    pub kind: LoaderErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl LoaderError {
    /// Creates a new loader error.
    pub fn new(kind: LoaderErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for LoaderError {}

impl std::fmt::Display for LoaderErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BootInfo => write!(f, "loader boot info error"),
            Self::Observation => write!(f, "loader observation error"),
        }
    }
}

impl From<hv_boot_abi::BootError> for LoaderError {
    fn from(err: hv_boot_abi::BootError) -> Self {
        Self::new(LoaderErrorKind::BootInfo, err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_error_display_includes_kind() {
        let err = LoaderError::new(LoaderErrorKind::BootInfo, "bad blob");
        assert!(err.to_string().contains("loader boot info error"));
        assert!(LoaderErrorKind::Observation.to_string().contains("loader observation error"));
    }
}
