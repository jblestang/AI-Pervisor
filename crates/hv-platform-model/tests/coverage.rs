//! Coverage-oriented tests for platform validation and planning edge cases.

use hv_config_model::{compile_config_from_str, FeatureRequirement, SmtPolicy};
use hv_platform_model::{
    parse_observed_platform_json, plan_static_platform_ir, validate_platform, PlatformErrorKind,
};

fn reference_compiled() -> hv_config_model::CompiledConfig {
    let yaml = include_str!("../../../configs/qemu.yaml");
    compile_config_from_str(yaml).expect("compile reference config")
}

fn reference_observed_json() -> String {
    include_str!("fixtures/observed/qemu_reference.json").to_string()
}

#[test]
fn parse_observed_platform_rejects_invalid_json() {
    let err = parse_observed_platform_json("{").expect_err("must fail");
    assert_eq!(err.kind, PlatformErrorKind::Parse);
}

#[test]
fn observed_arch_requirement_rejects_unknown_arch() {
    let mut json = reference_observed_json();
    json = json.replace("\"x86_64\"", "\"aarch64\"");
    let observed = parse_observed_platform_json(&json).expect("parse");
    let err = observed.arch_requirement().expect_err("must fail");
    assert_eq!(err.kind, PlatformErrorKind::Validation);
}

#[test]
fn validate_rejects_missing_page_size() {
    let compiled = reference_compiled();
    let mut json = reference_observed_json();
    json = json.replace("2097152", "4194304");
    let observed = parse_observed_platform_json(&json).expect("parse");
    let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
    assert!(err.message.contains("page size"));
}

#[test]
fn validate_rejects_missing_pci_device() {
    let compiled = reference_compiled();
    let mut observed = parse_observed_platform_json(&reference_observed_json()).expect("parse");
    observed.pci_devices.pop();
    let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
    assert!(err.message.contains("PCI device"));
}

#[test]
fn validate_rejects_insufficient_physical_cores() {
    let compiled = reference_compiled();
    let mut json = reference_observed_json();
    json = json.replace("\"physical_cores\": 4", "\"physical_cores\": 1");
    let observed = parse_observed_platform_json(&json).expect("parse");
    let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
    assert!(err.message.contains("physical cores"));
}

#[test]
fn validate_rejects_disabled_feature_when_present() {
    let yaml = include_str!("../../hv-config-model/tests/fixtures/valid/all_feature_levels.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let observed = parse_observed_platform_json(&reference_observed_json()).expect("parse");
    let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
    assert!(err.message.contains("disabled"));
}

#[test]
fn validate_emits_preferred_feature_warnings() {
    let compiled = reference_compiled();
    let mut json = reference_observed_json();
    json = json.replace("\"x2apic\": true", "\"x2apic\": false");
    json = json.replace("\"vpid\": true", "\"vpid\": false");
    let observed = parse_observed_platform_json(&json).expect("parse");
    let (_validated, warnings) = validate_platform(&compiled.requirements, &observed).expect("validate");
    assert_eq!(warnings.len(), 2);
}

#[test]
fn validate_rejects_smt_when_policy_disabled() {
    let yaml = include_str!("../../hv-config-model/tests/fixtures/valid/smt_disabled.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let mut json = reference_observed_json();
    json = json.replace("\"smt_enabled\": false", "\"smt_enabled\": true");
    let observed = parse_observed_platform_json(&json).expect("parse");
    let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
    assert!(err.message.contains("SMT"));
}

#[test]
fn validate_warns_when_smt_enabled_under_exclusive_core_policy() {
    let compiled = reference_compiled();
    assert_eq!(compiled.requirements.smt_policy, SmtPolicy::ExclusiveCore);
    let mut json = reference_observed_json();
    json = json.replace("\"smt_enabled\": false", "\"smt_enabled\": true");
    let observed = parse_observed_platform_json(&json).expect("parse");
    let (_validated, warnings) = validate_platform(&compiled.requirements, &observed).expect("validate");
    assert_eq!(warnings.len(), 1);
}

#[test]
fn validate_rejects_each_required_feature_when_missing() {
    let compiled = reference_compiled();
    for field in ["vmx", "ept", "vtd", "interrupt_remapping", "nx"] {
        let mut json = reference_observed_json();
        json = json.replace(&format!("\"{field}\": true"), &format!("\"{field}\": false"));
        let observed = parse_observed_platform_json(&json).expect("parse");
        let err = validate_platform(&compiled.requirements, &observed).expect_err("must fail");
        assert!(err.message.contains(field));
    }
    assert_eq!(compiled.requirements.vmx, FeatureRequirement::Required);
}

#[test]
fn planner_covers_all_reference_partitions_and_channels() {
    let compiled = reference_compiled();
    let planned = plan_static_platform_ir(&compiled.intent).expect("plan");
    assert_eq!(planned.platform_name, compiled.intent.platform_name);
    assert!(planned.hypervisor_reserve.size.bytes() > 0);
}
