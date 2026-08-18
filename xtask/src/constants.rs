//! xtask-specific constants.

/// Default minimum required line coverage percentage.
pub const DEFAULT_COVERAGE_MIN_LINES: u8 = 95;

/// Default libFuzzer iteration count for smoke runs.
pub const DEFAULT_FUZZ_RUNS: u32 = 512;

/// Default configuration path for EFI loader builds.
pub const DEFAULT_EFI_CONFIG_PATH: &str = "configs/qemu.yaml";

/// Configuration path for OVMF smoke boot (TCG-friendly requirements).
pub const DEFAULT_OVMF_SMOKE_CONFIG_PATH: &str = "configs/ovmf-smoke.yaml";

/// Default output path for the built UEFI loader image.
pub const DEFAULT_EFI_OUTPUT_PATH: &str = "build/hv-loader.efi";

/// Default output path for the built UEFI hypervisor image.
pub const DEFAULT_HYPERVISOR_EFI_OUTPUT_PATH: &str = "build/hv-hypervisor.efi";

/// Default output directory for boot-chain images.
pub const DEFAULT_BOOT_CHAIN_OUTPUT_DIR: &str = "build/boot-chain";

/// Default OVMF/QEMU smoke boot timeout in seconds.
pub const DEFAULT_OVMF_SMOKE_TIMEOUT_SECS: u64 = 60;

/// Working directory for OVMF smoke boot artifacts (ESP, vars copy, serial log).
pub const OVMF_SMOKE_WORK_DIR: &str = "build/ovmf-smoke";

/// Working directory for KVM live QEMU smoke boot artifacts.
pub const LIVE_QEMU_SMOKE_WORK_DIR: &str = "build/live-qemu-smoke";

/// Default output directory for REAL_HW boot-chain images.
pub const DEFAULT_LIVE_BOOT_CHAIN_OUTPUT_DIR: &str = "build/live-boot-chain";

/// Default live QEMU smoke boot timeout in seconds.
pub const DEFAULT_LIVE_QEMU_SMOKE_TIMEOUT_SECS: u64 = 90;

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
    "transfer_parse",
];
