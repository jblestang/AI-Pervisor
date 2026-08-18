//! VMX initialization orchestration.

use hv_config_model::FeatureRequirement;
use hv_platform_model::ValidatedPlatform;

use crate::backend::VmxBackend;
use crate::error::{VmxError, VmxErrorKind};
use crate::plan::VmxInitPlan;

/// Initializes VMX using the supplied backend after platform validation succeeded.
pub fn init_vmx<B: VmxBackend>(
    backend: &mut B,
    plan: &VmxInitPlan,
    validated: &ValidatedPlatform,
) -> Result<(), VmxError> {
    if !validated.observed.vmx {
        return Err(VmxError::new(
            VmxErrorKind::Requirements,
            "validated platform does not expose VMX",
        ));
    }
    if !validated.observed.ept {
        return Err(VmxError::new(
            VmxErrorKind::Requirements,
            "validated platform does not expose EPT",
        ));
    }
    backend.enable_vmx(plan)
}

/// Returns whether VMX init should proceed for the given feature requirement.
pub fn vmx_init_required(requirement: FeatureRequirement) -> bool {
    matches!(
        requirement,
        FeatureRequirement::Required | FeatureRequirement::Preferred
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::backend::{FailingVmxBackend, MockVmxBackend};
    use crate::plan::plan_vmx_init;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::{plan_static_platform_ir, validate_platform};

    #[test]
    fn init_vmx_invokes_mock_backend_for_reference_platform() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx plan");
        let observed = include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let observed = hv_platform_model::parse_observed_platform_json(observed).expect("parse");
        let (validated, _) =
            validate_platform(&compiled.requirements, &observed).expect("validate");
        let mut backend = MockVmxBackend::default();
        init_vmx(&mut backend, &plan, &validated).expect("init");
        assert_eq!(backend.enable_calls, 1);
        assert_eq!(backend.last_plan, Some(plan));
    }

    #[test]
    fn init_vmx_rejects_missing_vmx_capability() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx plan");
        let observed_json = include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let mut observed = hv_platform_model::parse_observed_platform_json(observed_json).expect("parse");
        observed.vmx = false;
        let validated = ValidatedPlatform::new(observed);
        let mut backend = MockVmxBackend::default();
        let err = init_vmx(&mut backend, &plan, &validated).expect_err("must fail");
        assert_eq!(err.kind, VmxErrorKind::Requirements);
    }

    #[test]
    fn init_vmx_rejects_missing_ept_capability() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx plan");
        let observed_json = include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let mut observed = hv_platform_model::parse_observed_platform_json(observed_json).expect("parse");
        observed.ept = false;
        let validated = ValidatedPlatform::new(observed);
        let mut backend = MockVmxBackend::default();
        let err = init_vmx(&mut backend, &plan, &validated).expect_err("must fail");
        assert_eq!(err.kind, VmxErrorKind::Requirements);
    }

    #[test]
    fn vmx_init_required_matches_feature_requirement() {
        use hv_config_model::FeatureRequirement;
        assert!(vmx_init_required(FeatureRequirement::Required));
        assert!(vmx_init_required(FeatureRequirement::Preferred));
        assert!(!vmx_init_required(FeatureRequirement::Optional));
        assert!(!vmx_init_required(FeatureRequirement::Disabled));
    }

    #[test]
    fn init_vmx_propagates_backend_failure() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx plan");
        let observed = include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let observed = hv_platform_model::parse_observed_platform_json(observed).expect("parse");
        let (validated, _) =
            validate_platform(&compiled.requirements, &observed).expect("validate");
        let mut backend = FailingVmxBackend;
        assert!(init_vmx(&mut backend, &plan, &validated).is_err());
    }
}
