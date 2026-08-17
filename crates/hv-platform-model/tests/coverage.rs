//! Coverage-oriented tests for platform validation and planning edge cases.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::{compile_config_from_str, FeatureRequirement, SmtPolicy};
use hv_platform_model::{
    observe_platform, parse_observed_platform_json, plan_static_platform_ir, validate_platform,
    CpuidSnapshot, ObservationInputs, PlatformErrorKind,
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

#[test]
fn observe_platform_rejects_descriptor_size_too_small() {
    let err = observe_platform(&ObservationInputs {
        cpuid: CpuidSnapshot {
            leaf1_ecx: 0,
            leaf1_edx: 0,
            leaf1_ebx: 0,
            leaf80000007_edx: None,
            leaf80000008_ecx: None,
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        acpi_tables: Vec::new(),
        memory_map: vec![0u8; 48],
        memory_descriptor_size: 8,
        pci_devices: Vec::new(),
    })
    .expect_err("must fail");
    assert_eq!(err.kind, PlatformErrorKind::Observation);
}

#[test]
fn observe_platform_without_dmar_reports_no_iommu() {
    let mut memory_map = vec![0u8; 48];
    memory_map[0..4].copy_from_slice(&hv_boot_abi::EFI_MEMORY_CONVENTIONAL.to_le_bytes());
    memory_map[24..32].copy_from_slice(&1u64.to_le_bytes());
    let observed = observe_platform(&ObservationInputs {
        cpuid: CpuidSnapshot {
            leaf1_ecx: 0,
            leaf1_edx: 0,
            leaf1_ebx: (2 << 16) | 2,
            leaf80000007_edx: None,
            leaf80000008_ecx: Some(0),
            leaf480_ecx: None,
            leaf480_ebx: None,
        },
        acpi_tables: Vec::new(),
        memory_map,
        memory_descriptor_size: 48,
        pci_devices: Vec::new(),
    })
    .expect("observe");
    assert!(!observed.vtd);
    assert!(!observed.interrupt_remapping);
    assert!(observed.smt_enabled);
}

#[test]
fn cpuid_snapshot_feature_helpers_cover_absent_leaves() {
    let snapshot = CpuidSnapshot {
        leaf1_ecx: 0,
        leaf1_edx: 0,
        leaf1_ebx: 1 << 16,
        leaf80000007_edx: None,
        leaf80000008_ecx: None,
        leaf480_ecx: None,
        leaf480_ebx: None,
    };
    assert!(!snapshot.vmx());
    assert!(!snapshot.nx());
    assert!(!snapshot.x2apic());
    assert!(!snapshot.invariant_tsc());
    assert!(!snapshot.ept());
    assert!(!snapshot.vpid());
    assert!(!snapshot.vmx_preemption_timer());
    assert!(!snapshot.smt_enabled());
    assert_eq!(snapshot.physical_cores(), 1);
}
