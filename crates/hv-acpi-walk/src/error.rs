//! ACPI table walk errors.

/// Kind of ACPI walk error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiWalkErrorKind {
    /// Firmware memory read failed.
    Memory,
    /// ACPI structure parse or checksum validation failed.
    Parse,
    /// Pointer or length bounds are invalid.
    Bounds,
}

/// Structured ACPI walk error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpiWalkError {
    /// Error category.
    pub kind: AcpiWalkErrorKind,
    /// Human-readable message.
    pub message: &'static str,
}

impl AcpiWalkError {
    /// Creates a new ACPI walk error.
    pub const fn new(kind: AcpiWalkErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }
}

impl core::fmt::Display for AcpiWalkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl core::fmt::Display for AcpiWalkErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Memory => write!(f, "acpi memory error"),
            Self::Parse => write!(f, "acpi parse error"),
            Self::Bounds => write!(f, "acpi bounds error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acpi_walk_error_display_includes_kind() {
        let err = AcpiWalkError::new(AcpiWalkErrorKind::Memory, "read failed");
        assert!(format!("{err}").contains("acpi memory error"));
        assert_eq!(
            format!("{}", AcpiWalkErrorKind::Bounds),
            "acpi bounds error"
        );
    }
}
