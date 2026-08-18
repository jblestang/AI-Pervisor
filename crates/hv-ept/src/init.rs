//! EPT initialization orchestration.

use hv_config_model::FeatureRequirement;
use hv_platform_model::ValidatedPlatform;

use crate::backend::EptBackend;
use crate::error::{EptError, EptErrorKind};
use crate::plan::EptInitPlan;

/// Initializes EPT using the supplied backend after platform validation succeeded.
pub fn init_ept<B: EptBackend>(
    backend: &mut B,
    plan: &EptInitPlan,
    validated: &ValidatedPlatform,
) -> Result<(), EptError> {
    if !validated.observed.ept {
        return Err(EptError::new(
            EptErrorKind::Requirements,
            "validated platform does not expose EPT",
        ));
    }
    backend.install_ept(plan)
}

/// Returns whether EPT init should proceed for the given feature requirement.
pub fn ept_init_required(requirement: FeatureRequirement) -> bool {
    matches!(
        requirement,
        FeatureRequirement::Required | FeatureRequirement::Preferred
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::backend::{FailingEptBackend, MockEptBackend};
    use crate::plan::plan_ept_init;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::{plan_static_platform_ir, validate_platform};
    use hv_vmx::plan_vmx_init;

    #[test]
    fn init_ept_invokes_mock_backend_for_reference_platform() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept plan");
        let observed = include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let observed = hv_platform_model::parse_observed_platform_json(observed).expect("parse");
        let (validated, _) =
            validate_platform(&compiled.requirements, &observed).expect("validate");
        let mut backend = MockEptBackend::default();
        init_ept(&mut backend, &plan, &validated).expect("init");
        assert_eq!(backend.install_calls, 1);
    }

    #[test]
    fn init_ept_rejects_missing_ept_capability() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept plan");
        let observed_json = include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let mut observed = hv_platform_model::parse_observed_platform_json(observed_json).expect("parse");
        observed.ept = false;
        let validated = ValidatedPlatform::new(observed);
        let mut backend = MockEptBackend::default();
        let err = init_ept(&mut backend, &plan, &validated).expect_err("must fail");
        assert_eq!(err.kind, EptErrorKind::Requirements);
    }

    #[test]
    fn init_ept_propagates_backend_failure() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept plan");
        let observed = include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let observed = hv_platform_model::parse_observed_platform_json(observed).expect("parse");
        let (validated, _) =
            validate_platform(&compiled.requirements, &observed).expect("validate");
        let mut backend = FailingEptBackend;
        assert!(init_ept(&mut backend, &plan, &validated).is_err());
    }
}
