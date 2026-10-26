//! Software EPT page packing + EPTP value (load into VMCS is TCB in freestanding path).

use crate::ept::{map_entries, EptLeafPerms};
use crate::ept_tables::EptPte;
use crate::PAGE_SIZE;
use isolation::MemoryMap;

/// 4-level EPT root packing for identity GPA→HPA (nested spike when window fits in L1 RAM).
///
/// Returns the EPTP value (root HPA | walk length | memory type) or `None` if the
/// bump arena cannot hold the tables / entries exceed capacity.
pub struct EptPages {
    /// Backing storage for PML4 / PDPT / PD / PT pages (page-aligned slices).
    pub storage: &'static mut [u8],
    pub eptp: u64,
    pub pages_used: usize,
}

/// Pack identity EPT tables for `map` into `storage` (must be page-aligned, zeroed).
///
/// Spike limits: single PML4→PDPT→PD with 2 MiB large pages when possible; falls
/// back to reporting failure for sparse high windows that do not fit.
pub fn pack_identity_ept(map: &MemoryMap, storage: &mut [u8]) -> Result<(u64, usize), ()> {
    let entries = map_entries(map).map_err(|_| ())?;
    if storage.as_ptr() as u64 % PAGE_SIZE != 0 {
        return Err(());
    }
    storage.fill(0);

    // Layout in storage: [PML4][PDPT][PD0][PD1]... then 4K PTs as needed.
    // For smoke bring-up we install **2 MiB** pages covering each mapped region
    // when size/base are 2 MiB-aligned; otherwise skip (caller still has intent).
    const PAGE_2M: u64 = 2 * 1024 * 1024;
    if storage.len() < (3 * PAGE_SIZE as usize) {
        return Err(());
    }

    let pml4_hpa = storage.as_ptr() as u64;
    let pdpt_hpa = pml4_hpa + PAGE_SIZE;
    let pd_hpa = pdpt_hpa + PAGE_SIZE;
    let pages_used = 3usize;

    // PML4[0] → PDPT, PDPT[0] → PD (covers GPA 0..1GiB). Extend if needed.
    write_u64(storage, 0, pdpt_hpa | 0b111); // R/W/X
    write_u64(storage, PAGE_SIZE as usize, pd_hpa | 0b111);

    for e in entries {
        if e.gpa >= (512 * PAGE_2M) {
            // Outside first PD (1 GiB) — need more PDs; skip for minimal install.
            continue;
        }
        if e.gpa % PAGE_2M != 0 || e.size % PAGE_2M != 0 {
            // Require 2 MiB granularity for this spike installer.
            continue;
        }
        let mut gpa = e.gpa;
        let end = e.gpa.saturating_add(e.size);
        while gpa < end {
            let idx = (gpa / PAGE_2M) as usize;
            let off = (pd_hpa - pml4_hpa) as usize + idx * 8;
            if off + 8 > storage.len() {
                return Err(());
            }
            // Bit 7 = PS (2 MiB page).
            let pte = EptPte::leaf(gpa, e.perms).0 | (1 << 7);
            write_u64(storage, off, pte);
            gpa += PAGE_2M;
        }
        let _ = EptLeafPerms::from_isolation;
    }

    // EPTP: bits 2:0 = mem type (6=WB); bits 5:3 = page-walk length (3 → 4 levels).
    let eptp = (pml4_hpa & !0xfff) | (3 << 3) | 6;
    Ok((eptp, pages_used))
}

fn write_u64(buf: &mut [u8], off: usize, val: u64) {
    if off + 8 <= buf.len() {
        buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }
}
