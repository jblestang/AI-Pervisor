//! xtask CLI integration tests covering main dispatch paths.

use std::path::PathBuf;
use std::process::Command;

use hv_config::artifacts::CONFIG_SHA256;
use hv_config::constants::{CLI_EXIT_USAGE, DEFAULT_CONFIG_OUTPUT_DIR};

fn qemu_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../configs/qemu.yaml")
}

#[test]
fn cli_build_succeeds() {
    let status = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("build")
        .status()
        .expect("spawn");
    assert!(status.success());
}

#[test]
fn cli_missing_subcommand_exits_with_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(CLI_EXIT_USAGE));
}

#[test]
fn cli_config_generate_uses_default_output() {
    let path = qemu_config_path();
    let output = tempfile::tempdir().expect("tempdir");
    let status = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(output.path())
        .args(["config", "generate", path.to_str().expect("utf8 path")])
        .status()
        .expect("spawn");
    assert!(status.success());
    assert!(output
        .path()
        .join(DEFAULT_CONFIG_OUTPUT_DIR)
        .join(CONFIG_SHA256)
        .is_file());
}

#[test]
fn cli_config_validate_succeeds() {
    let path = qemu_config_path();
    let status = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["config", "validate", path.to_str().expect("utf8 path")])
        .status()
        .expect("spawn");
    assert!(status.success());
}

#[test]
fn cli_config_generate_succeeds() {
    let path = qemu_config_path();
    let output = tempfile::tempdir().expect("tempdir");
    let status = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "config",
            "generate",
            path.to_str().expect("utf8 path"),
            "-o",
            output.path().to_str().expect("utf8 path"),
        ])
        .status()
        .expect("spawn");
    assert!(status.success());
}
