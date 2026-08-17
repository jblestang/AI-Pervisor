//! hv-config dispatch tests.

#![allow(clippy::expect_used)]

use std::path::PathBuf;

use hv_config::{dispatch_config, validate_config, ConfigCommand};

#[test]
fn dispatch_validate_and_generate_reference_config() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
    assert_eq!(
        dispatch_config(ConfigCommand::Validate {
            path: path.clone()
        }),
        0
    );
    let output = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        dispatch_config(ConfigCommand::Generate {
            path,
            output: output.path().to_path_buf(),
        }),
        0
    );
}

#[test]
fn validate_config_rejects_missing_file_via_dispatch() {
    let path = PathBuf::from("/no/such/config.yaml");
    assert_eq!(validate_config(&path), 1);
    assert_eq!(
        dispatch_config(ConfigCommand::Validate { path }),
        1
    );
}
