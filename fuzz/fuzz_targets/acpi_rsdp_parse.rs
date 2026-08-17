#![no_main]

use hv_boot_abi::{validate_rsdp_section, AcpiRsdp};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = AcpiRsdp::parse(data);
    let _ = validate_rsdp_section(data);
});
