#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = hv_boot_abi::HypervisorTransferView::parse(data);
    if let Ok(view) = hv_boot_abi::HypervisorTransferView::parse(data) {
        let _ = hv_boot_abi::decode_observation_transfer(view.observation());
    }
});
