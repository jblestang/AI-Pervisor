//! Firmware physical memory access.

use crate::error::{AcpiWalkError, AcpiWalkErrorKind};

/// Reads bytes from firmware physical addresses.
pub trait PhysicalMemory {
    /// Reads `buffer.len()` bytes starting at `physical_address`.
    fn read_physical(&self, physical_address: u64, buffer: &mut [u8]) -> Result<(), AcpiWalkError>;
}

/// Contiguous firmware memory image used by host-side loader tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareMemoryImage {
    /// Physical address mapped to `bytes[0]`.
    pub base_address: u64,
    /// Backing bytes for the mapped range.
    pub bytes: alloc::vec::Vec<u8>,
}

impl FirmwareMemoryImage {
    /// Creates a new firmware memory image.
    pub fn new(base_address: u64, bytes: alloc::vec::Vec<u8>) -> Self {
        Self {
            base_address,
            bytes,
        }
    }
}

impl PhysicalMemory for FirmwareMemoryImage {
    fn read_physical(&self, physical_address: u64, buffer: &mut [u8]) -> Result<(), AcpiWalkError> {
        let offset = physical_address.checked_sub(self.base_address).ok_or(
            AcpiWalkError::new(AcpiWalkErrorKind::Memory, "physical address below image base"),
        )?;
        let end = offset
            .checked_add(buffer.len() as u64)
            .ok_or(AcpiWalkError::new(
                AcpiWalkErrorKind::Bounds,
                "physical read overflow",
            ))?;
        if end > self.base_address.checked_add(self.bytes.len() as u64).ok_or(
            AcpiWalkError::new(AcpiWalkErrorKind::Bounds, "image length overflow"),
        )? {
            return Err(AcpiWalkError::new(
                AcpiWalkErrorKind::Memory,
                "physical read exceeds firmware image",
            ));
        }
        let start = offset as usize;
        let end = start
            .checked_add(buffer.len())
            .ok_or(AcpiWalkError::new(
                AcpiWalkErrorKind::Bounds,
                "physical read slice overflow",
            ))?;
        let slice = self.bytes.get(start..end).ok_or(AcpiWalkError::new(
            AcpiWalkErrorKind::Memory,
            "physical read slice unavailable",
        ))?;
        for (index, byte) in buffer.iter_mut().enumerate() {
            let value = slice
                .get(index)
                .ok_or(AcpiWalkError::new(
                    AcpiWalkErrorKind::Memory,
                    "physical read byte missing",
                ))?;
            *byte = *value;
        }
        Ok(())
    }
}
