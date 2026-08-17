//! Broad validation and compilation coverage tests.

#![allow(clippy::assertions_on_constants)]

use hv_config_model::{
    compile_config_from_str, load_raw_from_str, validate_semantics, validate_syntax, ConfigErrorKind,
};

fn load_fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => {
            assert!(false, "read fixture {name}: {err}");
            String::new()
        }
    }
}

fn assert_syntax_error(fixture: &str) {
    let yaml = load_fixture(fixture);
    let raw = match load_raw_from_str(&yaml) {
        Ok(raw) => raw,
        Err(err) => {
            assert!(false, "yaml parse: {err}");
            return;
        }
    };
    match validate_syntax(&raw) {
        Err(err) => assert_eq!(err.kind, ConfigErrorKind::Syntax),
        Ok(()) => {
            let failed = std::hint::black_box(false);
            assert!(failed, "syntax error expected");
        }
    }
}

fn assert_semantic_error(fixture: &str) {
    let yaml = load_fixture(fixture);
    let raw = match load_raw_from_str(&yaml) {
        Ok(raw) => raw,
        Err(err) => {
            assert!(false, "yaml parse: {err}");
            return;
        }
    };
    if let Err(err) = validate_syntax(&raw) {
        assert!(false, "syntax should pass: {err}");
        return;
    }
    match validate_semantics(&raw) {
        Err(err) => assert_eq!(err.kind, ConfigErrorKind::Semantic),
        Ok(_) => {
            let failed = std::hint::black_box(false);
            assert!(failed, "semantic error expected");
        }
    }
}

#[test]
fn reference_config_compiles_with_requirements_and_intent() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = match compile_config_from_str(yaml) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(false, "compile: {err}");
            return;
        }
    };
    assert_eq!(compiled.requirements.expected_pci_devices.len(), 2);
    assert_eq!(compiled.intent.partitions.len(), 3);
    assert_eq!(compiled.warnings.len(), 0);
}

#[test]
fn minimal_config_compiles() {
    let yaml = load_fixture("valid/minimal.yaml");
    let compiled = match compile_config_from_str(&yaml) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(false, "compile minimal: {err}");
            return;
        }
    };
    assert_eq!(compiled.normalized.partitions.len(), 1);
}

#[test]
fn all_feature_levels_fixture_compiles() {
    let yaml = load_fixture("valid/all_feature_levels.yaml");
    let compiled = match compile_config_from_str(&yaml) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(false, "compile: {err}");
            return;
        }
    };
    let partition = match compiled.normalized.partitions.first() {
        Some(partition) => partition,
        None => {
            assert!(false, "missing partition");
            return;
        }
    };
    assert_eq!(partition.devices.len(), 2);
}

#[test]
fn smt_disabled_fixture_compiles() {
    let yaml = load_fixture("valid/smt_disabled.yaml");
    let compiled = match compile_config_from_str(&yaml) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(false, "compile: {err}");
            return;
        }
    };
    assert_eq!(
        compiled.normalized.requirements.smt_policy,
        hv_config_model::NormalizedSmtPolicy::Disabled
    );
}

#[test]
fn datapath_device_without_role_compiles_under_mid_policy() {
    let yaml = load_fixture("valid/datapath_device_without_role.yaml");
    match compile_config_from_str(&yaml) {
        Ok(_) => {}
        Err(err) => assert!(false, "compile: {err}"),
    }
}

#[test]
fn datapath_same_partition_gateway_compiles() {
    let yaml = load_fixture("valid/datapath_same_partition_gateway.yaml");
    match compile_config_from_str(&yaml) {
        Ok(_) => {}
        Err(err) => assert!(false, "compile: {err}"),
    }
}

#[test]
fn syntax_rejects_empty_platform_name() {
    assert_syntax_error("invalid/empty_platform_name.yaml");
}

#[test]
fn syntax_rejects_bad_arch() {
    assert_syntax_error("invalid/bad_arch.yaml");
}

#[test]
fn syntax_rejects_empty_partitions() {
    assert_syntax_error("invalid/empty_partitions.yaml");
}

#[test]
fn syntax_rejects_empty_partition_id() {
    assert_syntax_error("invalid/empty_partition_id.yaml");
}

#[test]
fn syntax_rejects_zero_vcpus() {
    assert_syntax_error("invalid/zero_vcpus.yaml");
}

#[test]
fn syntax_rejects_zero_memory() {
    assert_syntax_error("invalid/zero_memory_gib.yaml");
}

#[test]
fn syntax_rejects_empty_ipc_id() {
    assert_syntax_error("invalid/empty_ipc_id.yaml");
}

#[test]
fn syntax_rejects_zero_queue_slots() {
    assert_syntax_error("invalid/zero_queue_slots.yaml");
}

#[test]
fn syntax_rejects_zero_slot_size() {
    assert_syntax_error("invalid/zero_slot_size.yaml");
}

#[test]
fn semantic_rejects_duplicate_bdf() {
    assert_semantic_error("invalid/duplicate_bdf.yaml");
}

#[test]
fn semantic_rejects_overlapping_cores() {
    assert_semantic_error("invalid/overlapping_cores.yaml");
}

#[test]
fn semantic_rejects_duplicate_ipc() {
    assert_semantic_error("invalid/duplicate_ipc.yaml");
}

#[test]
fn semantic_rejects_unknown_ipc_producer() {
    assert_semantic_error("invalid/unknown_ipc_producer.yaml");
}

#[test]
fn semantic_rejects_unknown_ipc_consumer() {
    assert_semantic_error("invalid/unknown_ipc_consumer.yaml");
}

#[test]
fn semantic_rejects_ipc_self_loop() {
    assert_semantic_error("invalid/ipc_self.yaml");
}

#[test]
fn semantic_rejects_ipc_cycle() {
    assert_semantic_error("invalid/ipc_cycle.yaml");
}

#[test]
fn semantic_rejects_direct_in_out_bypass() {
    assert_semantic_error("invalid/direct_in_out.yaml");
}

#[test]
fn semantic_rejects_bad_guest_partition() {
    assert_semantic_error("invalid/bad_guest_partition.yaml");
}

#[test]
fn semantic_rejects_bad_sha256() {
    assert_semantic_error("invalid/bad_sha256.yaml");
}

#[test]
fn semantic_rejects_invalid_bdf() {
    let yaml = load_fixture("invalid/invalid_bdf.yaml");
    let raw = match load_raw_from_str(&yaml) {
        Ok(raw) => raw,
        Err(err) => {
            assert!(false, "yaml parse: {err}");
            return;
        }
    };
    if let Err(err) = validate_syntax(&raw) {
        assert!(false, "syntax ok: {err}");
        return;
    }
    match validate_semantics(&raw) {
        Err(err) => assert_eq!(err.kind, ConfigErrorKind::Syntax),
        Ok(_) => {
            let failed = std::hint::black_box(false);
            assert!(failed, "bdf error expected");
        }
    }
}

#[test]
fn allow_cross_partition_emits_warnings() {
    let yaml = load_fixture("valid/allow_cross_partition.yaml");
    let compiled = match compile_config_from_str(&yaml) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(false, "compile: {err}");
            return;
        }
    };
    assert_eq!(compiled.warnings.len(), 2);
}

#[test]
fn shared_core_non_exclusive_emits_warning() {
    let yaml = load_fixture("valid/shared_core_non_exclusive.yaml");
    let compiled = match compile_config_from_str(&yaml) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(false, "compile: {err}");
            return;
        }
    };
    assert_eq!(compiled.warnings.len(), 3);
}
