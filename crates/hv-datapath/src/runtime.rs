//! Datapath live runtime backend and seam outcomes.

use crate::error::DatapathError;
use crate::forward::forward_synthetic_frame;
use crate::topology::DatapathForwardPlan;

/// How a datapath live seam completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatapathLiveDisposition {
    /// Inputs validated without executing guest datapath instructions.
    ValidatedOnly,
    /// Live datapath seam executed.
    Executed,
    /// Live environment unavailable.
    Unavailable,
}

/// Outcome of a datapath live runtime invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathLiveOutcome {
    /// How the seam completed.
    pub disposition: DatapathLiveDisposition,
    /// Whether a synthetic frame was forwarded in mock runtime.
    pub synthetic_frame_forwarded: bool,
    /// Whether IN e1000 TX doorbell was observed.
    pub e1000_tx_observed: bool,
}

/// Mock datapath backend for host-side Gate D live orchestration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockDatapathBackend {
    /// Forward plan with queue backing stores.
    pub forward_plan: DatapathForwardPlan,
    /// Last live outcome.
    pub last_outcome: Option<DatapathLiveOutcome>,
}

impl MockDatapathBackend {
    /// Creates a mock backend from a forward plan.
    pub fn new(forward_plan: DatapathForwardPlan) -> Self {
        Self {
            forward_plan,
            last_outcome: None,
        }
    }

    /// Runs mock datapath live forwarding.
    pub fn run_live(&mut self) -> Result<DatapathLiveOutcome, DatapathError> {
        forward_synthetic_frame(&mut self.forward_plan)?;
        let outcome = DatapathLiveOutcome {
            disposition: DatapathLiveDisposition::ValidatedOnly,
            synthetic_frame_forwarded: true,
            e1000_tx_observed: self.forward_plan.in_e1000.tx_doorbell,
        };
        self.last_outcome = Some(outcome.clone());
        Ok(outcome)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use crate::forward::plan_datapath_forward;

    #[test]
    fn mock_backend_forwards_synthetic_frame() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_datapath_forward(&layout).expect("plan");
        let mut backend = MockDatapathBackend::new(plan);
        let outcome = backend.run_live().expect("live");
        assert!(outcome.synthetic_frame_forwarded);
        assert!(outcome.e1000_tx_observed);
    }
}
