//! hv-config library and CLI integration tests.

use std::path::Path;

use hv_config::artifacts::{
    CONFIG_SHA256, HYPERVISOR_EMBEDDED_CONFIG_RS, PLATFORM_LAYOUT, STATIC_INTENT_JSON,
    STATIC_PLATFORM_LAYOUT_JSON, STATIC_PLATFORM_RS,
};
use hv_config::validate_config;
use tempfile::tempdir;

#[test]
fn validate_config_accepts_reference() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
    assert_eq!(validate_config(&path), 0);
}

#[test]
fn validate_config_rejects_missing_file() {
    let path = Path::new("/no/such/config.yaml");
    assert_eq!(validate_config(path), 1);
}

#[test]
fn generate_writes_artifacts() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
    let output = tempdir().expect("tempdir");
    let status = hv_config::generate::generate(&path, output.path());
    assert_eq!(status, 0);
    assert!(output.path().join(CONFIG_SHA256).is_file());
    assert!(output.path().join(STATIC_PLATFORM_RS).is_file());
    assert!(output.path().join(STATIC_INTENT_JSON).is_file());
    assert!(output.path().join(STATIC_PLATFORM_LAYOUT_JSON).is_file());
    assert!(output.path().join(PLATFORM_LAYOUT).is_file());
    assert!(output.path().join(HYPERVISOR_EMBEDDED_CONFIG_RS).is_file());
    let embedded = std::fs::read_to_string(output.path().join(HYPERVISOR_EMBEDDED_CONFIG_RS))
        .expect("embedded config");
    assert!(embedded.contains("REQUIREMENTS_SNAPSHOT"));
    assert!(embedded.contains("hypervisor_reserve_phys"));
}

#[test]
fn validate_config_prints_warnings() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/hv-config-model/tests/fixtures/valid/allow_cross_partition.yaml");
    assert_eq!(validate_config(&path), 0);
}

#[test]
fn generate_rejects_invalid_config() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/hv-config-model/tests/fixtures/invalid/bad_arch.yaml");
    let output = tempdir().expect("tempdir");
    assert_eq!(hv_config::generate::generate(&path, output.path()), 1);
}

#[test]
fn generate_fails_when_output_is_read_only() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
    let output = tempdir().expect("tempdir");
    let mut perms = std::fs::metadata(output.path())
        .expect("metadata")
        .permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(output.path(), perms).expect("set readonly");
    assert_eq!(hv_config::generate::generate(&path, output.path()), 1);
}
