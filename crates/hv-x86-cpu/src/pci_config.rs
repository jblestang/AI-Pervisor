//! PCI config space access for outer e1000 BAR discovery.

use hv_types::PciBdf;

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Intel vendor id for e1000-class devices.
pub const E1000_PCI_VENDOR_ID: u16 = 0x8086;
/// Intel 82540EM device id used by QEMU e1000.
pub const E1000_PCI_DEVICE_ID: u16 = 0x100E;

/// Decodes BAR0 MMIO base from PCI config dwords at offsets 0x10 (and 0x14 for 64-bit).
pub fn decode_bar0_mmio_from_config_dwords(
    bar0_low: u32,
    bar0_high: u32,
) -> Result<u64, CpuSeamError> {
    if bar0_low & 0x1 != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "PCI BAR0 is I/O space, expected MMIO",
        ));
    }
    let mut bar = u64::from(bar0_low & !0xF);
    if bar0_low & 0x6 == 0x4 {
        bar |= u64::from(bar0_high) << 32;
    }
    Ok(bar)
}

/// Returns true when the PCI vendor/device dword matches QEMU e1000.
pub fn is_e1000_vendor_device(vendor_device: u32) -> bool {
    let vendor = vendor_device & 0xFFFF;
    let device = vendor_device >> 16;
    vendor == u32::from(E1000_PCI_VENDOR_ID) && device == u32::from(E1000_PCI_DEVICE_ID)
}

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
    if !is_e1000_vendor_device(vendor_device) {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "PCI device at requested BDF is not e1000-class",
        ));
    }
    let bar0_low = pci_config_read32(bdf, 0x10)?;
    let bar0_high = pci_config_read32(bdf, 0x14)?;
    decode_bar0_mmio_from_config_dwords(bar0_low, bar0_high)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_types::{PciBus, PciDevice, PciFunction, PciSegment};

    #[test]
    fn pci_config_read32_unavailable_in_test_harness() {
        let bdf = PciBdf::new(
            PciSegment::new(0),
            PciBus::new(0),
            PciDevice::new(3),
            PciFunction::new(0),
        );
        assert!(pci_config_read32(bdf, 0).is_err());
    }

    #[test]
    fn decode_32bit_mmio_bar0() {
        let bar = decode_bar0_mmio_from_config_dwords(0xFEB0_0000, 0).expect("bar");
        assert_eq!(bar, 0xFEB0_0000);
    }

    #[test]
    fn decode_64bit_mmio_bar0() {
        let bar = decode_bar0_mmio_from_config_dwords(0xFEB0_0004, 0).expect("bar");
        assert_eq!(bar, 0xFEB0_0000);
    }

    #[test]
    fn rejects_io_space_bar0() {
        assert!(decode_bar0_mmio_from_config_dwords(0x0000_0001, 0).is_err());
    }

    #[test]
    fn recognizes_e1000_vendor_device() {
        assert!(is_e1000_vendor_device(0x100E_8086));
        assert!(!is_e1000_vendor_device(0xFFFF_FFFF));
    }
}
