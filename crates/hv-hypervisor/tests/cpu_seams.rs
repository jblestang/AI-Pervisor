//! Host re-export coverage for Gate C CPU seam entry points.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::compile_config_from_str;
use hv_hypervisor::{
    boot_check_and_init_gate_c_cpu_seam, boot_from_transfer_and_init_gate_c_cpu_seam,
    boot_from_transfer_and_init_gate_c_cpu_seam_from_snapshots, layout_snapshot_from_platform_ir,
    requirements_snapshot_from_platform,
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
use hv_types::{ByteSize, PciBdf, PciBus, PciDevice, PciFunction, PciSegment};
use hv_vmx::VMXON_REGION_MIN_BYTES;

fn reference_handoff() -> (
    Vec<u8>,
    Vec<u8>,
    hv_config_model::PlatformRequirements,
    hv_platform_model::ObservationInputs,
    hv_platform_model::StaticPlatformIR,
    hv_boot_abi::RequirementsSnapshot,
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
    (
        transfer,
        handoff.boot_info_blob,
        compiled.requirements,
        handoff.observation,
        layout,
        snapshot,
    )
}

#[test]
fn hypervisor_reexport_cpu_seam_boot_path_runs() {
    let (transfer, _, _, _, layout, snapshot) = reference_handoff();
    let result = boot_from_transfer_and_init_gate_c_cpu_seam(&transfer, &snapshot, &layout)
        .expect("cpu seam reexport");
    assert!(result.programming.vmxon_region.is_some());
    assert!(result.vmx_seam.is_some());
}

#[test]
fn hypervisor_reexport_cpu_seam_snapshot_boot_path_runs() {
    let (transfer, _, _, _, layout, snapshot) = reference_handoff();
    let layout_snapshot = layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
    let result = boot_from_transfer_and_init_gate_c_cpu_seam_from_snapshots(
        &transfer,
        &snapshot,
        &layout_snapshot,
    )
    .expect("cpu seam snapshot reexport");
    assert!(result.programming.ept_tables.is_some());
}

#[test]
fn hypervisor_reexport_cpu_seam_boot_check_path_runs() {
    let (_, boot_info, requirements, observation, layout, _) = reference_handoff();
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let result = boot_check_and_init_gate_c_cpu_seam(
        &boot_info,
        &compiled.digest.bytes,
        &requirements,
        &observation,
        &layout,
    )
    .expect("cpu seam boot check reexport");
    assert!(result.programming.init.validated.observed.vtd);
}

#[test]
fn hypervisor_cpu_seam_rejects_ept_planning_failure() {
    let (transfer, _, _, _, mut layout, snapshot) = reference_handoff();
    layout.hypervisor_reserve.size = ByteSize::new(VMXON_REGION_MIN_BYTES);
    assert!(boot_from_transfer_and_init_gate_c_cpu_seam(&transfer, &snapshot, &layout).is_err());
}

#[test]
fn hypervisor_cpu_seam_skips_optional_backends() {
    let (transfer, _, _, _, layout, mut snapshot) = reference_handoff();
    snapshot.ept = hv_boot_abi::FEATURE_OPTIONAL;
    snapshot.vtd = hv_boot_abi::FEATURE_OPTIONAL;
    let result = boot_from_transfer_and_init_gate_c_cpu_seam(&transfer, &snapshot, &layout)
        .expect("cpu seam optional");
    assert!(result.programming.init.validated.observed.ept);
    assert!(result.ept_seam.is_none());
    assert!(result.vtd_seam.is_none());
}
