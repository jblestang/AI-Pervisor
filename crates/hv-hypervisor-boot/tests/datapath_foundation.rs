//! Gate D datapath foundation orchestration tests (requires `datapath-foundation` feature).

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::compile_config_from_str;
use hv_guest_abi::GuestIpcRole;
use hv_guest_boot::GuestBootInfoView;
use hv_hypervisor_boot::{
    boot_from_transfer_and_init_gate_d_datapath_foundation,
    boot_from_transfer_and_init_gate_d_datapath_foundation_from_snapshots,
    layout_snapshot_from_platform_ir, requirements_snapshot_from_platform,
    GateDDatapathFoundationResult,
};
use hv_loader::{
    build_hypervisor_transfer, build_loader_handoff, encode_qemu_reference_firmware,
    LoaderHandoffInput,
};
use hv_observation_types::{
    CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
    CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
    CPUID_80000007_EDX_INVARIANT_TSC_BIT,
};
use hv_platform_model::plan_static_platform_ir;
use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment, VmId};
use hv_x86_cpu::{CpuInstructionDisposition, MockPageAllocator};

fn reference_handoff_snapshot_and_layout() -> (
    Vec<u8>,
    hv_boot_abi::RequirementsSnapshot,
    hv_platform_model::StaticPlatformIR,
) {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let firmware = encode_qemu_reference_firmware();
    let handoff = build_loader_handoff(
        &LoaderHandoffInput::with_default_descriptor_size(
            compiled.digest.bytes,
            {
                let mut memory_map = vec![0u8; 48];
                memory_map[0..4]
                    .copy_from_slice(&hv_boot_abi::EFI_MEMORY_CONVENTIONAL.to_le_bytes());
                memory_map[24..32].copy_from_slice(&(2_097_152u64).to_le_bytes());
                memory_map
            },
            firmware
                .bytes
                .get(0x1000..0x1000 + 36)
                .expect("rsdp")
                .to_vec(),
            CpuidSnapshot {
                leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
                leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
                leaf1_ebx: (4 << 16) | 4,
                leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
                leaf80000008_ecx: Some(3),
                leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
                leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
            },
            vec![
                PciBdf::new(
                    PciSegment::new(0),
                    PciBus::new(0),
                    PciDevice::new(3),
                    PciFunction::new(0),
                ),
                PciBdf::new(
                    PciSegment::new(0),
                    PciBus::new(0),
                    PciDevice::new(4),
                    PciFunction::new(0),
                ),
            ],
        ),
        &firmware,
    )
    .expect("handoff");
    let transfer = build_hypervisor_transfer(&handoff).expect("transfer");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let snapshot = requirements_snapshot_from_platform(
        &compiled.requirements,
        compiled.digest.bytes,
        layout.hypervisor_reserve.host_phys.raw(),
        layout.hypervisor_reserve.size.bytes(),
    )
    .expect("snapshot");
    (transfer, snapshot, layout)
}

fn assert_datapath_foundation_validate_only(result: &GateDDatapathFoundationResult) {
    assert!(!result.vmx_launch.real_hw.live.live_environment_ready);
    assert!(result.vmx_launch.real_hw.vmcs_phys.is_some());
    assert!(result.vmx_launch.guest_entry_phys.is_some());
    assert!(result
        .vmx_launch
        .guest_boot_info
        .as_ref()
        .is_some_and(|info| !info.is_empty()));
    if let Some(launch) = &result.vmx_launch.launch_seam {
        assert_ne!(launch.disposition, CpuInstructionDisposition::Executed);
    }
    assert_eq!(result.partition_boot_infos.len(), 3);
    assert_eq!(result.datapath_plans.len(), 3);

    let in_blob = result
        .partition_boot_infos
        .iter()
        .find(|(vm_id, _)| *vm_id == VmId::new(0))
        .map(|(_, blob)| blob.as_slice())
        .expect("in blob");
    let in_view = GuestBootInfoView::parse(in_blob).expect("parse in");
    assert_eq!(in_view.header().ipc_region_count, 1);
    assert_eq!(in_view.header().device_region_count, 1);
    assert_eq!(
        in_view.ipc_region(0).expect("ipc").role,
        GuestIpcRole::Producer
    );

    let mid_blob = result
        .partition_boot_infos
        .iter()
        .find(|(vm_id, _)| *vm_id == VmId::new(1))
        .map(|(_, blob)| blob.as_slice())
        .expect("mid blob");
    let mid_view = GuestBootInfoView::parse(mid_blob).expect("parse mid");
    assert_eq!(mid_view.header().ipc_region_count, 2);
    assert_eq!(mid_view.header().device_region_count, 0);

    let out_blob = result
        .partition_boot_infos
        .iter()
        .find(|(vm_id, _)| *vm_id == VmId::new(2))
        .map(|(_, blob)| blob.as_slice())
        .expect("out blob");
    let out_view = GuestBootInfoView::parse(out_blob).expect("parse out");
    assert_eq!(out_view.header().ipc_region_count, 1);
    assert_eq!(out_view.header().device_region_count, 1);
    assert_eq!(
        out_view.ipc_region(0).expect("ipc").role,
        GuestIpcRole::Consumer
    );
}

#[test]
fn boot_from_transfer_and_init_gate_d_datapath_foundation_accepts_reference_transfer() {
    let (transfer, snapshot, layout) = reference_handoff_snapshot_and_layout();
    let mut allocator = MockPageAllocator::new(0x0000_0000_0B00_0000);
    let result = boot_from_transfer_and_init_gate_d_datapath_foundation(
        &transfer,
        &snapshot,
        &layout,
        &mut allocator,
    )
    .expect("datapath foundation");
    assert_datapath_foundation_validate_only(&result);
}

#[test]
fn boot_from_transfer_and_init_gate_d_datapath_foundation_from_snapshots_accepts_reference_transfer(
) {
    let (transfer, snapshot, layout) = reference_handoff_snapshot_and_layout();
    let layout_snapshot = layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
    let mut allocator = MockPageAllocator::new(0x0000_0000_0C00_0000);
    let result = boot_from_transfer_and_init_gate_d_datapath_foundation_from_snapshots(
        &transfer,
        &snapshot,
        &layout_snapshot,
        &mut allocator,
    )
    .expect("datapath foundation snapshots");
    assert_datapath_foundation_validate_only(&result);
}
