//! VMX launch CPU seam tests.

#![allow(clippy::expect_used)]

use hv_config_model::compile_config_from_str;
use hv_guest_boot::GUEST_SMOKE_IMAGE;
use hv_platform_model::plan_static_platform_ir;
use hv_vmx::{
    plan_vmx_init, plan_vmx_launch, program_vmcs_fields, DEFAULT_SMOKE_GUEST_PARTITION_ID,
};
use hv_x86_cpu::{
    install_guest_image, run_vmx_launch_cpu_seam, CpuInstructionDisposition, MockPageAllocator,
};

#[test]
fn run_vmx_launch_cpu_seam_keeps_validate_only_without_live_environment() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
    let launch =
        plan_vmx_launch(&layout, &vmx_plan, DEFAULT_SMOKE_GUEST_PARTITION_ID).expect("launch");
    let fields = program_vmcs_fields(&launch);
    let outcome = run_vmx_launch_cpu_seam(0x3000, &fields, launch.vm_id).expect("seam");
    assert_ne!(outcome.disposition, CpuInstructionDisposition::Executed);
}

#[test]
fn install_guest_image_records_copy_via_mock_allocator() {
    let mut allocator = MockPageAllocator::new(0x0000_0000_0B00_0000);
    let guest_phys = install_guest_image(&mut allocator, GUEST_SMOKE_IMAGE).expect("install");
    assert!(guest_phys > 0);
    assert!(!allocator.copies.is_empty());
}
