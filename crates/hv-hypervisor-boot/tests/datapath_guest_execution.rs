//! Gate D datapath guest execution orchestration tests (requires `datapath-guest-execution`).

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::compile_config_from_str;
use hv_datapath::DatapathRuntimeDisposition;
use hv_guest_boot::{GUEST_SOURCE_ELFS_AVAILABLE, REFERENCE_GUEST_PARTITION_IDS};
use hv_hypervisor_boot::{
    boot_from_transfer_and_init_gate_d_datapath_guest_execution,
    boot_from_transfer_and_init_gate_d_datapath_guest_execution_from_snapshots,
    layout_snapshot_from_platform_ir, requirements_snapshot_from_platform,
    GateDDatapathGuestExecutionResult,
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
use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};
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
                memory_map[0..4].copy_from_slice(&hv_boot_abi::EFI_MEMORY_CONVENTIONAL.to_le_bytes());
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

fn assert_datapath_guest_execution_validate_only(result: &GateDDatapathGuestExecutionResult) {
    assert_eq!(
        result.live.boot_infos_installed,
        REFERENCE_GUEST_PARTITION_IDS.len() as u32
    );
    assert!(result.live.sources.runtime.runtime.guest_frame_forwarded);
    assert_eq!(
        result.execution_seam.partitions_validated,
        REFERENCE_GUEST_PARTITION_IDS.len() as u32
    );
    assert!(result.execution_seam.vmexit_stub_validated);
    assert_ne!(
        result.execution_seam.disposition,
        CpuInstructionDisposition::Executed
    );
    assert!(
        matches!(
            result.live.sources.runtime.runtime.disposition,
            DatapathRuntimeDisposition::ValidatedOnly | DatapathRuntimeDisposition::Unavailable
        ),
        "unexpected runtime disposition: {:?}",
        result.live.sources.runtime.runtime.disposition
    );
    for record in &result.live.sources.runtime.benchmark.guests.partition_launches {
        let boot_info_phys = record
            .boot_info_guest_phys
            .expect("boot info installed for partition");
        assert!(boot_info_phys > record.guest_entry_phys);
    }
}

#[test]
fn boot_from_transfer_and_init_gate_d_datapath_guest_execution_accepts_reference_transfer() {
    if !GUEST_SOURCE_ELFS_AVAILABLE {
        eprintln!("skipping: run cargo xtask build-guests to embed source ELFs");
        return;
    }

    let (transfer, snapshot, layout) = reference_handoff_snapshot_and_layout();
    let mut allocator = MockPageAllocator::new(0x0000_0000_2100_0000);
    let result = boot_from_transfer_and_init_gate_d_datapath_guest_execution(
        &transfer,
        &snapshot,
        &layout,
        &mut allocator,
    )
    .expect("guest execution");
    assert_datapath_guest_execution_validate_only(&result);
}

#[test]
fn boot_from_transfer_and_init_gate_d_datapath_guest_execution_from_snapshots_accepts_reference_transfer(
) {
    if !GUEST_SOURCE_ELFS_AVAILABLE {
        eprintln!("skipping: run cargo xtask build-guests to embed source ELFs");
        return;
    }

    let (transfer, snapshot, layout) = reference_handoff_snapshot_and_layout();
    let layout_snapshot = layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
    let mut allocator = MockPageAllocator::new(0x0000_0000_2200_0000);
    let result = boot_from_transfer_and_init_gate_d_datapath_guest_execution_from_snapshots(
        &transfer,
        &snapshot,
        &layout_snapshot,
        &mut allocator,
    )
    .expect("guest execution snapshots");
    assert_datapath_guest_execution_validate_only(&result);
}
