//! Boot info blob parsing and validation helpers.

use core::mem::size_of;

use hv_types::SHA256_DIGEST_BYTES;

use crate::constants::RSDP_SIGNATURE;
use crate::acpi::AcpiRsdp;
use crate::descriptor_kind;
use crate::error::{BootError, BootErrorKind};
use crate::{boot_abi_is_compatible, BootInfoDescriptor, BootInfoHeader};

/// Borrowed view over a boot info blob.
#[derive(Debug, Clone, Copy)]
pub struct BootInfoView<'a> {
    bytes: &'a [u8],
    header: BootInfoHeader,
}

impl<'a> BootInfoView<'a> {
    /// Parses and validates the boot info header.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, BootError> {
        if bytes.len() < size_of::<BootInfoHeader>() {
            return Err(BootError::new(
                BootErrorKind::Parse,
                "boot info shorter than header",
            ));
        }

        let header = read_header(bytes)?;
        if !boot_abi_is_compatible(&header) {
            return Err(BootError::new(
                BootErrorKind::Incompatible,
                "unsupported boot info header",
            ));
        }
        if (header.size as usize) > bytes.len() {
            return Err(BootError::new(
                BootErrorKind::Bounds,
                "declared boot info size exceeds buffer",
            ));
        }
        if (header.size as usize) < size_of::<BootInfoHeader>() {
            return Err(BootError::new(
                BootErrorKind::Bounds,
                "declared boot info size smaller than header",
            ));
        }

