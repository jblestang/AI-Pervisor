#![no_main]

use hv_acpi_walk::{collect_acpi_tables, FirmwareMemoryImage};
use hv_boot_abi::AcpiRsdp;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 36 {
        return;
    }

    let rsdp_bytes = &data[0..36];
    let Ok(parsed) = AcpiRsdp::parse(rsdp_bytes) else {
        return;
    };

    let image_len = data.len().max(0x1000).min(0x200_000);
    let mut image = data.to_vec();
    if image.len() < image_len {
        image.resize(image_len, 0);
    } else {
        image.truncate(image_len);
    }

    let memory = FirmwareMemoryImage::new(0, image);
    let _ = collect_acpi_tables(&memory, &parsed);
});
