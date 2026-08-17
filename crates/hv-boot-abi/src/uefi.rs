//! UEFI memory map structures shared across the boot path.

/// UEFI memory map descriptor matching the firmware layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UefiMemoryDescriptor {
    /// Memory type identifier.
    pub typ: u32,
    /// Padding required by the UEFI layout.
    pub padding: u32,
    /// Physical start address.
    pub physical_start: u64,
    /// Virtual start address.
    pub virtual_start: u64,
    /// Number of 4 KiB pages.
    pub number_of_pages: u64,
    /// Memory attribute bitmask.
    pub attribute: u64,
    /// Reserved bytes matching the 48-byte OVMF descriptor size.
    pub reserved: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn uefi_memory_descriptor_layout_is_stable() {
        assert_eq!(size_of::<UefiMemoryDescriptor>(), 48);
        assert_eq!(align_of::<UefiMemoryDescriptor>(), 8);
    }
}
