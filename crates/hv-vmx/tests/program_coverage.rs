//! Additional coverage for VMX hardware programming paths.

use hv_config_model::compile_config_from_str;
use hv_platform_model::plan_static_platform_ir;
use hv_vmx::{plan_vmx_init, program_vmxon_region, REFERENCE_VMXON_REVISION, VMXON_REGION_MIN_BYTES};
use hv_types::ByteSize;

#[test]
fn program_vmxon_region_rejects_undersized_plan() {
    use hv_types::HostPhysAddr;
    use hv_vmx::{VmxErrorKind, VmxInitPlan};

    let plan = VmxInitPlan {
        vmxon_region_phys: HostPhysAddr::new(0x1000),
        vmxon_region_bytes: ByteSize::new(VMXON_REGION_MIN_BYTES - 1),
    };
    let err = program_vmxon_region(&plan, REFERENCE_VMXON_REVISION).expect_err("must fail");
    assert_eq!(err.kind, VmxErrorKind::Planning);
}

#[test]
fn program_vmxon_region_accepts_reference_layout() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
    let region = program_vmxon_region(&plan, REFERENCE_VMXON_REVISION).expect("program");
    assert!(region.bytes.len() >= VMXON_REGION_MIN_BYTES as usize);
}
