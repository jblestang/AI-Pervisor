//! Minimal guest long-mode page tables for L2 enter (host-testable packing).
//!
//! Guests are linked at a low VA (typically `0x20_0000`). We map:
//! - `[link_base, link_base + image_span)` → `private_base + (va - link_base)` with 2 MiB pages
//! - listed identity GPAs (boot info, rings, arena, stats) as VA == GPA
//!
//! Tables live in a caller-supplied page-aligned buffer; the returned CR3 is the
//! buffer's base interpreted as a GPA (identity EPT → HPA).

use crate::PAGE_SIZE;

const PAGE_2M: u64 = 2 * 1024 * 1024;

/// Errors while packing guest page tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuestPtError {
    UnalignedStorage,
    TooSmall,
    Overflow,
    UnalignedMap,
}

/// One VA→GPA translation request (page-aligned; size multiple of 4 KiB).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestMap {
    pub va: u64,
    pub gpa: u64,
    pub size: u64,
}

/// Result of packing: CR3 GPA (= storage base under identity EPT) + pages used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestPtPack {
    pub cr3_gpa: u64,
    pub pages_used: usize,
}

fn align_up(v: u64, a: u64) -> Option<u64> {
    let m = a - 1;
    v.checked_add(m).map(|x| x & !m)
}

fn write_u64(buf: &mut [u8], off: usize, val: u64) -> Result<(), GuestPtError> {
    if off + 8 > buf.len() {
        return Err(GuestPtError::TooSmall);
    }
    buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
    Ok(())
}

/// Pack a 4-level identity-friendly guest page table into `storage`.
///
/// Layout: `[PML4][PDPT][PD]` covering the low 1 GiB with 2 MiB pages for each
/// requested map. Maps outside the first 1 GiB are rejected (`Overflow`).
pub fn pack_guest_pt(storage: &mut [u8], maps: &[GuestMap]) -> Result<GuestPtPack, GuestPtError> {
    if storage.as_ptr() as u64 % PAGE_SIZE != 0 {
        return Err(GuestPtError::UnalignedStorage);
    }
    if storage.len() < 3 * PAGE_SIZE as usize {
        return Err(GuestPtError::TooSmall);
    }
    storage[..3 * PAGE_SIZE as usize].fill(0);

    let pml4 = storage.as_ptr() as u64;
    let pdpt = pml4 + PAGE_SIZE;
    let pd = pdpt + PAGE_SIZE;

    // Present|Writable|User (guest walks these as supervisor; U=1 is fine).
    const FLAGS: u64 = 0b111;
    write_u64(storage, 0, pdpt | FLAGS)?;
    write_u64(storage, PAGE_SIZE as usize, pd | FLAGS)?;

    for m in maps {
        if m.size == 0 {
            continue;
        }
        if m.va % PAGE_SIZE != 0 || m.gpa % PAGE_SIZE != 0 || m.size % PAGE_SIZE != 0 {
            return Err(GuestPtError::UnalignedMap);
        }
        // Expand to 2 MiB coverage for the spike installer.
        let va_start = m.va & !(PAGE_2M - 1);
        let va_end = align_up(m.va.checked_add(m.size).ok_or(GuestPtError::Overflow)?, PAGE_2M)
            .ok_or(GuestPtError::Overflow)?;
        let gpa_start = m.gpa.checked_sub(m.va.checked_sub(va_start).ok_or(GuestPtError::Overflow)?)
            .ok_or(GuestPtError::Overflow)?;

        let mut va = va_start;
        let mut gpa = gpa_start;
        while va < va_end {
            if va >= 512 * PAGE_2M {
                return Err(GuestPtError::Overflow);
            }
            let idx = (va / PAGE_2M) as usize;
            let off = (pd - pml4) as usize + idx * 8;
            // PS (bit 7) = 2 MiB page; Present|RW|User.
            write_u64(storage, off, gpa | FLAGS | (1 << 7))?;
            va = va.checked_add(PAGE_2M).ok_or(GuestPtError::Overflow)?;
            gpa = gpa.checked_add(PAGE_2M).ok_or(GuestPtError::Overflow)?;
        }
    }

    Ok(GuestPtPack {
        cr3_gpa: pml4,
        pages_used: 3,
    })
}

/// Build the standard map list for a spike guest: image window + identity tips.
pub fn spike_guest_maps(
    link_base: u64,
    private_base: u64,
    image_span: u64,
    identity: &[u64],
) -> Result<alloc::vec::Vec<GuestMap>, GuestPtError> {
    let mut maps = alloc::vec::Vec::new();
    let span = align_up(image_span.max(PAGE_2M), PAGE_2M).ok_or(GuestPtError::Overflow)?;
    maps.push(GuestMap {
        va: link_base,
        gpa: private_base,
        size: span,
    });
    for &gpa in identity {
        let base = gpa & !0xfff;
        maps.push(GuestMap {
            va: base,
            gpa: base,
            size: PAGE_SIZE,
        });
    }
    Ok(maps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C, align(4096))]
    struct PageBuf([u8; 16 * 1024]);

    #[test]
    fn packs_image_and_identity_maps() {
        let mut buf = PageBuf([0; 16 * 1024]);
        let maps = spike_guest_maps(
            0x20_0000,
            0x1000_0000,
            0x10_0000,
            &[0x1200_0000, 0x1200_1000],
        )
        .unwrap();
        let pack = pack_guest_pt(&mut buf.0, &maps).unwrap();
        assert_eq!(pack.pages_used, 3);
        assert_eq!(pack.cr3_gpa, buf.0.as_ptr() as u64);

        // PD index for VA 0x200000 = 1 (2 MiB pages).
        let pd_off = 2 * 4096 + 1 * 8;
        let leaf = u64::from_le_bytes(buf.0[pd_off..pd_off + 8].try_into().unwrap());
        assert_eq!(leaf & !0xfff, 0x1000_0000);
        assert_ne!(leaf & (1 << 7), 0, "PS bit");

        // Identity GPA 0x12000000 → PD index 0x12000000/2MiB = 144.
        let idx = (0x1200_0000u64 / PAGE_2M) as usize;
        let off = 2 * 4096 + idx * 8;
        let leaf = u64::from_le_bytes(buf.0[off..off + 8].try_into().unwrap());
        assert_eq!(leaf & !0xfff, 0x1200_0000);
    }

    #[test]
    fn rejects_unaligned_storage() {
        let mut buf = [0u8; 16 * 1024];
        let maps = [GuestMap {
            va: 0x200000,
            gpa: 0x10000000,
            size: 0x200000,
        }];
        // Offset 1 → unaligned.
        let err = pack_guest_pt(&mut buf[1..], &maps).unwrap_err();
        assert_eq!(err, GuestPtError::UnalignedStorage);
    }
}