        let view = Self { bytes, header };
        view.validate_layout()?;
        Ok(view)
    }

    /// Returns the parsed header.
    pub const fn header(&self) -> &BootInfoHeader {
        &self.header
    }

    /// Returns the full boot info slice bounded by the declared size.
    pub fn bounded_bytes(&self) -> Result<&'a [u8], BootError> {
        self.bytes
            .get(0..self.header.size as usize)
            .ok_or(BootError::new(
                BootErrorKind::Bounds,
                "boot info bounded slice unavailable",
            ))
    }

    /// Returns the number of descriptors in the table.
    pub const fn descriptor_count(&self) -> u32 {
        self.header.descriptor_count
    }

    /// Reads one descriptor by index.
    pub fn descriptor(&self, index: u32) -> Result<BootInfoDescriptor, BootError> {
        let bounded = self.bounded_bytes()?;
        let table_offset = self.header.descriptor_table_offset as usize;
        let entry_offset = table_offset
            .checked_add(size_of::<BootInfoDescriptor>() * index as usize)
            .ok_or(BootError::new(
                BootErrorKind::Bounds,
                "descriptor index overflow",
            ))?;
        let entry_end = entry_offset
            .checked_add(size_of::<BootInfoDescriptor>())
            .ok_or(BootError::new(
                BootErrorKind::Bounds,
                "descriptor entry overflow",
            ))?;
        let entry_bytes = bounded.get(entry_offset..entry_end).ok_or(
            BootError::new(BootErrorKind::Bounds, "descriptor entry out of bounds"),
        )?;
        read_descriptor(entry_bytes)
    }

    /// Returns the payload bytes for a descriptor.
    pub fn section(&self, descriptor: &BootInfoDescriptor) -> Result<&'a [u8], BootError> {
        let bounded = self.bounded_bytes()?;
        let start = descriptor.offset as usize;
        let end = start
            .checked_add(descriptor.size as usize)
            .ok_or(BootError::new(
                BootErrorKind::Bounds,
                "descriptor section overflow",
            ))?;
        bounded
            .get(start..end)
            .ok_or(BootError::new(
                BootErrorKind::Bounds,
                "descriptor section out of bounds",
            ))
    }

    /// Finds the first descriptor of the requested kind.
    pub fn find_descriptor(&self, kind: u32) -> Result<Option<BootInfoDescriptor>, BootError> {
        for index in 0..self.header.descriptor_count {
            let descriptor = self.descriptor(index)?;
            if descriptor.kind == kind {
                return Ok(Some(descriptor));
            }
        }
        Ok(None)
    }

    /// Verifies the header configuration digest against the expected value.
    pub fn verify_config_digest(
        &self,
        expected: &[u8; SHA256_DIGEST_BYTES],
    ) -> Result<(), BootError> {
        if self.header.config_digest == *expected {
            Ok(())
        } else {
            Err(BootError::new(
                BootErrorKind::DigestMismatch,
                "config digest mismatch",
            ))
        }
    }

    /// Returns the memory-map section if present.
    pub fn memory_map_section(&self) -> Result<Option<&'a [u8]>, BootError> {
        match self.find_descriptor(descriptor_kind::MEMORY_MAP)? {
            Some(descriptor) => Ok(Some(self.section(&descriptor)?)),
            None => Ok(None),
        }
    }

    /// Returns the ACPI RSDP section if present.
    pub fn rsdp_section(&self) -> Result<Option<&'a [u8]>, BootError> {
        match self.find_descriptor(descriptor_kind::RSDP)? {
            Some(descriptor) => Ok(Some(self.section(&descriptor)?)),
            None => Ok(None),
        }
    }

    fn validate_layout(&self) -> Result<(), BootError> {
        let bounded = self.bounded_bytes()?;
        let declared_size = self.header.size as usize;
        let table_offset = self.header.descriptor_table_offset as usize;
        let table_bytes = (self.header.descriptor_count as usize)
            .checked_mul(size_of::<BootInfoDescriptor>())
            .ok_or(BootError::new(
                BootErrorKind::Bounds,
                "descriptor table size overflow",
            ))?;
        let table_end = table_offset.checked_add(table_bytes).ok_or(BootError::new(
            BootErrorKind::Bounds,
            "descriptor table end overflow",
        ))?;
        if table_end > declared_size {
            return Err(BootError::new(
                BootErrorKind::Bounds,
                "descriptor table exceeds declared boot info size",
            ));
        }
        if table_end > bounded.len() {
            return Err(BootError::new(
                BootErrorKind::Bounds,
                "descriptor table exceeds bounded boot info",
            ));
        }

        for index in 0..self.header.descriptor_count {
            let descriptor = self.descriptor(index)?;
            let section_end = (descriptor.offset as usize)
                .checked_add(descriptor.size as usize)
                .ok_or(BootError::new(
                    BootErrorKind::Bounds,
                    "descriptor section end overflow",
                ))?;
            if section_end > declared_size {
                return Err(BootError::new(
                    BootErrorKind::Bounds,
                    "descriptor section exceeds declared boot info size",
                ));
            }
        }
        Ok(())
    }
}

/// Validates an ACPI RSDP section signature and checksums.
pub fn validate_rsdp_section(section: &[u8]) -> Result<(), BootError> {
    if section.len() < RSDP_SIGNATURE.len() {
        return Err(BootError::new(
            BootErrorKind::Parse,
            "RSDP section truncated",
        ));
    }
    AcpiRsdp::parse(section).map(|_| ())
}

fn read_header(bytes: &[u8]) -> Result<BootInfoHeader, BootError> {
    let header_bytes = bytes
        .get(0..size_of::<BootInfoHeader>())
        .ok_or(BootError::new(
            BootErrorKind::Parse,
            "header bytes unavailable",
        ))?;
    let mut offset = 0usize;
    let mut magic = [0u8; 8];
    copy_field(&mut magic, header_bytes, &mut offset)?;
    Ok(BootInfoHeader {
        magic,
        version: read_u32(header_bytes, &mut offset)?,
        size: read_u32(header_bytes, &mut offset)?,
        config_digest: read_digest(header_bytes, &mut offset)?,
        descriptor_table_offset: read_u32(header_bytes, &mut offset)?,
        descriptor_count: read_u32(header_bytes, &mut offset)?,
    })
}

