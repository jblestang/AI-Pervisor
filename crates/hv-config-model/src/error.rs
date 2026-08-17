//! Structured configuration errors and warnings.

use std::path::PathBuf;

/// Kind of configuration error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigErrorKind {
    /// I/O or YAML parse failure.
    Parse,
    /// Syntax/schema validation failure.
    Syntax,
    /// Semantic validation failure.
    Semantic,
    /// Overflow or arithmetic failure during normalization.
    Arithmetic,
    /// Internal serialization failure.
    Internal,
}

/// Structured configuration error with location context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// Error category.
    pub kind: ConfigErrorKind,
    /// Optional dotted path within the configuration tree.
    pub path: Option<String>,
    /// Human-readable message.
    pub message: String,
    /// Optional source file path.
    pub file: Option<PathBuf>,
}

impl ConfigError {
    /// Creates a new configuration error.
    pub fn new(kind: ConfigErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            path: None,
            message: message.into(),
            file: None,
        }
    }

    /// Attaches a configuration path to the error.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Attaches a source file path to the error.
    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = Some(file);
        self
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{} (at {}): {}", self.kind, path, self.message)
        } else {
            write!(f, "{}: {}", self.kind, self.message)
        }
    }
}

impl std::error::Error for ConfigError {}

impl std::fmt::Display for ConfigErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse => write!(f, "parse error"),
            Self::Syntax => write!(f, "syntax error"),
            Self::Semantic => write!(f, "semantic error"),
            Self::Arithmetic => write!(f, "arithmetic error"),
            Self::Internal => write!(f, "internal error"),
        }
    }
}

/// Kind of non-fatal configuration warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningKind {
    /// Security-relevant configuration concern.
    Security,
    /// Safety-relevant configuration concern.
    Safety,
    /// Timing or determinism concern.
    Timing,
}

/// Non-fatal warning emitted during compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWarning {
    /// Warning category.
    pub kind: WarningKind,
    /// Optional dotted path within the configuration tree.
    pub path: Option<String>,
    /// Human-readable message.
    pub message: String,
}

impl ConfigWarning {
    /// Creates a new configuration warning.
    pub fn new(kind: WarningKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            path: None,
            message: message.into(),
        }
    }

    /// Attaches a configuration path to the warning.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

impl std::fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{:?} warning at {}: {}", self.kind, path, self.message)
        } else {
            write!(f, "{:?} warning: {}", self.kind, self.message)
        }
    }
}
