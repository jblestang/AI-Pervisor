//! Additional normalization coverage.

#![allow(clippy::expect_used)]

use hv_config_model::{
    compile_config_from_str, load_raw_from_str, normalize, validate_semantics, validate_syntax,
    ConfigErrorKind,
};

#[test]
fn compile_rejects_overflow_memory() {
    let yaml = include_str!("fixtures/invalid/huge_memory_gib.yaml");
    let err = compile_config_from_str(yaml).expect_err("overflow");
    assert_eq!(err.kind, ConfigErrorKind::Arithmetic);
}

#[test]
fn compile_rejects_ipc_shared_memory_overflow() {
    let yaml = include_str!("fixtures/invalid/ipc_shared_overflow.yaml");
    let err = compile_config_from_str(yaml).expect_err("ipc overflow");
    assert_eq!(err.kind, ConfigErrorKind::Arithmetic);
}

#[test]
fn compile_rejects_min_ram_overflow() {
    let yaml = include_str!("fixtures/invalid/min_ram_overflow.yaml");
    let err = compile_config_from_str(yaml).expect_err("min ram overflow");
    assert_eq!(err.kind, ConfigErrorKind::Arithmetic);
}

#[test]
fn compile_rejects_guest_memory_sum_overflow() {
    let yaml = include_str!("fixtures/invalid/guest_memory_sum_overflow.yaml");
    let err = compile_config_from_str(yaml).expect_err("guest memory sum overflow");
    assert_eq!(err.kind, ConfigErrorKind::Arithmetic);
}

#[test]
fn compile_rejects_ipc_sum_overflow() {
    let yaml = include_str!("fixtures/invalid/ipc_sum_overflow.yaml");
    let err = compile_config_from_str(yaml).expect_err("ipc sum overflow");
    assert_eq!(err.kind, ConfigErrorKind::Arithmetic);
}

#[test]
fn smt_disabled_fixture_normalizes_disabled_policy() {
    let yaml = include_str!("fixtures/valid/smt_disabled.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile smt disabled");
    assert_eq!(
        compiled.normalized.requirements.smt_policy,
        hv_config_model::NormalizedSmtPolicy::Disabled
    );
}

#[test]
fn normalize_rejects_invalid_device_bdf() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let mut raw = load_raw_from_str(yaml).expect("parse");
    let device = raw
        .partitions
        .first_mut()
        .and_then(|partition| partition.devices.first_mut())
        .expect("device fixture");
    device.bdf = "not-a-bdf".to_string();
    let err = normalize(raw).expect_err("invalid bdf");
    assert_eq!(err.kind, ConfigErrorKind::Syntax);
}

#[test]
fn normalize_sorts_devices_by_bdf() {
    let yaml = include_str!("fixtures/valid/all_feature_levels.yaml");
    let raw = load_raw_from_str(yaml).expect("parse");
    validate_syntax(&raw).expect("syntax");
    validate_semantics(&raw).expect("semantic");
    let normalized = normalize(raw).expect("normalize");
    let devices = &normalized.partitions.first().expect("partition").devices;
    assert_eq!(devices.len(), 2);
    let first = devices.first().expect("first device");
    let second = devices.get(1).expect("second device");
    assert!(first.bdf.device.raw() <= second.bdf.device.raw());
}
