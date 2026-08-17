#![no_main]

use hv_config_model::parse_bdf;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let _ = parse_bdf(input);
});
