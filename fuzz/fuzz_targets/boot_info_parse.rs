#![no_main]

use hv_boot_abi::{descriptor_kind, validate_rsdp_section, BootInfoView};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(view) = BootInfoView::parse(data) else {
        return;
    };

    let _ = view.bounded_bytes();
    for index in 0..view.descriptor_count() {
        let Ok(descriptor) = view.descriptor(index) else {
            continue;
        };
        let _ = view.section(&descriptor);
        if descriptor.kind == descriptor_kind::RSDP {
            if let Ok(section) = view.section(&descriptor) {
                let _ = validate_rsdp_section(section);
            }
        }
    }

    let _ = view.find_descriptor(descriptor_kind::MEMORY_MAP);
    let _ = view.find_descriptor(descriptor_kind::RSDP);
    let _ = view.find_descriptor(descriptor_kind::CONFIG);
    let _ = view.verify_config_digest(&[0u8; 32]);
});
