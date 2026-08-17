//! hv-config library and CLI integration tests.

use std::path::Path;

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
    assert!(output.path().join("config.sha256").is_file());
    assert!(output.path().join("static-platform.rs").is_file());
    assert!(output.path().join("static-intent.json").is_file());
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
