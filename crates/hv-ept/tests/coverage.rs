//! Additional coverage for EPT error paths.

#![allow(clippy::expect_used, clippy::indexing_slicing)]
use hv_config_model::compile_config_from_str;
use hv_ept::{plan_ept_init, EptErrorKind};
use hv_platform_model::plan_static_platform_ir;
use hv_types::ByteSize;
use hv_vmx::{plan_vmx_init, VMXON_REGION_MIN_BYTES};

#[test]
fn plan_ept_init_rejects_unaligned_guest_mapping() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let mut layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    layout.guest_memory[0].host_phys = hv_types::HostPhysAddr::new(0x1001);
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
    let err = plan_ept_init(&layout, &vmx_plan).expect_err("must fail");
    assert_eq!(err.kind, EptErrorKind::Planning);
}

#[test]
fn plan_ept_init_rejects_zero_sized_mapping() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let mut layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    layout.guest_memory[0].size = ByteSize::new(0);
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
    let err = plan_ept_init(&layout, &vmx_plan).expect_err("must fail");
    assert_eq!(err.kind, EptErrorKind::Planning);
}

#[test]
fn plan_ept_init_rejects_undersized_hypervisor_reserve() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let mut layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    layout.hypervisor_reserve.size = ByteSize::new(VMXON_REGION_MIN_BYTES);
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
    let err = plan_ept_init(&layout, &vmx_plan).expect_err("must fail");
    assert_eq!(err.kind, EptErrorKind::Planning);
}

#[test]
fn plan_ept_init_rejects_unaligned_host_mapping() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let mut layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    layout.guest_memory[0].host_phys = hv_types::HostPhysAddr::new(0x1001);
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
    let err = plan_ept_init(&layout, &vmx_plan).expect_err("must fail");
    assert_eq!(err.kind, EptErrorKind::Planning);
}

#[test]
fn ept_error_display_includes_kind_and_message() {
    use hv_ept::{EptError, EptErrorKind};
    let err = EptError::new(EptErrorKind::Backend, "mock failure");
    assert!(format!("{err}").contains("ept backend error"));
    assert!(format!("{err}").contains("mock failure"));
    assert!(format!("{}", EptErrorKind::Planning).contains("planning"));
    assert!(format!("{}", EptErrorKind::Requirements).contains("requirements"));
}

#[test]
fn ept_init_required_matches_feature_requirement() {
    use hv_config_model::FeatureRequirement;
    use hv_ept::ept_init_required;
    assert!(ept_init_required(FeatureRequirement::Required));
    assert!(ept_init_required(FeatureRequirement::Preferred));
    assert!(!ept_init_required(FeatureRequirement::Optional));
    assert!(!ept_init_required(FeatureRequirement::Disabled));
}
