//! VT-d backend trait and mock implementation.

use crate::error::{VtdError, VtdErrorKind};
use crate::plan::VtdInitPlan;

/// Backend that programs VT-d / interrupt remapping for device isolation.
pub trait VtdBackend {
    /// Enables VT-d using the planned device assignments.
    fn enable_vtd(&mut self, plan: &VtdInitPlan) -> Result<(), VtdError>;
}

/// Records VT-d init requests for host-side MODEL tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MockVtdBackend {
    /// Number of successful enable calls.
    pub enable_calls: u32,
    /// Last plan submitted to the backend.
    pub last_plan: Option<VtdInitPlan>,
}

impl VtdBackend for MockVtdBackend {
    fn enable_vtd(&mut self, plan: &VtdInitPlan) -> Result<(), VtdError> {
        self.enable_calls = self.enable_calls.saturating_add(1);
        self.last_plan = Some(plan.clone());
        Ok(())
    }
}

/// Backend that always fails enablement (negative tests).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FailingVtdBackend;

impl VtdBackend for FailingVtdBackend {
    fn enable_vtd(&mut self, _plan: &VtdInitPlan) -> Result<(), VtdError> {
        Err(VtdError::new(
            VtdErrorKind::Backend,
            "mock backend rejected VT-d enablement",
        ))
    }
}
