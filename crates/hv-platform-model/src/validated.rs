//! Validated platform produced after fail-closed comparison.

use crate::observed::ObservedPlatform;

/// Platform snapshot validated against compile-time requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPlatform {
    /// Observed platform that satisfied all required checks.
    pub observed: ObservedPlatform,
}

impl ValidatedPlatform {
    /// Creates a validated platform wrapper.
    pub const fn new(observed: ObservedPlatform) -> Self {
        Self { observed }
    }
}
