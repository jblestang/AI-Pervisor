//! Observed platform snapshot populated at boot from CPUID, ACPI, and firmware data.

use serde::{Deserialize, Serialize};

use hv_config_model::{ArchRequirement, SUPPORTED_ARCH};
use hv_types::{ByteSize, PciBdf};

/// Snapshot of platform capabilities observed at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedPlatform {
    /// Observed architecture string.
    pub arch: String,
    /// VMX support observed via CPUID.
    pub vmx: bool,
    /// EPT support observed via CPUID.
    pub ept: bool,
    /// VT-d / IOMMU support observed via firmware or CPUID.
    pub vtd: bool,
    /// Number of physical cores available to the hypervisor.
    pub physical_cores: u32,
    /// Total platform RAM visible to the hypervisor.
    pub ram_bytes: ByteSize,
    /// Whether simultaneous multithreading is enabled.
    pub smt_enabled: bool,
    /// Interrupt remapping support observed via ACPI/firmware.
    pub interrupt_remapping: bool,
    /// x2APIC support observed via CPUID.
    pub x2apic: bool,
    /// Invariant TSC support observed via CPUID.
    pub invariant_tsc: bool,
    /// VPID support observed via CPUID.
    pub vpid: bool,
    /// VMX preemption timer support observed via CPUID.
    pub vmx_preemption_timer: bool,
    /// NX support observed via CPUID.
    pub nx: bool,
    /// Supported page sizes in bytes, sorted ascending.
    pub page_sizes: Vec<u64>,
    /// PCI devices discovered on the platform.
    pub pci_devices: Vec<PciBdf>,
}

impl ObservedPlatform {
    /// Returns the observed architecture as a typed requirement when supported.
    pub fn arch_requirement(&self) -> Result<ArchRequirement, crate::error::PlatformError> {
        if self.arch == SUPPORTED_ARCH {
            Ok(ArchRequirement::X86_64)
        } else {
            Err(crate::error::PlatformError::new(
                crate::error::PlatformErrorKind::Validation,
                format!("unsupported observed arch '{}'", self.arch),
            ))
        }
    }
}

/// Parses an observed platform snapshot from JSON text.
pub fn parse_observed_platform_json(contents: &str) -> Result<ObservedPlatform, crate::error::PlatformError> {
    serde_json::from_str(contents).map_err(|err| {
        crate::error::PlatformError::new(
            crate::error::PlatformErrorKind::Parse,
            format!("failed to parse observed platform JSON: {err}"),
        )
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_reference_observed_fixture() {
        let json = include_str!("../tests/fixtures/observed/qemu_reference.json");
        let observed = parse_observed_platform_json(json).expect("parse");
        assert_eq!(observed.arch, SUPPORTED_ARCH);
        assert_eq!(observed.pci_devices.len(), 2);
        assert_eq!(observed.arch_requirement(), Ok(ArchRequirement::X86_64));
    }
}
