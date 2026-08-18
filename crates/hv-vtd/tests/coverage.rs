//! Additional coverage for VT-d planning and errors.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::compile_config_from_str;
use hv_platform_model::plan_static_platform_ir;
use hv_vtd::{plan_vtd_init, VtdError, VtdErrorKind};

#[test]
fn plan_vtd_init_preserves_pci_vm_ids_from_layout() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let plan = plan_vtd_init(&layout, true).expect("plan");
    assert_eq!(plan.device_assignments.len(), 2);
    assert!(plan.device_assignments.iter().any(|entry| entry.vm_id == 0));
    assert!(plan.device_assignments.iter().any(|entry| entry.vm_id == 2));
}

#[test]
fn vtd_error_display_includes_kind_and_message() {
    let err = VtdError::new(VtdErrorKind::Backend, "mock failure");
    assert!(format!("{err}").contains("vtd backend error"));
    assert!(format!("{err}").contains("mock failure"));
    assert!(format!("{}", VtdErrorKind::Planning).contains("planning"));
    assert!(format!("{}", VtdErrorKind::Requirements).contains("requirements"));
}

#[test]
fn vtd_init_required_matches_feature_requirement() {
    use hv_config_model::FeatureRequirement;
    use hv_vtd::vtd_init_required;
    assert!(vtd_init_required(FeatureRequirement::Required));
    assert!(vtd_init_required(FeatureRequirement::Preferred));
    assert!(!vtd_init_required(FeatureRequirement::Optional));
    assert!(!vtd_init_required(FeatureRequirement::Disabled));
}
