//! CLI entry point integration tests.

use std::path::Path;
use std::process::Command;

use hv_config::artifacts::CONFIG_SHA256;
use hv_config::constants::{CLI_EXIT_USAGE, DEFAULT_CONFIG_OUTPUT_DIR};

#[test]
fn cli_validate_succeeds() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_hv-config"))
        .args(["validate", path.to_str().expect("utf8 path")])
        .status()
        .expect("spawn");
    assert!(status.success());
}

#[test]
fn cli_generate_subcommand_succeeds() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
    let output = tempfile::tempdir().expect("tempdir");
    let status = Command::new(env!("CARGO_BIN_EXE_hv-config"))
        .args([
            "generate",
            path.to_str().expect("utf8 path"),
            "-o",
            output.path().to_str().expect("utf8 path"),
        ])
        .status()
        .expect("spawn");
    assert!(status.success());
}

#[test]
fn cli_missing_subcommand_exits_with_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_hv-config"))
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(CLI_EXIT_USAGE));
}

#[test]
fn cli_validate_rejects_missing_file() {
    let status = Command::new(env!("CARGO_BIN_EXE_hv-config"))
        .args(["validate", "/no/such/config.yaml"])
        .status()
        .expect("spawn");
    assert!(!status.success());
}

#[test]
fn cli_generate_uses_default_output_directory() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
    let output = tempfile::tempdir().expect("tempdir");
    let status = Command::new(env!("CARGO_BIN_EXE_hv-config"))
        .current_dir(output.path())
        .args(["generate", path.to_str().expect("utf8 path")])
        .status()
        .expect("spawn");
    assert!(status.success());
    assert!(
        output
            .path()
            .join(DEFAULT_CONFIG_OUTPUT_DIR)
            .join(CONFIG_SHA256)
            .is_file()
    );
}
