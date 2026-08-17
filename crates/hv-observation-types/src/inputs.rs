//! Inputs required to observe a platform at boot time.

use alloc::vec::Vec;

use hv_types::PciBdf;

use crate::cpuid::CpuidSnapshot;

/// Inputs required to observe a platform at boot time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationInputs {
    /// CPUID snapshot collected by the loader.
    pub cpuid: CpuidSnapshot,
    /// Flattened ACPI table bytes collected by the loader.
    pub acpi_tables: Vec<u8>,
    /// Raw UEFI memory map bytes.
    pub memory_map: Vec<u8>,
    /// Size of one UEFI memory map descriptor.
    pub memory_descriptor_size: usize,
    /// PCI devices discovered by firmware.
    pub pci_devices: Vec<PciBdf>,
}
