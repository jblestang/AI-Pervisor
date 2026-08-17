#![no_main]

use hv_boot_abi::{UefiMemoryDescriptor, UEFI_MEMORY_DESCRIPTOR_MIN_SIZE};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() >= UEFI_MEMORY_DESCRIPTOR_MIN_SIZE {
        let _ = UefiMemoryDescriptor::parse(data);
    }

    for stride in [40usize, 48, 56, 64, 72, 80] {
        let mut offset = 0usize;
        while offset + UEFI_MEMORY_DESCRIPTOR_MIN_SIZE <= data.len() {
            if let Some(slice) = data.get(offset..offset + stride) {
                let _ = UefiMemoryDescriptor::parse(slice);
            }
            offset = offset.saturating_add(stride);
        }
    }
});