fn read_digest(bytes: &[u8], offset: &mut usize) -> Result<[u8; SHA256_DIGEST_BYTES], BootError> {
    let mut digest = [0u8; SHA256_DIGEST_BYTES];
    copy_field(&mut digest, bytes, offset)?;
    Ok(digest)
}

fn read_descriptor(bytes: &[u8]) -> Result<BootInfoDescriptor, BootError> {
    let mut offset = 0usize;
    Ok(BootInfoDescriptor {
        kind: read_u32(bytes, &mut offset)?,
        offset: read_u32(bytes, &mut offset)?,
        size: read_u32(bytes, &mut offset)?,
    })
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, BootError> {
    let end = offset
        .checked_add(size_of::<u32>())
        .ok_or(BootError::new(BootErrorKind::Parse, "u32 read overflow"))?;
    let slice = bytes
        .get(*offset..end)
        .ok_or(BootError::new(BootErrorKind::Parse, "u32 read out of bounds"))?;
    let chunk: [u8; 4] = slice.try_into().map_err(|_| {
        BootError::new(BootErrorKind::Parse, "u32 read truncated")
    })?;
    let value = u32::from_le_bytes(chunk);
    *offset = end;
    Ok(value)
}

fn copy_field(
    destination: &mut [u8],
    source: &[u8],
    offset: &mut usize,
) -> Result<(), BootError> {
    let end = offset
        .checked_add(destination.len())
        .ok_or(BootError::new(
            BootErrorKind::Parse,
            "field copy overflow",
        ))?;
    let slice = source
        .get(*offset..end)
        .ok_or(BootError::new(
            BootErrorKind::Parse,
            "field copy out of bounds",
        ))?;
    for (index, byte) in destination.iter_mut().enumerate() {
        let value = slice
            .get(index)
            .ok_or(BootError::new(BootErrorKind::Parse, "field byte missing"))?;
        *byte = *value;
    }
    *offset = end;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::{BOOT_ABI_VERSION, BOOT_INFO_MAGIC};

    #[test]
    fn rejects_digest_mismatch_on_header_only_blob() {
        let digest = [0xAA; SHA256_DIGEST_BYTES];
        let blob = encode_header_only_blob(digest);
        let view = BootInfoView::parse(&blob).expect("parse");
        let bad = [0xBB; SHA256_DIGEST_BYTES];
        let err = view.verify_config_digest(&bad).expect_err("must fail");
        assert_eq!(err.kind, BootErrorKind::DigestMismatch);
    }

    #[test]
    fn rejects_incompatible_magic() {
        let digest = [0u8; SHA256_DIGEST_BYTES];
        let mut blob = encode_header_only_blob(digest);
        if let Some(byte) = blob.get_mut(0) {
            *byte = b'X';
        }
        let err = BootInfoView::parse(&blob).expect_err("must fail");
        assert_eq!(err.kind, BootErrorKind::Incompatible);
    }

    fn encode_header_only_blob(digest: [u8; SHA256_DIGEST_BYTES]) -> [u8; 56] {
        let mut blob = [0u8; 56];
        blob[0..8].copy_from_slice(&BOOT_INFO_MAGIC);
        blob[8..12].copy_from_slice(&BOOT_ABI_VERSION.to_le_bytes());
        blob[12..16].copy_from_slice(&56u32.to_le_bytes());
        blob[16..48].copy_from_slice(&digest);
        blob[48..52].copy_from_slice(&56u32.to_le_bytes());
        blob[52..56].copy_from_slice(&0u32.to_le_bytes());
        blob
    }

    #[test]
    fn rejects_descriptor_table_beyond_declared_size() {
        let digest = [0u8; SHA256_DIGEST_BYTES];
        let mut blob = encode_header_only_blob(digest);
        blob[52..56].copy_from_slice(&1u32.to_le_bytes());
        let err = BootInfoView::parse(&blob).expect_err("must fail");
        assert_eq!(err.kind, BootErrorKind::Bounds);
    }
}
