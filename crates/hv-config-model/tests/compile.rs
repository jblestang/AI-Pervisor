//! Integration tests for configuration compilation.

use hv_config_model::{compile_config_from_str, ConfigErrorKind};

#[test]
fn compiles_reference_config() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("reference config must compile");
    assert_eq!(compiled.normalized.partitions.len(), 3);
    assert_eq!(compiled.normalized.ipc_channels.len(), 2);
    assert_eq!(compiled.digest.to_hex().len(), 64);
}

#[test]
fn rejects_unknown_yaml_field() {
    let yaml = include_str!("fixtures/invalid/unknown_field.yaml");
    let err = compile_config_from_str(yaml).expect_err("unknown field");
    assert_eq!(err.kind, ConfigErrorKind::Parse);
}

#[test]
fn rejects_duplicate_partitions() {
    let yaml = include_str!("fixtures/invalid/duplicate_partition.yaml");
    let err = compile_config_from_str(yaml).expect_err("duplicate partition");
    assert_eq!(err.kind, ConfigErrorKind::Semantic);
}

proptest::proptest! {
    #[test]
    fn vm_ids_are_dense(_seed in 0u32..4) {
        use proptest::prop_assert_eq;
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        for (index, partition) in compiled.normalized.partitions.iter().enumerate() {
            prop_assert_eq!(partition.vm_id.raw(), index as u32);
        }
    }
}
