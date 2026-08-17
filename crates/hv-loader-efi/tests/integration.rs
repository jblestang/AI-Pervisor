//! UEFI loader entry integration tests.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use hv_boot_abi::EFI_MEMORY_CONVENTIONAL;
use hv_config_model::compile_config_from_str;
use hv_loader::{encode_qemu_reference_firmware, FirmwareMemoryImage};
use hv_loader_efi::{uefi_loader_entry, UefiLoaderParams};
use hv_platform_model::{
    CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
    CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
    CPUID_80000007_EDX_INVARIANT_TSC_BIT,
};
use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

#[test]
fn uefi_loader_entry_builds_qemu_reference_handoff() {
    let yaml = include_str!("../../../configs/qemu.yaml");
    let compiled = compile_config_from_str(yaml).expect("compile");
    let firmware = encode_qemu_reference_firmware();
    let rsdp = rsdp_bytes_from_firmware(&firmware);

    let mut memory_map = vec![0u8; 48];
    memory_map[0..4].copy_from_slice(&EFI_MEMORY_CONVENTIONAL.to_le_bytes());
    memory_map[24..32].copy_from_slice(&(2_097_152u64).to_le_bytes());

    let handoff = uefi_loader_entry(UefiLoaderParams {
        config_digest: compiled.digest.bytes,
        memory_map,
        memory_descriptor_size: 48,
        rsdp,
        firmware_memory: firmware,
        cpuid: CpuidSnapshot {
            leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
            leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
            leaf1_ebx: (4 << 16) | 4,
            leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
            leaf80000008_ecx: Some(3),
            leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
            leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
        },
        pci_devices: vec![
            PciBdf {
                segment: PciSegment::new(0),
                bus: PciBus::new(0),
                device: PciDevice::new(3),
                function: PciFunction::new(0),
            },
            PciBdf {
                segment: PciSegment::new(0),
                bus: PciBus::new(0),
                device: PciDevice::new(4),
                function: PciFunction::new(0),
            },
        ],
    })
    .expect("entry");

    assert!(!handoff.observation.acpi_tables.is_empty());
}

fn rsdp_bytes_from_firmware(firmware: &FirmwareMemoryImage) -> Vec<u8> {
    firmware
        .bytes
        .get(0x1000..0x1000 + 36)
        .expect("rsdp")
        .to_vec()
}
