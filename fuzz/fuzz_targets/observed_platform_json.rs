#![no_main]

use hv_platform_model::parse_observed_platform_json;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(contents) = std::str::from_utf8(data) else {
        return;
    };

    let _ = parse_observed_platform_json(contents);
});
