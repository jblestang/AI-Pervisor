//! Datapath live forward outcomes (validate-only default until live VMX guest execution).

use crate::error::DatapathError;
use crate::forward::forward_synthetic_frame;
use crate::topology::DatapathForwardPlan;

/// How a datapath live forward completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatapathLiveDisposition {
    /// Inputs validated without executing guest datapath instructions.
    ValidatedOnly,
    /// Live datapath seam executed.
    Executed,
    /// Live environment unavailable.
    Unavailable,
}

/// Outcome of a datapath live forward invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathLiveOutcome {
    /// How the forward completed.
    pub disposition: DatapathLiveDisposition,
    /// Whether a synthetic frame was forwarded in→mid→out.
    pub synthetic_frame_forwarded: bool,
    /// Whether IN e1000 TX doorbell was observed.
    pub e1000_tx_observed: bool,
}

/// Forwards one synthetic frame and returns the live forward outcome.
pub fn run_datapath_live_forward(
    plan: &mut DatapathForwardPlan,
) -> Result<DatapathLiveOutcome, DatapathError> {
    forward_synthetic_frame(plan)?;
    Ok(DatapathLiveOutcome {
        disposition: DatapathLiveDisposition::ValidatedOnly,
        synthetic_frame_forwarded: true,
        e1000_tx_observed: plan.in_e1000.tx_doorbell,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use crate::forward::plan_datapath_forward;

    #[test]
    fn run_datapath_live_forward_traverses_in_mid_out() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let mut plan = plan_datapath_forward(&layout).expect("plan");
        let outcome = run_datapath_live_forward(&mut plan).expect("live forward");
        assert!(outcome.synthetic_frame_forwarded);
        assert!(outcome.e1000_tx_observed);
    }
}
