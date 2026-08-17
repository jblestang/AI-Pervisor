#![no_main]

use hv_boot_abi::UEFI_MEMORY_DESCRIPTOR_MIN_SIZE;
use hv_platform_model::{observe_platform, CpuidSnapshot, ObservationInputs};
use libfuzzer_sys::fuzz_target;

fn descriptor_size_from_fuzz(data: &[u8]) -> usize {
    let selector = data.first().copied().unwrap_or(48);
    let stride = 40 + (usize::from(selector) % 9) * 8;
    stride.max(UEFI_MEMORY_DESCRIPTOR_MIN_SIZE)
}

fn cpuid_from_fuzz(data: &[u8]) -> CpuidSnapshot {
    let read_u32 = |offset: usize| -> u32 {
        data.get(offset..offset + 4)
            .map(|chunk| {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(chunk);
                u32::from_le_bytes(bytes)
            })
            .unwrap_or(0)
    };

    CpuidSnapshot {
        leaf1_ecx: read_u32(4),
        leaf1_edx: read_u32(8),
        leaf1_ebx: read_u32(12),
        leaf80000007_edx: Some(read_u32(16)),
        leaf80000008_ecx: Some(read_u32(20)),
        leaf480_ecx: Some(read_u32(24)),
        leaf480_ebx: Some(read_u32(28)),
    }
}

fuzz_target!(|data: &[u8]| {
    let descriptor_size = descriptor_size_from_fuzz(data);
    let payload = data.get(1..).unwrap_or(&[]);

    let inputs = ObservationInputs {
        cpuid: cpuid_from_fuzz(data),
        acpi_tables: payload.to_vec(),
        memory_map: payload.to_vec(),
        memory_descriptor_size: descriptor_size,
        pci_devices: Vec::new(),
    };

    let _ = observe_platform(&inputs);
});
