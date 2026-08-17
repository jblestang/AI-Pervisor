//! UEFI memory map structures shared across the boot path.

use crate::constants::{
    UEFI_MEMORY_DESCRIPTOR_MIN_SIZE, UEFI_MEMORY_DESCRIPTOR_OVMF_SIZE,
};
use crate::error::{BootError, BootErrorKind};

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

impl UefiMemoryDescriptor {
    /// Parses the first semantic fields from a descriptor slice.
    pub fn parse(bytes: &[u8]) -> Result<Self, BootError> {
        if bytes.len() < UEFI_MEMORY_DESCRIPTOR_MIN_SIZE {
            return Err(BootError::new(
                BootErrorKind::Parse,
                "UEFI memory descriptor truncated",
            ));
        }

        let reserved = if bytes.len() >= UEFI_MEMORY_DESCRIPTOR_OVMF_SIZE {
            read_u64(bytes, 40)?
        } else {
            0
        };

        Ok(Self {
            typ: read_u32(bytes, 0)?,
            padding: read_u32(bytes, 4)?,
            physical_start: read_u64(bytes, 8)?,
            virtual_start: read_u64(bytes, 16)?,
            number_of_pages: read_u64(bytes, 24)?,
            attribute: read_u64(bytes, 32)?,
            reserved,
        })
    }
}

fn read_u32(bytes: &[u8], start: usize) -> Result<u32, BootError> {
    let slice = bytes
        .get(start..start + 4)
        .ok_or(BootError::new(
            BootErrorKind::Parse,
            "UEFI descriptor u32 truncated",
        ))?;
    let chunk: [u8; 4] = slice.try_into().map_err(|_| {
        BootError::new(BootErrorKind::Parse, "UEFI descriptor u32 truncated")
    })?;
    Ok(u32::from_le_bytes(chunk))
}

fn read_u64(bytes: &[u8], start: usize) -> Result<u64, BootError> {
    let slice = bytes
        .get(start..start + 8)
        .ok_or(BootError::new(
            BootErrorKind::Parse,
            "UEFI descriptor u64 truncated",
        ))?;
    let chunk: [u8; 8] = slice.try_into().map_err(|_| {
        BootError::new(BootErrorKind::Parse, "UEFI descriptor u64 truncated")
    })?;
    Ok(u64::from_le_bytes(chunk))
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

    #[test]
    fn parse_accepts_40_byte_descriptor() {
        let mut bytes = [0u8; 40];
        bytes[0..4].copy_from_slice(&7u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&1u64.to_le_bytes());
        let descriptor = UefiMemoryDescriptor::parse(&bytes).expect("parse");
        assert_eq!(descriptor.typ, 7);
        assert_eq!(descriptor.reserved, 0);
    }
}
