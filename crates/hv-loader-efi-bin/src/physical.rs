//! Identity-mapped physical memory reads for ACPI discovery under UEFI.

use core::ptr;

use hv_acpi_walk::{AcpiWalkError, AcpiWalkErrorKind, PhysicalMemory};

/// Maximum physical address readable during early boot (4 GiB).
pub const DEFAULT_MAX_PHYSICAL_ADDRESS: u64 = 0x1_0000_0000;

/// Reads firmware physical memory through the identity-mapped low region.
pub struct IdentityMappedPhysicalMemory {
    max_address: u64,
}

impl IdentityMappedPhysicalMemory {
    /// Creates a reader capped at `max_address`.
    pub const fn new(max_address: u64) -> Self {
        Self { max_address }
    }
}

impl PhysicalMemory for IdentityMappedPhysicalMemory {
    fn read_physical(&self, physical_address: u64, buffer: &mut [u8]) -> Result<(), AcpiWalkError> {
        let end = physical_address.checked_add(buffer.len() as u64).ok_or(
            AcpiWalkError::new(AcpiWalkErrorKind::Bounds, "physical read overflow"),
        )?;
        if end > self.max_address {
            return Err(AcpiWalkError::new(
                AcpiWalkErrorKind::Memory,
                "physical read exceeds mapped region",
            ));
        }

        let mut offset = 0usize;
        while offset < buffer.len() {
            let address = physical_address
                .checked_add(offset as u64)
                .ok_or(AcpiWalkError::new(
                    AcpiWalkErrorKind::Bounds,
                    "physical byte offset overflow",
                ))?;
            let byte = unsafe { ptr::read_volatile(address as *const u8) };
            if let Some(slot) = buffer.get_mut(offset) {
                *slot = byte;
            }
            offset = offset.saturating_add(1);
        }

        Ok(())
    }
}
