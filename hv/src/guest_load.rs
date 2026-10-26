//! Load guest images into layout-derived private regions (paths/sizes from config).

use crate::guest_mem::GpaRegion;

/// Errors while copying a guest image into a private GPA slot buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    ImageEmpty,
    ImageTooLarge,
    BadElf,
    Truncated,
}

/// Raw image bytes destination: host-owned buffer representing guest private RAM.
///
/// Nested bring-up (Task 7) maps this buffer at `region.base` in EPT. This helper
/// never chooses lab addresses — the caller supplies the layout region.
pub fn load_raw_image(dest: &mut [u8], region: GpaRegion, image: &[u8]) -> Result<u64, LoadError> {
    if image.is_empty() {
        return Err(LoadError::ImageEmpty);
    }
    if image.len() as u64 > region.size {
        return Err(LoadError::ImageTooLarge);
    }
    if dest.len() < image.len() {
        return Err(LoadError::Truncated);
    }
    dest[..image.len()].copy_from_slice(image);
    // Entry GPA = region base for flat binaries (ELF overrides via [`load_elf64`]).
    Ok(region.base)
}

/// Minimal ELF64 little-endian PT_LOAD loader (ET_EXEC / ET_DYN).
///
/// Returns the entry GPA. Segment file data is copied at `p_vaddr` relative to
/// `region.base` when `p_vaddr` falls inside the private window; absolute
/// `p_vaddr` equal to GPAs inside the region is also accepted.
pub fn load_elf64(dest: &mut [u8], region: GpaRegion, image: &[u8]) -> Result<u64, LoadError> {
    if image.len() < 64 {
        return Err(LoadError::BadElf);
    }
    if &image[0..4] != b"\x7fELF" {
        return Err(LoadError::BadElf);
    }
    if image[4] != 2 || image[5] != 1 {
        // EI_CLASS=ELFCLASS64, EI_DATA=ELFDATA2LSB
        return Err(LoadError::BadElf);
    }
    let entry = u64::from_le_bytes(image[24..32].try_into().unwrap());
    let phoff = u64::from_le_bytes(image[32..40].try_into().unwrap()) as usize;
    let phentsize = u16::from_le_bytes(image[54..56].try_into().unwrap()) as usize;
    let phnum = u16::from_le_bytes(image[56..58].try_into().unwrap()) as usize;
    if phentsize < 56 || phnum == 0 {
        return Err(LoadError::BadElf);
    }

    for i in 0..phnum {
        let off = phoff
            .checked_add(i.checked_mul(phentsize).ok_or(LoadError::BadElf)?)
            .ok_or(LoadError::BadElf)?;
        let ph = image.get(off..off + 56).ok_or(LoadError::Truncated)?;
        let p_type = u32::from_le_bytes(ph[0..4].try_into().unwrap());
        if p_type != 1 {
            // PT_LOAD
            continue;
        }
        let p_offset = u64::from_le_bytes(ph[8..16].try_into().unwrap()) as usize;
        let p_vaddr = u64::from_le_bytes(ph[16..24].try_into().unwrap());
        let p_filesz = u64::from_le_bytes(ph[32..40].try_into().unwrap()) as usize;
        let p_memsz = u64::from_le_bytes(ph[40..48].try_into().unwrap()) as usize;

        let dest_off = if p_vaddr >= region.base {
            (p_vaddr - region.base) as usize
        } else {
            p_vaddr as usize
        };
        let end = dest_off.checked_add(p_memsz).ok_or(LoadError::ImageTooLarge)?;
        if end > dest.len() || (end as u64) > region.size {
            return Err(LoadError::ImageTooLarge);
        }
        if p_filesz > 0 {
            let src_end = p_offset.checked_add(p_filesz).ok_or(LoadError::Truncated)?;
            let src = image.get(p_offset..src_end).ok_or(LoadError::Truncated)?;
            dest[dest_off..dest_off + p_filesz].copy_from_slice(src);
        }
        if p_memsz > p_filesz {
            dest[dest_off + p_filesz..end].fill(0);
        }
    }

    // If entry looks like an offset within the window, relocate; else use as GPA.
    let entry_gpa = if entry < region.size {
        region.base.checked_add(entry).ok_or(LoadError::BadElf)?
    } else if entry >= region.base
        && entry < region.base.checked_add(region.size).ok_or(LoadError::BadElf)?
    {
        entry
    } else {
        // Keep ELF e_entry as-is (linker scripts may use low identity GPAs).
        entry
    };
    Ok(entry_gpa)
}

/// Prefer ELF when magic matches; otherwise raw flat copy.
pub fn load_guest_image(dest: &mut [u8], region: GpaRegion, image: &[u8]) -> Result<u64, LoadError> {
    if image.len() >= 4 && &image[0..4] == b"\x7fELF" {
        load_elf64(dest, region, image)
    } else {
        load_raw_image(dest, region, image)
    }
}
