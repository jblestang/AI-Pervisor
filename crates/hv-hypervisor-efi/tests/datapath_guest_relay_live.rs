//! Gate D datapath guest relay live hypervisor EFI entry tests.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::compile_config_from_str;
use hv_datapath::GUEST_RELAY_BENCHMARK_FRAMES;
use hv_hypervisor_efi::boot_hypervisor_from_transfer_datapath_guest_relay_live;
use hv_hypervisor_boot::{
    layout_snapshot_from_platform_ir, requirements_snapshot_from_platform,
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
use hv_x86_cpu::MockPageAllocator;

#[test]
fn boot_hypervisor_from_transfer_datapath_guest_relay_live_accepts_reference_handoff() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let requirements = requirements_snapshot_from_platform(
        &compiled.requirements,
        compiled.digest.bytes,
        layout.hypervisor_reserve.host_phys.raw(),
        layout.hypervisor_reserve.size.bytes(),
    )
    .expect("snapshot");
    let layout_snapshot = layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
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
    let mut allocator = MockPageAllocator::new(0x0000_0000_2900_0000);
    let markers = boot_hypervisor_from_transfer_datapath_guest_relay_live(
        &transfer,
        &compiled.digest.bytes,
        &requirements,
        &layout_snapshot,
        &mut allocator,
    )
    .expect("datapath guest relay live boot");
    assert_eq!(markers.execution.live.guest_boot_infos_installed, 3);
    assert!(markers.execution.live.sources.runtime.guest_datapath_frame_forwarded);
    assert!(markers.guest_throughput_target_met);
    assert!(markers.guest_throughput_min_mbit_per_sec >= 200);
    assert!(!markers.guest_throughput_executed);
    assert!(!markers.execution.guest_code_executed);
    assert_eq!(
        markers.sustained_relay_frames,
        u64::from(GUEST_RELAY_BENCHMARK_FRAMES)
    );
    assert_eq!(
        markers.execution.guest_code_executed,
        markers.execution.runtime_disposition_executed
    );
}
