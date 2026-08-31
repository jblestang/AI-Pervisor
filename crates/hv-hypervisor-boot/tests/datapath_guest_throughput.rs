//! Gate D datapath guest throughput orchestration tests (requires `datapath-guest-throughput`).

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::compile_config_from_str;
use hv_datapath::{DatapathRuntimeDisposition, GuestThroughputDisposition};
use hv_guest_boot::{GUEST_SOURCE_ELFS_AVAILABLE, REFERENCE_GUEST_PARTITION_IDS};
use hv_hypervisor_boot::{
    boot_from_transfer_and_init_gate_d_datapath_guest_throughput,
    boot_from_transfer_and_init_gate_d_datapath_guest_throughput_from_snapshots,
    layout_snapshot_from_platform_ir, requirements_snapshot_from_platform,
    GateDDatapathGuestThroughputResult,
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

fn assert_datapath_guest_throughput_validate_only(result: &GateDDatapathGuestThroughputResult) {
    assert_eq!(
        result.execution.live.boot_infos_installed,
        REFERENCE_GUEST_PARTITION_IDS.len() as u32
    );
    assert!(result.execution.live.sources.runtime.runtime.guest_frame_forwarded);
    assert!(result.throughput.benchmark.target_met);
    assert!(result.throughput.guest_relay_frames > 0);
    assert!(
        matches!(
            result.throughput.disposition,
            GuestThroughputDisposition::ValidatedOnly | GuestThroughputDisposition::Unavailable
        ),
        "unexpected throughput disposition: {:?}",
        result.throughput.disposition
    );
    if result.throughput.disposition == GuestThroughputDisposition::Unavailable {
        assert_eq!(
            result.throughput_seam.disposition,
            CpuInstructionDisposition::SkippedNoHardware
        );
    }
    assert_ne!(
        result.throughput_seam.disposition,
        CpuInstructionDisposition::Executed
    );
    assert_eq!(
        result.throughput_seam.partitions_validated,
        REFERENCE_GUEST_PARTITION_IDS.len() as u32
    );
    assert_eq!(
        result.throughput_seam.measurement_runs_validated,
        result.throughput.benchmark.runs_completed
    );
    assert!(
        matches!(
            result.execution.live.sources.runtime.runtime.disposition,
            DatapathRuntimeDisposition::ValidatedOnly | DatapathRuntimeDisposition::Unavailable
        ),
        "unexpected runtime disposition: {:?}",
        result.execution.live.sources.runtime.runtime.disposition
    );
}

#[test]
fn boot_from_transfer_and_init_gate_d_datapath_guest_throughput_accepts_reference_transfer() {
    if !GUEST_SOURCE_ELFS_AVAILABLE {
        eprintln!("skipping: run cargo xtask build-guests to embed source ELFs");
        return;
    }

    let (transfer, snapshot, layout) = reference_handoff_snapshot_and_layout();
    let mut allocator = MockPageAllocator::new(0x0000_0000_2400_0000);
    let result = boot_from_transfer_and_init_gate_d_datapath_guest_throughput(
        &transfer,
        &snapshot,
        &layout,
        &mut allocator,
    )
    .expect("guest throughput");
    assert_datapath_guest_throughput_validate_only(&result);
}

#[test]
fn boot_from_transfer_and_init_gate_d_datapath_guest_throughput_from_snapshots_accepts_reference_transfer(
) {
    if !GUEST_SOURCE_ELFS_AVAILABLE {
        eprintln!("skipping: run cargo xtask build-guests to embed source ELFs");
        return;
    }

    let (transfer, snapshot, layout) = reference_handoff_snapshot_and_layout();
    let layout_snapshot = layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
    let mut allocator = MockPageAllocator::new(0x0000_0000_2500_0000);
    let result = boot_from_transfer_and_init_gate_d_datapath_guest_throughput_from_snapshots(
        &transfer,
        &snapshot,
        &layout_snapshot,
        &mut allocator,
    )
    .expect("guest throughput snapshots");
    assert_datapath_guest_throughput_validate_only(&result);
}
