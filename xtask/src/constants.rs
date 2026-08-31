//! xtask-specific constants.

/// Cargo feature enabling REAL_HW hypervisor EFI builds.
pub const HYPERVISOR_EFI_REAL_HW_FEATURE: &str = "real-hw-execution";

/// Cargo feature enabling VMX launch on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_VMX_LAUNCH_FEATURE: &str = "vmx-launch";

/// Cargo feature enabling Gate D datapath foundation on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_FOUNDATION_FEATURE: &str = "datapath-foundation";

/// Cargo feature enabling Gate D datapath live on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_LIVE_FEATURE: &str = "datapath-live";

/// Cargo feature enabling Gate D datapath malicious on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_MALICIOUS_FEATURE: &str = "datapath-malicious";

/// Cargo feature enabling Gate D datapath guests on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_GUESTS_FEATURE: &str = "datapath-guests";

/// Cargo feature enabling Gate D datapath benchmark on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_BENCHMARK_FEATURE: &str = "datapath-benchmark";

/// Cargo feature enabling Gate D datapath runtime on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_RUNTIME_FEATURE: &str = "datapath-runtime";

/// Cargo feature enabling Gate D datapath guest sources on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_GUEST_SOURCES_FEATURE: &str = "datapath-guest-sources";

/// Cargo feature enabling Gate D datapath guest live on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_GUEST_LIVE_FEATURE: &str = "datapath-guest-live";

/// Cargo feature enabling Gate D datapath guest execution on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_GUEST_EXECUTION_FEATURE: &str = "datapath-guest-execution";

/// Cargo feature enabling Gate D datapath guest throughput on the REAL_HW hypervisor EFI path.
pub const HYPERVISOR_EFI_DATAPATH_GUEST_THROUGHPUT_FEATURE: &str = "datapath-guest-throughput";

/// OVMF serial marker emitted when BDS attempts to boot an application.
pub const OVMF_BOOT_ATTEMPT_MARKER: &str = "BdsDxe: starting Boot";

/// OVMF serial marker substring indicating a boot application failure.
pub const OVMF_BOOT_FAILURE_MARKER: &str = "failed to start Boot";

/// Guest RAM for smoke boots; must satisfy `configs/qemu.yaml` `min_ram_gib`.
pub const SMOKE_GUEST_MEMORY_MIB: &str = "8192";

/// SMP topology for smoke boots; must satisfy `configs/qemu.yaml` `min_physical_cores`.
pub const SMOKE_GUEST_SMP: &str = "4";

/// QEMU machine/accel for OVMF TCG smoke boot.
pub const OVMF_SMOKE_MACHINE: &str = "q35,accel=tcg";

/// QEMU machine/accel for KVM live REAL_HW smoke boot.
pub const LIVE_QEMU_MACHINE: &str = "q35,accel=kvm";

/// QEMU CPU model for KVM live REAL_HW smoke boot.
pub const LIVE_QEMU_CPU: &str = "host";

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
