//! VMX initialization errors.

use alloc::string::String;

/// Category of VMX initialization failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmxErrorKind {
    /// Planning or layout validation failed.
    Planning,
    /// Platform requirements were not satisfied for VMX bring-up.
    Requirements,
    /// Backend rejected the init sequence.
    Backend,
}

/// Structured VMX initialization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmxError {
    /// Error category.
    pub kind: VmxErrorKind,
    /// Human-readable message.
    pub message: String,
}

impl VmxError {
    /// Creates a new VMX error.
    pub fn new(kind: VmxErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl core::fmt::Display for VmxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}
impl core::fmt::Display for VmxErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Planning => write!(f, "vmx planning error"),
            Self::Requirements => write!(f, "vmx requirements error"),
            Self::Backend => write!(f, "vmx backend error"),
        }
    }
}

impl From<hv_platform_model::PlatformError> for VmxError {
    fn from(err: hv_platform_model::PlatformError) -> Self {
        Self::new(VmxErrorKind::Planning, err.message)
    }
}
