//! Legacy PCI config space access for outer e1000 BAR discovery.

use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Reads PCI config dword at `offset` for `bdf`.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions", not(test)))]
pub fn pci_config_read32(bdf: PciBdf, offset: u8) -> Result<u32, CpuSeamError> {
    let address = (0x8000_0000u32
        | (u32::from(bdf.bus.raw()) << 16)
        | (u32::from(bdf.device.raw() & 0x1F) << 11)
        | (u32::from(bdf.function.raw() & 0x7) << 8))
        | u32::from(offset & 0xFC);
    // SAFETY: firmware/hypervisor runs with I/O privileges on x86_64 hosts.
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") 0xCF8u16,
            in("eax") address,
            options(nomem, nostack, preserves_flags)
        );
        let mut value: u32;
        core::arch::asm!(
            "in eax, dx",
            in("dx") 0xCFCu16,
            out("eax") value,
            options(nomem, nostack, preserves_flags)
        );
        Ok(value)
    }
}

/// Reads PCI config dword at `offset` for `bdf`.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions", not(test))))]
pub fn pci_config_read32(_bdf: PciBdf, _offset: u8) -> Result<u32, CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "PCI config access unavailable in this build",
    ))
}

/// Reads BAR0 MMIO base for an e1000-class device at `bdf`.
pub fn read_pci_bar0_mmio(bdf: PciBdf) -> Result<u64, CpuSeamError> {
    let vendor_device = pci_config_read32(bdf, 0x00)?;
    if (vendor_device & 0xFFFF) == 0xFFFF {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "PCI device not present at requested BDF",
        ));
    }
    let bar0_low = pci_config_read32(bdf, 0x10)?;
    if bar0_low & 0x1 != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "PCI BAR0 is I/O space, expected MMIO",
        ));
    }
    let mut bar = u64::from(bar0_low & !0xF);
    if bar0_low & 0x6 == 0x4 {
        let bar0_high = pci_config_read32(bdf, 0x14)?;
        bar |= u64::from(bar0_high) << 32;
    }
    Ok(bar)
}

/// Parses segment/bus/device/function from a `PciBdf`.
pub fn pci_bdf_from_parts(
    segment: u16,
    bus: u8,
    device: u8,
    function: u8,
) -> PciBdf {
    PciBdf::new(
        PciSegment::new(segment),
        PciBus::new(bus),
        PciDevice::new(device),
        PciFunction::new(function),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn pci_config_read32_unavailable_in_test_harness() {
        let bdf = pci_bdf_from_parts(0, 0, 3, 0);
        assert!(pci_config_read32(bdf, 0).is_err());
    }
}
