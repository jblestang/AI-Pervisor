//! Coverage-oriented hypervisor boot and VMX orchestration tests.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_config_model::{compile_config_from_str, PlatformRequirements};
use hv_hypervisor_boot::{
    boot_check_and_init_gate_c, boot_check_and_init_vmx, boot_from_transfer_and_init_gate_c,
    boot_from_transfer_and_init_vmx, boot_from_transfer_snapshot,
    requirements_snapshot_from_platform, BootCheckErrorKind,
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
use hv_vmx::{init_vmx, FailingVmxBackend, MockVmxBackend, VMXON_REGION_MIN_BYTES};

fn reference_handoff_snapshot_and_layout() -> (
    Vec<u8>,
    hv_boot_abi::RequirementsSnapshot,
    PlatformRequirements,
    hv_platform_model::StaticPlatformIR,
) {
    let (transfer, snapshot, requirements) = reference_handoff_and_snapshot();
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    (transfer, snapshot, requirements, layout)
}

#[test]
fn boot_from_transfer_and_init_gate_c_programming_accepts_reference_transfer() {
    let (transfer, snapshot, _, layout) = reference_handoff_snapshot_and_layout();
    let result = hv_hypervisor_boot::boot_from_transfer_and_init_gate_c_programming(
        &transfer, &snapshot, &layout,
    )
    .expect("gate c programming");
    assert!(result.init.validated.observed.vmx);
    assert!(result.vmxon_region.is_some());
    assert!(result.ept_tables.is_some());
    assert!(result.vtd_tables.is_some());
}

#[test]
fn boot_from_transfer_and_init_gate_c_programming_from_snapshots_accepts_reference_transfer() {
    let (transfer, snapshot, _, layout) = reference_handoff_snapshot_and_layout();
    let layout_snapshot =
        hv_hypervisor_boot::layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
    let result = hv_hypervisor_boot::boot_from_transfer_and_init_gate_c_programming_from_snapshots(
        &transfer,
        &snapshot,
        &layout_snapshot,
    )
    .expect("gate c programming snapshots");
    assert!(result.init.validated.observed.vtd);
    assert!(result.vtd_tables.is_some());
}

#[test]
fn boot_from_transfer_and_init_gate_c_from_snapshots_accepts_reference_transfer() {
    let (transfer, snapshot, _, layout) = reference_handoff_snapshot_and_layout();
    let layout_snapshot =
        hv_hypervisor_boot::layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
    let result = hv_hypervisor_boot::boot_from_transfer_and_init_gate_c_from_snapshots(
        &transfer,
        &snapshot,
        &layout_snapshot,
    )
    .expect("gate c snapshots");
    assert!(result.validated.observed.vmx);
    assert!(!result.ept_plan.identity_mappings.is_empty());
}

#[test]
fn boot_check_and_init_gate_c_programming_accepts_reference_inputs() {
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
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let result = hv_hypervisor_boot::boot_check_and_init_gate_c_programming(
        &handoff.boot_info_blob,
        &compiled.digest.bytes,
        &compiled.requirements,
        &handoff.observation,
        &layout,
    )
    .expect("gate c programming");
    assert!(result.init.validated.observed.vmx);
    assert!(result.vmxon_region.is_some());
}

#[test]
fn boot_from_transfer_and_init_gate_c_accepts_reference_transfer() {
    let (transfer, snapshot, _, layout) = reference_handoff_snapshot_and_layout();
    let result = boot_from_transfer_and_init_gate_c(&transfer, &snapshot, &layout).expect("gate c");
    assert!(result.validated.observed.vmx);
    assert!(!result.ept_plan.identity_mappings.is_empty());
    assert_eq!(
        result.vtd_plan.device_assignments.len(),
        layout.pci_devices.len()
    );
}

#[test]
fn boot_check_and_init_gate_c_accepts_reference_inputs() {
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
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let result = boot_check_and_init_gate_c(
        &handoff.boot_info_blob,
        &compiled.digest.bytes,
        &compiled.requirements,
        &handoff.observation,
        &layout,
    )
    .expect("gate c");
    assert!(result.validated.observed.interrupt_remapping);
}

#[test]
fn boot_from_transfer_and_init_gate_c_rejects_ept_planning_failure() {
    let (transfer, snapshot, _, mut layout) = reference_handoff_snapshot_and_layout();
    layout.hypervisor_reserve.size = ByteSize::new(VMXON_REGION_MIN_BYTES);
    let err =
        boot_from_transfer_and_init_gate_c(&transfer, &snapshot, &layout).expect_err("must fail");
    assert_eq!(err.kind, BootCheckErrorKind::Platform);
}

#[test]
fn boot_from_transfer_and_init_gate_c_skips_optional_ept_and_vtd() {
    let (transfer, mut snapshot, _, layout) = reference_handoff_snapshot_and_layout();
    snapshot.ept = hv_boot_abi::FEATURE_OPTIONAL;
    snapshot.vtd = hv_boot_abi::FEATURE_OPTIONAL;
    let result = boot_from_transfer_and_init_gate_c(&transfer, &snapshot, &layout).expect("gate c");
    assert!(result.validated.observed.ept);
    assert!(result.validated.observed.vtd);
}

fn reference_handoff_and_snapshot() -> (
    Vec<u8>,
    hv_boot_abi::RequirementsSnapshot,
    PlatformRequirements,
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
    (transfer, snapshot, compiled.requirements)
}

#[test]
fn boot_check_and_init_vmx_accepts_reference_inputs() {
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
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let result = boot_check_and_init_vmx(
        &handoff.boot_info_blob,
        &compiled.digest.bytes,
        &compiled.requirements,
        &handoff.observation,
        layout.hypervisor_reserve.host_phys.raw(),
        layout.hypervisor_reserve.size.bytes(),
    )
    .expect("boot+vmx");
    assert!(result.validated.observed.vmx);
}

#[test]
fn boot_from_transfer_and_init_vmx_rejects_invalid_reserve_in_snapshot() {
    let (transfer, mut snapshot, _) = reference_handoff_and_snapshot();
    snapshot.hypervisor_reserve_bytes = 4095;
    let err = boot_from_transfer_and_init_vmx(&transfer, &snapshot).expect_err("must fail");
    assert_eq!(err.kind, BootCheckErrorKind::Platform);
}

#[test]
fn boot_from_transfer_and_init_vmx_skips_backend_when_vmx_optional() {
    let (transfer, mut snapshot, _) = reference_handoff_and_snapshot();
    snapshot.vmx = hv_boot_abi::FEATURE_OPTIONAL;
    let result = boot_from_transfer_and_init_vmx(&transfer, &snapshot).expect("boot");
    assert!(result.validated.observed.vmx);
}

#[test]
fn boot_from_transfer_and_init_vmx_invokes_backend_for_required_vmx() {
    let (transfer, snapshot, _) = reference_handoff_and_snapshot();
    let result = boot_from_transfer_and_init_vmx(&transfer, &snapshot).expect("boot+vmx");
    assert!(result.validated.observed.vmx);
    let mut backend = MockVmxBackend::default();
    init_vmx(&mut backend, &result.vmx_plan, &result.validated).expect("init");
    assert_eq!(backend.enable_calls, 1);
}

#[test]
fn boot_from_transfer_and_init_vmx_maps_backend_failure() {
    let (transfer, snapshot, _) = reference_handoff_and_snapshot();
    let result = boot_from_transfer_and_init_vmx(&transfer, &snapshot).expect("boot+vmx");
    let mut backend = FailingVmxBackend;
    assert!(init_vmx(&mut backend, &result.vmx_plan, &result.validated).is_err());
}

#[test]
fn platform_requirements_from_snapshot_rejects_invalid_arch() {
    let (_, mut snapshot, _) = reference_handoff_and_snapshot();
    snapshot.arch = 99;
    assert!(hv_hypervisor_boot::platform_requirements_from_snapshot(&snapshot).is_err());
}

#[test]
fn platform_requirements_from_snapshot_rejects_oversized_page_and_pci_counts() {
    let (_, mut snapshot, _) = reference_handoff_and_snapshot();
    snapshot.page_size_count = hv_boot_abi::MAX_REQUIREMENTS_PAGE_SIZES as u32 + 1;
    assert!(hv_hypervisor_boot::platform_requirements_from_snapshot(&snapshot).is_err());
    snapshot.page_size_count = 1;
    snapshot.expected_pci_count = hv_boot_abi::MAX_REQUIREMENTS_PCI_DEVICES as u32 + 1;
    assert!(hv_hypervisor_boot::platform_requirements_from_snapshot(&snapshot).is_err());
}

#[test]
fn requirements_snapshot_from_platform_rejects_oversized_inputs() {
    let (_, _, requirements) = reference_handoff_and_snapshot();
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let mut oversized = requirements.clone();
    oversized.page_sizes.sizes = vec![4096; hv_boot_abi::MAX_REQUIREMENTS_PAGE_SIZES + 1];
    assert!(requirements_snapshot_from_platform(
        &oversized,
        compiled.digest.bytes,
        layout.hypervisor_reserve.host_phys.raw(),
        layout.hypervisor_reserve.size.bytes(),
    )
    .is_err());
}

#[test]
fn requirements_snapshot_from_platform_rejects_oversized_pci_list() {
    let (_, _, requirements) = reference_handoff_and_snapshot();
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
    let device = requirements
        .expected_pci_devices
        .first()
        .expect("device")
        .clone();
    let mut oversized = requirements.clone();
    oversized.expected_pci_devices = (0..=hv_boot_abi::MAX_REQUIREMENTS_PCI_DEVICES)
        .map(|_| device.clone())
        .collect();
    assert!(requirements_snapshot_from_platform(
        &oversized,
        compiled.digest.bytes,
        layout.hypervisor_reserve.host_phys.raw(),
        layout.hypervisor_reserve.size.bytes(),
    )
    .is_err());
}

#[test]
fn platform_requirements_from_snapshot_rejects_invalid_feature_and_smt_fields() {
    let (_, mut snapshot, _) = reference_handoff_and_snapshot();
    snapshot.vmx = 99;
    assert!(hv_hypervisor_boot::platform_requirements_from_snapshot(&snapshot).is_err());
    snapshot.vmx = hv_boot_abi::FEATURE_REQUIRED;
    snapshot.smt_policy = 99;
    assert!(hv_hypervisor_boot::platform_requirements_from_snapshot(&snapshot).is_err());
}

#[test]
fn boot_from_transfer_snapshot_rejects_invalid_snapshot() {
    let (transfer, mut snapshot, _) = reference_handoff_and_snapshot();
    snapshot.arch = 99;
    assert!(boot_from_transfer_snapshot(&transfer, &snapshot).is_err());
}

#[test]
fn boot_from_transfer_rejects_invalid_transfer_blob() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let err = hv_hypervisor_boot::boot_from_transfer(
        &[0xAA; 32],
        &compiled.digest.bytes,
        &compiled.requirements,
    )
    .expect_err("must fail");
    assert_eq!(err.kind, BootCheckErrorKind::BootAbi);
}
