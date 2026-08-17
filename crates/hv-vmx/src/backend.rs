//! VMX backend trait and mock implementation.

use crate::error::{VmxError, VmxErrorKind};
use crate::plan::VmxInitPlan;

/// Backend that performs VMX hardware initialization steps.
pub trait VmxBackend {
    /// Enables VMX root operation using the planned VMXON region.
    fn enable_vmx(&mut self, plan: &VmxInitPlan) -> Result<(), VmxError>;
}

/// Records VMX init requests for host-side MODEL tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MockVmxBackend {
    /// Number of successful enable calls.
    pub enable_calls: u32,
    /// Last plan submitted to the backend.
    pub last_plan: Option<VmxInitPlan>,
}

impl VmxBackend for MockVmxBackend {
    fn enable_vmx(&mut self, plan: &VmxInitPlan) -> Result<(), VmxError> {
        self.enable_calls = self.enable_calls.saturating_add(1);
        self.last_plan = Some(*plan);
        Ok(())
    }
}

/// Backend that always fails enablement (negative tests).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FailingVmxBackend;

impl VmxBackend for FailingVmxBackend {
    fn enable_vmx(&mut self, _plan: &VmxInitPlan) -> Result<(), VmxError> {
        Err(VmxError::new(
            VmxErrorKind::Backend,
            "mock backend rejected VMX enablement",
        ))
    }
}
