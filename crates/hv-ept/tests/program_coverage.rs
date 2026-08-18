//! Additional coverage for EPT hardware programming paths.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::compile_config_from_str;
use hv_ept::{
    encode_identity_ept_entry, plan_ept_init, program_ept_tables, EptErrorKind, EptIdentityMapping,
    EptInitPlan, EPT_ENTRY_READ, EPT_PAGE_SIZE_BYTES,
};
use hv_platform_model::plan_static_platform_ir;
use hv_types::{ByteSize, HostPhysAddr};
use hv_vmx::plan_vmx_init;

#[test]
fn program_ept_tables_accepts_empty_identity_mappings() {
    let plan = EptInitPlan {
        identity_mappings: vec![],
        root_table_phys: HostPhysAddr::new(0x2000),
        root_table_bytes: ByteSize::new(4096),
    };
    let tables = program_ept_tables(&plan).expect("program");
    assert!(tables.mappings.is_empty());
    assert_eq!(tables.root_table.len(), 4096);
}

#[test]
fn encode_identity_ept_entry_sets_read_bit() {
    let entry = encode_identity_ept_entry(0x10_0000);
    assert_ne!(entry & EPT_ENTRY_READ, 0);
}

#[test]
fn program_ept_tables_rejects_zero_sized_mapping() {
    let plan = EptInitPlan {
        identity_mappings: vec![EptIdentityMapping {
            guest_phys: HostPhysAddr::new(0x1000),
            host_phys: HostPhysAddr::new(0x1000),
            size_bytes: ByteSize::new(0),
        }],
        root_table_phys: HostPhysAddr::new(0x2000),
        root_table_bytes: ByteSize::new(4096),
    };
    let err = program_ept_tables(&plan).expect_err("must fail");
    assert_eq!(err.kind, EptErrorKind::Planning);
}

#[test]
fn program_ept_tables_rejects_unaligned_guest_base() {
    let plan = EptInitPlan {
        identity_mappings: vec![EptIdentityMapping {
            guest_phys: HostPhysAddr::new(0x1001),
            host_phys: HostPhysAddr::new(0x1000),
            size_bytes: ByteSize::new(EPT_PAGE_SIZE_BYTES),
        }],
        root_table_phys: HostPhysAddr::new(0x2000),
        root_table_bytes: ByteSize::new(4096),
    };
    let err = program_ept_tables(&plan).expect_err("must fail");
    assert_eq!(err.kind, EptErrorKind::Planning);
}

#[test]
fn program_ept_tables_accepts_reference_layout() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
    let plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
    let tables = program_ept_tables(&plan).expect("program");
    assert!(!tables.mappings.is_empty());
}
