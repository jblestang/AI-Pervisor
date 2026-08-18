//! Datapath planning errors.

use alloc::string::String;

/// Category of datapath planning failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatapathErrorKind {
    /// Partition or layout input was invalid.
    InvalidInput,
    /// IPC queue integrity or bounds violation.
    IpcViolation,
}

/// Structured datapath planning error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathError {
    /// Error category.
    pub kind: DatapathErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl DatapathError {
    /// Creates a new datapath planning error.
    pub fn new(kind: DatapathErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
