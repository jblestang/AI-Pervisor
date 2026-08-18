//! EPT backend trait and mock implementation.

use crate::error::{EptError, EptErrorKind};
use crate::plan::EptInitPlan;

/// Backend that installs EPT structures for guest address spaces.
pub trait EptBackend {
    /// Installs the planned EPT hierarchy.
    fn install_ept(&mut self, plan: &EptInitPlan) -> Result<(), EptError>;
}

/// Records EPT init requests for host-side MODEL tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MockEptBackend {
    /// Number of successful install calls.
    pub install_calls: u32,
    /// Last plan submitted to the backend.
    pub last_plan: Option<EptInitPlan>,
}

impl EptBackend for MockEptBackend {
    fn install_ept(&mut self, plan: &EptInitPlan) -> Result<(), EptError> {
        self.install_calls = self.install_calls.saturating_add(1);
        self.last_plan = Some(plan.clone());
        Ok(())
    }
}

/// Backend that always fails installation (negative tests).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FailingEptBackend;

impl EptBackend for FailingEptBackend {
    fn install_ept(&mut self, _plan: &EptInitPlan) -> Result<(), EptError> {
        Err(EptError::new(
            EptErrorKind::Backend,
            "mock backend rejected EPT installation",
        ))
    }
}
