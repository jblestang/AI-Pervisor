//! Guest datapath runtime fixture tests.

#![allow(clippy::expect_used)]

use hv_config_model::compile_config_from_str;
use hv_datapath::{run_guest_datapath_runtime, GUEST_DATAPATH_IPC_HOPS};
use hv_platform_model::plan_static_platform_ir;

#[test]
fn guest_datapath_runtime_matches_reference_topology() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let (_plan, outcome) = run_guest_datapath_runtime(&layout).expect("runtime");
    assert!(outcome.guest_frame_forwarded);
    assert_eq!(outcome.ipc_hops_completed, GUEST_DATAPATH_IPC_HOPS);
}
