//! Compromised-guest IPC attack fixture tests.

#![allow(clippy::expect_used)]

use hv_config_model::compile_config_from_str;
use hv_datapath::{
    apply_compromised_guest_write, enforce_forward_integrity, plan_datapath_forward,
    run_reference_compromised_scenarios, CompromisedGuestAction, DatapathErrorKind, E1000Partition, IpcChannelSelector, REFERENCE_COMPROMISED_SCENARIOS,
    REFERENCE_IPC_SLOT_SIZE_BYTES,
};
use hv_platform_model::plan_static_platform_ir;

fn reference_forward_plan() -> Result<hv_datapath::DatapathForwardPlan, hv_datapath::DatapathError> {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    plan_datapath_forward(&layout)
}

#[test]
fn reference_compromised_scenarios_all_blocked() {
    let (integrity_ok, blocked) =
        run_reference_compromised_scenarios(reference_forward_plan).expect("suite");
    assert!(integrity_ok);
    assert_eq!(blocked, REFERENCE_COMPROMISED_SCENARIOS.len() as u32);
}

#[test]
fn forged_slot_metadata_detected_by_integrity_scan() {
    let mut plan = reference_forward_plan().expect("plan");
    apply_compromised_guest_write(
        &mut plan,
        CompromisedGuestAction::ForgedSlotMetadata {
            channel: IpcChannelSelector::ChanB,
            slot_index: 0,
            forged_payload_len: REFERENCE_IPC_SLOT_SIZE_BYTES + 128,
        },
    )
    .expect("apply");
    let err = enforce_forward_integrity(&plan).expect_err("must fail");
    assert_eq!(err.kind, DatapathErrorKind::IpcViolation);
}

#[test]
fn corrupt_head_tail_detected_by_integrity_scan() {
    let mut plan = reference_forward_plan().expect("plan");
    apply_compromised_guest_write(
        &mut plan,
        CompromisedGuestAction::CorruptHeadTail {
            channel: IpcChannelSelector::ChanA,
            forged_head: 1,
            forged_tail: 4,
        },
    )
    .expect("apply");
    assert!(enforce_forward_integrity(&plan).is_err());
}

#[test]
fn cross_partition_chan_a_corruption_detected() {
    let mut plan = reference_forward_plan().expect("plan");
    apply_compromised_guest_write(&mut plan, CompromisedGuestAction::CrossPartitionCorruptChanA)
        .expect("apply");
    assert!(enforce_forward_integrity(&plan).is_err());
}

#[test]
fn stale_slot_replay_detected() {
    let mut plan = reference_forward_plan().expect("plan");
    apply_compromised_guest_write(
        &mut plan,
        CompromisedGuestAction::StaleSlotReplay {
            channel: IpcChannelSelector::ChanA,
            slot_index: 0,
        },
    )
    .expect("apply");
    assert!(enforce_forward_integrity(&plan).is_err());
}

#[test]
fn e1000_read_only_write_rejected_at_apply() {
    let mut plan = reference_forward_plan().expect("plan");
    let err = apply_compromised_guest_write(
        &mut plan,
        CompromisedGuestAction::E1000ReadOnlyWrite {
            partition: E1000Partition::In,
            offset: hv_datapath::E1000_REG_TDH,
            value: 1,
        },
    )
    .expect_err("must fail");
    assert_eq!(err.kind, DatapathErrorKind::IpcViolation);
}
