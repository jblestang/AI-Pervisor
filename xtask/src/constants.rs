//! xtask-specific constants.

/// Default minimum required line coverage percentage.
pub const DEFAULT_COVERAGE_MIN_LINES: u8 = 95;

/// Default libFuzzer iteration count for smoke runs.
pub const DEFAULT_FUZZ_RUNS: u32 = 512;

/// libFuzzer targets built from `fuzz/Cargo.toml`.
pub const FUZZ_TARGETS: &[&str] = &[
    "boot_info_parse",
    "acpi_rsdp_parse",
    "uefi_descriptor_parse",
    "acpi_walk",
    "config_yaml",
    "pci_bdf_parse",
    "observed_platform_json",
    "observe_platform",
];
