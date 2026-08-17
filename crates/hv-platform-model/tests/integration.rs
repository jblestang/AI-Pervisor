//! Integration tests for platform validation and planning.

use hv_config_model::compile_config_from_str;
use hv_platform_model::{
    parse_observed_platform_json, plan_static_platform_ir, validate_platform, PlatformErrorKind,
};

#[test]
fn end_to_end_validate_reference_observed_platform() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let json = include_str!("fixtures/observed/qemu_reference.json");
    let observed = parse_observed_platform_json(json).expect("parse");
    let (validated, warnings) =
        validate_platform(&compiled.requirements, &observed).expect("validate");
    assert_eq!(validated.observed.arch, "x86_64");
    assert!(warnings.is_empty());
    let planned = plan_static_platform_ir(&compiled.intent).expect("plan");
    assert_eq!(planned.guest_memory.len(), compiled.intent.partitions.len());
}

#[test]
fn end_to_end_rejects_incompatible_observed_platform() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let json = include_str!("fixtures/observed/missing_vmx.json");
    let observed = parse_observed_platform_json(json).expect("parse");
    let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
    assert_eq!(err.kind, PlatformErrorKind::Validation);
}
