#![no_main]

use hv_config_model::{load_raw_from_str, validate_semantics, validate_syntax};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(contents) = std::str::from_utf8(data) else {
        return;
    };

    let Ok(raw) = load_raw_from_str(contents) else {
        return;
    };

    let _ = validate_syntax(&raw);
    let _ = validate_semantics(&raw);
});
