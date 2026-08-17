//! Additional validation coverage for optional feature handling.

use hv_config_model::compile_config_from_str;
use hv_platform_model::{parse_observed_platform_json, validate_platform};

#[test]
fn optional_features_do_not_require_presence() {
    let yaml = include_str!("../../hv-config-model/tests/fixtures/valid/all_feature_levels.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let json = include_str!("fixtures/observed/qemu_reference.json");
    let mut observed = parse_observed_platform_json(json).expect("parse");
    observed.vmx = false;
    observed.ept = false;
    observed.vtd = false;
    observed.vpid = false;
    observed.interrupt_remapping = true;
    observed.nx = true;
    validate_platform(&compiled.requirements, &observed).expect("optional levels tolerate absence");
}
