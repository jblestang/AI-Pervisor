//! VT-d initialization orchestration.

use hv_config_model::FeatureRequirement;
use hv_platform_model::ValidatedPlatform;

use crate::backend::VtdBackend;
use crate::error::{VtdError, VtdErrorKind};
use crate::plan::VtdInitPlan;

/// Initializes VT-d using the supplied backend after platform validation succeeded.
pub fn init_vtd<B: VtdBackend>(
    backend: &mut B,
    plan: &VtdInitPlan,
    validated: &ValidatedPlatform,
) -> Result<(), VtdError> {
    if !validated.observed.vtd {
        return Err(VtdError::new(
            VtdErrorKind::Requirements,
            "validated platform does not expose VT-d",
        ));
    }
    if plan.interrupt_remapping && !validated.observed.interrupt_remapping {
        return Err(VtdError::new(
            VtdErrorKind::Requirements,
            "validated platform does not expose interrupt remapping",
        ));
    }
    backend.enable_vtd(plan)
}

/// Returns whether VT-d init should proceed for the given feature requirement.
pub fn vtd_init_required(requirement: FeatureRequirement) -> bool {
    matches!(
        requirement,
        FeatureRequirement::Required | FeatureRequirement::Preferred
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::backend::{FailingVtdBackend, MockVtdBackend};
    use crate::plan::plan_vtd_init;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::{plan_static_platform_ir, validate_platform};

    #[test]
    fn init_vtd_invokes_mock_backend_for_reference_platform() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd plan");
        let observed =
            include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let observed = hv_platform_model::parse_observed_platform_json(observed).expect("parse");
        let (validated, _) =
            validate_platform(&compiled.requirements, &observed).expect("validate");
        let mut backend = MockVtdBackend::default();
        init_vtd(&mut backend, &plan, &validated).expect("init");
        assert_eq!(backend.enable_calls, 1);
    }

    #[test]
    fn init_vtd_rejects_missing_vtd_capability() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd plan");
        let observed_json =
            include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let mut observed =
            hv_platform_model::parse_observed_platform_json(observed_json).expect("parse");
        observed.vtd = false;
        let validated = ValidatedPlatform::new(observed);
        let mut backend = MockVtdBackend::default();
        let err = init_vtd(&mut backend, &plan, &validated).expect_err("must fail");
        assert_eq!(err.kind, VtdErrorKind::Requirements);
    }

    #[test]
    fn init_vtd_rejects_missing_interrupt_remapping() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd plan");
        let observed_json =
            include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let mut observed =
            hv_platform_model::parse_observed_platform_json(observed_json).expect("parse");
        observed.interrupt_remapping = false;
        let validated = ValidatedPlatform::new(observed);
        let mut backend = MockVtdBackend::default();
        let err = init_vtd(&mut backend, &plan, &validated).expect_err("must fail");
        assert_eq!(err.kind, VtdErrorKind::Requirements);
    }

    #[test]
    fn init_vtd_propagates_backend_failure() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd plan");
        let observed =
            include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let observed = hv_platform_model::parse_observed_platform_json(observed).expect("parse");
        let (validated, _) =
            validate_platform(&compiled.requirements, &observed).expect("validate");
        let mut backend = FailingVtdBackend;
        assert!(init_vtd(&mut backend, &plan, &validated).is_err());
    }
}
