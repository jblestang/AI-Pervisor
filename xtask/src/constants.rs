//! xtask-specific constants.

/// Default minimum required line coverage percentage.
pub const DEFAULT_COVERAGE_MIN_LINES: u8 = 95;

/// Default libFuzzer iteration count for smoke runs.
pub const DEFAULT_FUZZ_RUNS: u32 = 512;

/// Default configuration path for EFI loader builds.
pub const DEFAULT_EFI_CONFIG_PATH: &str = "configs/qemu.yaml";

/// Default output path for the built UEFI loader image.
pub const DEFAULT_EFI_OUTPUT_PATH: &str = "build/hv-loader.efi";

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
