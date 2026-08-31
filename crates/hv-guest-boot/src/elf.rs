//! Minimal ELF64 parsing for reference guest images.

use alloc::vec::Vec;

/// Category of guest ELF parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestElfErrorKind {
    /// Image bytes were invalid or truncated.
    InvalidInput,
    /// ELF header or program header constraints were violated.
    FormatViolation,
}

/// Structured guest ELF parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestElfError {
    /// Error category.
    pub kind: GuestElfErrorKind,
    /// Human-readable message.
    pub message: alloc::string::String,
}

impl GuestElfError {
    /// Creates a new guest ELF parse error.
    pub fn new(kind: GuestElfErrorKind, message: impl Into<alloc::string::String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// One PT_LOAD segment extracted from a guest ELF image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestElfLoadSegment {
    /// File offset of segment contents.
    pub file_offset: u64,
    /// Virtual address where the segment is linked.
    pub vaddr: u64,
    /// Bytes copied from the image for this segment.
    pub bytes: Vec<u8>,
}

/// Parsed guest ELF64 executable used for resident install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestElfImage {
    /// ELF entry virtual address.
    pub entry_vaddr: u64,
    /// Load segments to copy into guest physical memory.
    pub load_segments: Vec<GuestElfLoadSegment>,
}

const ELF_MAGIC: [u8; 4] = *b"\x7fELF";
const ELFCLASS64: u8 = 2;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 0x003E;
const PT_LOAD: u32 = 1;
const ELF64_HEADER_SIZE: usize = 64;
const ELF64_PHENT_SIZE: usize = 56;

/// Parses a minimal ELF64 ET_EXEC image containing PT_LOAD segments.
pub fn parse_elf64(bytes: &[u8]) -> Result<GuestElfImage, GuestElfError> {
    if bytes.len() < ELF64_HEADER_SIZE {
        return Err(GuestElfError::new(
            GuestElfErrorKind::InvalidInput,
            "elf image smaller than header",
        ));
    }
    if bytes.get(0..4) != Some(&ELF_MAGIC) {
        return Err(GuestElfError::new(
            GuestElfErrorKind::FormatViolation,
            "elf magic mismatch",
        ));
    }
    if *bytes.get(4).ok_or_else(invalid_input)? != ELFCLASS64 {
        return Err(GuestElfError::new(
            GuestElfErrorKind::FormatViolation,
            "elf class must be 64-bit",
        ));
    }
    let elf_type = read_u16(bytes, 0x10)?;
    if elf_type != ET_EXEC {
        return Err(GuestElfError::new(
            GuestElfErrorKind::FormatViolation,
            "elf type must be ET_EXEC",
        ));
    }
    let machine = read_u16(bytes, 0x12)?;
    if machine != EM_X86_64 {
        return Err(GuestElfError::new(
            GuestElfErrorKind::FormatViolation,
            "elf machine must be x86-64",
        ));
    }
    let entry_vaddr = read_u64(bytes, 0x18)?;
    let phoff = read_u64(bytes, 0x20)? as usize;
    let phentsize = read_u16(bytes, 0x36)? as usize;
    let phnum = read_u16(bytes, 0x38)? as usize;
    if phentsize != ELF64_PHENT_SIZE {
        return Err(GuestElfError::new(
            GuestElfErrorKind::FormatViolation,
            "unexpected elf program header size",
        ));
    }
    let mut load_segments = Vec::new();
    for index in 0..phnum {
        let base = phoff
            .checked_add(index.checked_mul(phentsize).ok_or_else(overflow)?)
            .ok_or_else(overflow)?;
        let p_type = read_u32(bytes, base)?;
        if p_type != PT_LOAD {
            continue;
        }
        let p_offset = read_u64(bytes, base + 0x08)?;
        let p_vaddr = read_u64(bytes, base + 0x10)?;
        let p_filesz = read_u64(bytes, base + 0x20)?;
        let end = p_offset.checked_add(p_filesz).ok_or_else(overflow)? as usize;
        if end > bytes.len() {
            return Err(GuestElfError::new(
                GuestElfErrorKind::FormatViolation,
                "elf PT_LOAD segment extends past image end",
            ));
        }
        let start = p_offset as usize;
        let segment_bytes = bytes.get(start..end).ok_or_else(invalid_input)?.to_vec();
        load_segments.push(GuestElfLoadSegment {
            file_offset: p_offset,
            vaddr: p_vaddr,
            bytes: segment_bytes,
        });
    }
    if load_segments.is_empty() {
        return Err(GuestElfError::new(
            GuestElfErrorKind::FormatViolation,
            "elf image has no PT_LOAD segments",
        ));
    }
    Ok(GuestElfImage {
        entry_vaddr,
        load_segments,
    })
}

/// Returns the guest physical entry address after loading at `region_base`.
pub fn guest_entry_phys_for_region(
    image: &GuestElfImage,
    region_base: u64,
) -> Result<u64, GuestElfError> {
    region_base.checked_add(image.entry_vaddr).ok_or_else(|| {
        GuestElfError::new(GuestElfErrorKind::FormatViolation, "guest entry overflow")
    })
}

fn invalid_input() -> GuestElfError {
    GuestElfError::new(GuestElfErrorKind::InvalidInput, "elf read out of bounds")
}

fn overflow() -> GuestElfError {
    GuestElfError::new(GuestElfErrorKind::FormatViolation, "elf offset overflow")
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, GuestElfError> {
    let slice = bytes.get(offset..offset + 2).ok_or_else(invalid_input)?;
    Ok(u16::from_le_bytes([
        slice.first().copied().unwrap_or(0),
        slice.get(1).copied().unwrap_or(0),
    ]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, GuestElfError> {
    let slice = bytes.get(offset..offset + 4).ok_or_else(invalid_input)?;
    Ok(u32::from_le_bytes([
        slice.first().copied().unwrap_or(0),
        slice.get(1).copied().unwrap_or(0),
        slice.get(2).copied().unwrap_or(0),
        slice.get(3).copied().unwrap_or(0),
    ]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, GuestElfError> {
    let slice = bytes.get(offset..offset + 8).ok_or_else(invalid_input)?;
    Ok(u64::from_le_bytes([
        slice.first().copied().unwrap_or(0),
        slice.get(1).copied().unwrap_or(0),
        slice.get(2).copied().unwrap_or(0),
        slice.get(3).copied().unwrap_or(0),
        slice.get(4).copied().unwrap_or(0),
        slice.get(5).copied().unwrap_or(0),
        slice.get(6).copied().unwrap_or(0),
        slice.get(7).copied().unwrap_or(0),
    ]))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/partition_elfs.rs"));

    #[test]
    fn parse_reference_in_elf() {
        let image = parse_elf64(GUEST_IN_ELF).expect("parse");
        assert_eq!(image.entry_vaddr, 0);
        assert_eq!(image.load_segments.len(), 1);
        assert_eq!(
            image.load_segments.first().expect("segment").bytes.len(),
            24
        );
    }

    #[test]
    fn guest_entry_phys_matches_region_base_for_zero_linked_entry() {
        let image = parse_elf64(GUEST_MID_ELF).expect("parse");
        assert_eq!(
            guest_entry_phys_for_region(&image, 0x1000).expect("entry"),
            0x1000
        );
    }
}
