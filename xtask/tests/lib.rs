//! xtask library tests.

#[test]
fn run_true_succeeds() {
    assert_eq!(xtask::run("true", &[]), 0);
}

#[test]
fn run_false_returns_nonzero() {
    assert_eq!(xtask::run("false", &[]), 1);
}

#[test]
fn run_missing_command_returns_error_code() {
    assert_eq!(xtask::run("/no/such/binary", &[]), 1);
}

#[test]
fn run_config_validate_reference() {
    let path = format!("{}/../configs/qemu.yaml", env!("CARGO_MANIFEST_DIR"));
    assert_eq!(xtask::run_config_validate(path.as_ref()), 0);
}

#[test]
fn run_config_generate_reference() {
    let path = format!("{}/../configs/qemu.yaml", env!("CARGO_MANIFEST_DIR"));
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dir.path().to_string_lossy().to_string();
    assert_eq!(
        xtask::run_config_generate(path.as_ref(), output.as_ref()),
        0
    );
}

#[test]
fn run_build_succeeds() {
    assert_eq!(xtask::run_build(), 0);
}
