//! Four-level EPT paging materialization for programmed guest mappings.

use alloc::vec;
use alloc::vec::Vec;

use crate::constants::EPT_PAGE_SIZE_BYTES;
use crate::error::{EptError, EptErrorKind};
use crate::program::{
    encode_identity_ept_entry, EptProgrammedTables, EPT_ENTRY_EXECUTE, EPT_ENTRY_MEMORY_TYPE_WB,
    EPT_ENTRY_READ, EPT_ENTRY_WRITE,
};

const EPT_ENTRIES_PER_TABLE: usize = 512;
const EPT_TABLE_BYTES: usize = 4096;
const EPT_SYNTHETIC_TABLE_FLAG: u64 = 1 << 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableRef {
    Root,
    Nested(usize),
}

/// Rebuilds the in-memory EPT hierarchy from programmed mapping records.
pub fn materialize_ept_paging(tables: &mut EptProgrammedTables) -> Result<(), EptError> {
    tables.paging_tables.clear();
    tables.root_table = vec![0u8; EPT_TABLE_BYTES];
    let mappings = tables.mappings.clone();
    for mapping in &mappings {
        if mapping.size_bytes == 0 || mapping.size_bytes % EPT_PAGE_SIZE_BYTES != 0 {
            return Err(EptError::new(
                EptErrorKind::Planning,
                "EPT mapping size must be a non-zero page multiple",
            ));
        }
        let page_count = mapping
            .size_bytes
            .checked_div(EPT_PAGE_SIZE_BYTES)
            .ok_or_else(|| EptError::new(EptErrorKind::Planning, "EPT mapping page count overflow"))?;
        for page_index in 0..page_count {
            let guest_page = mapping
                .guest_phys
                .checked_add(page_index * EPT_PAGE_SIZE_BYTES)
                .ok_or_else(|| EptError::new(EptErrorKind::Planning, "guest page address overflow"))?;
            let host_page = mapping
                .host_phys
                .checked_add(page_index * EPT_PAGE_SIZE_BYTES)
                .ok_or_else(|| EptError::new(EptErrorKind::Planning, "host page address overflow"))?;
            map_guest_page(tables, guest_page, host_page)?;
        }
    }
    Ok(())
}

/// Returns whether the in-memory EPT hierarchy maps a guest physical page.
pub fn ept_maps_guest_page(tables: &EptProgrammedTables, guest_phys: u64) -> bool {
    ept_resolve_guest_page(tables, guest_phys).is_some()
}

/// Resolves a guest physical page to its host physical base via the in-memory hierarchy.
pub fn ept_resolve_guest_page(tables: &EptProgrammedTables, guest_phys: u64) -> Option<u64> {
    if guest_phys % EPT_PAGE_SIZE_BYTES != 0 {
        return None;
    }
    let indices = ept_indices(guest_phys);
    let pdpt = child_table(tables, TableRef::Root, indices.pml4)?;
    let pd = child_table(tables, TableRef::Nested(pdpt), indices.pdpt)?;
    let pt = child_table(tables, TableRef::Nested(pd), indices.pd)?;
    let entry = read_entry(table_bytes(tables, TableRef::Nested(pt)), indices.pt);
    decode_leaf_host_phys(entry)
}

/// Patches synthetic child-table pointers with installed host physical addresses.
pub fn patch_ept_table_host_phys(tables: &mut EptProgrammedTables, nested_phys: &[u64]) {
    patch_table_entries(&mut tables.root_table, nested_phys);
    for table in &mut tables.paging_tables {
        patch_table_entries(table, nested_phys);
    }
}

fn map_guest_page(
    tables: &mut EptProgrammedTables,
    guest_phys: u64,
    host_phys: u64,
) -> Result<(), EptError> {
    let indices = ept_indices(guest_phys);
    ensure_child_table(tables, TableRef::Root, indices.pml4)?;
    let pdpt = child_table_index(tables, TableRef::Root, indices.pml4).ok_or_else(|| {
        EptError::new(EptErrorKind::Planning, "EPT PDPT child table unavailable")
    })?;
    ensure_child_table(tables, TableRef::Nested(pdpt), indices.pdpt)?;
    let pd = child_table_index(tables, TableRef::Nested(pdpt), indices.pdpt).ok_or_else(|| {
        EptError::new(EptErrorKind::Planning, "EPT PD child table unavailable")
    })?;
    ensure_child_table(tables, TableRef::Nested(pd), indices.pd)?;
    let pt = child_table_index(tables, TableRef::Nested(pd), indices.pd).ok_or_else(|| {
        EptError::new(EptErrorKind::Planning, "EPT PT child table unavailable")
    })?;
    write_entry(
        table_bytes_mut(tables, TableRef::Nested(pt)),
        indices.pt,
        encode_identity_ept_entry(host_phys),
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EptIndices {
    pml4: usize,
    pdpt: usize,
    pd: usize,
    pt: usize,
}

fn ept_indices(guest_phys: u64) -> EptIndices {
    EptIndices {
        pml4: ((guest_phys >> 39) & 0x1FF) as usize,
        pdpt: ((guest_phys >> 30) & 0x1FF) as usize,
        pd: ((guest_phys >> 21) & 0x1FF) as usize,
        pt: ((guest_phys >> 12) & 0x1FF) as usize,
    }
}

fn ensure_child_table(
    tables: &mut EptProgrammedTables,
    parent: TableRef,
    index: usize,
) -> Result<(), EptError> {
    let parent_bytes = table_bytes(tables, parent);
    if read_entry(parent_bytes, index) != 0 {
        return Ok(());
    }
    let child = alloc_child_table(tables);
    write_entry(
        table_bytes_mut(tables, parent),
        index,
        encode_synthetic_table_pointer(child),
    );
    Ok(())
}

fn alloc_child_table(tables: &mut EptProgrammedTables) -> usize {
    let index = tables.paging_tables.len();
    tables.paging_tables.push(vec![0u8; EPT_TABLE_BYTES]);
    index
}

fn table_bytes<'a>(tables: &'a EptProgrammedTables, table_ref: TableRef) -> &'a [u8] {
    match table_ref {
        TableRef::Root => tables.root_table.as_slice(),
        TableRef::Nested(index) => tables
            .paging_tables
            .get(index)
            .map(|table| table.as_slice())
            .unwrap_or(&[]),
    }
}

fn table_bytes_mut<'a>(tables: &'a mut EptProgrammedTables, table_ref: TableRef) -> &'a mut [u8] {
    match table_ref {
        TableRef::Root => tables.root_table.as_mut_slice(),
        TableRef::Nested(index) => tables
            .paging_tables
            .get_mut(index)
            .map(|table| table.as_mut_slice())
            .unwrap_or(&mut []),
    }
}

fn child_table_index(tables: &EptProgrammedTables, parent: TableRef, index: usize) -> Option<usize> {
    let entry = read_entry(table_bytes(tables, parent), index);
    synthetic_table_index(entry)
}

fn child_table(tables: &EptProgrammedTables, parent: TableRef, index: usize) -> Option<usize> {
    child_table_index(tables, parent, index)
}

fn decode_leaf_host_phys(entry: u64) -> Option<u64> {
    if entry == 0 || entry & EPT_SYNTHETIC_TABLE_FLAG != 0 {
        return None;
    }
    Some(entry & 0x000F_FFFF_FFFF_F000)
}

fn encode_synthetic_table_pointer(index: usize) -> u64 {
    EPT_SYNTHETIC_TABLE_FLAG | ((index as u64) << 12)
}

fn synthetic_table_index(entry: u64) -> Option<usize> {
    if entry & EPT_SYNTHETIC_TABLE_FLAG == 0 {
        return None;
    }
    Some(((entry >> 12) & 0x7FFF_FFFF) as usize)
}

fn encode_table_pointer(host_phys: u64) -> u64 {
    EPT_ENTRY_READ
        | EPT_ENTRY_WRITE
        | EPT_ENTRY_EXECUTE
        | EPT_ENTRY_MEMORY_TYPE_WB
        | (host_phys & 0x000F_FFFF_FFFF_F000)
}

fn patch_table_entries(table: &mut [u8], nested_phys: &[u64]) {
    for slot in 0..EPT_ENTRIES_PER_TABLE {
        let entry = read_entry(table, slot);
        if entry & EPT_SYNTHETIC_TABLE_FLAG == 0 {
            continue;
        }
        let Some(index) = synthetic_table_index(entry) else {
            continue;
        };
        let Some(host_phys) = nested_phys.get(index).copied() else {
            continue;
        };
        write_entry(table, slot, encode_table_pointer(host_phys));
    }
}

fn read_entry(table: &[u8], index: usize) -> u64 {
    let offset = index.checked_mul(8).unwrap_or(0);
    let bytes = table.get(offset..offset + 8).unwrap_or(&[]);
    if bytes.len() < 8 {
        return 0;
    }
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

fn write_entry(table: &mut [u8], index: usize, value: u64) {
    let Some(offset) = index.checked_mul(8) else {
        return;
    };
    if let Some(slot) = table.get_mut(offset..offset + 8) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::program::append_ept_guest_mapping;
    use hv_datapath::RELAY_MEASUREMENT_PAGE_GUEST_PHYS;

    fn empty_tables() -> EptProgrammedTables {
        EptProgrammedTables {
            root_table_phys: 0,
            root_table: vec![0u8; EPT_TABLE_BYTES],
            mappings: Vec::new(),
            paging_tables: Vec::new(),
        }
    }

    #[test]
    fn materialize_maps_high_guest_physical_page() {
        let mut tables = empty_tables();
        append_ept_guest_mapping(
            &mut tables,
            RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
            0x3000,
            EPT_PAGE_SIZE_BYTES,
        )
        .expect("append");
        assert!(ept_maps_guest_page(&tables, RELAY_MEASUREMENT_PAGE_GUEST_PHYS));
        assert_eq!(
            ept_resolve_guest_page(&tables, RELAY_MEASUREMENT_PAGE_GUEST_PHYS),
            Some(0x3000)
        );
    }

    #[test]
    fn materialize_maps_low_identity_and_high_guest_pages() {
        let mut tables = empty_tables();
        append_ept_guest_mapping(&mut tables, 0, 0, 0x10_0000).expect("low");
        append_ept_guest_mapping(
            &mut tables,
            RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
            0x4000,
            EPT_PAGE_SIZE_BYTES,
        )
        .expect("high");
        assert!(ept_maps_guest_page(&tables, 0x1000));
        assert!(ept_maps_guest_page(&tables, RELAY_MEASUREMENT_PAGE_GUEST_PHYS));
    }

    #[test]
    fn patch_ept_table_host_phys_replaces_synthetic_child_pointers() {
        let mut tables = empty_tables();
        append_ept_guest_mapping(
            &mut tables,
            RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
            0x5000,
            EPT_PAGE_SIZE_BYTES,
        )
        .expect("append");
        let nested_count = tables.paging_tables.len();
        assert!(nested_count >= 3);
        let nested_phys: Vec<u64> = (0..nested_count)
            .map(|index| 0x10_0000 + index as u64 * 4096)
            .collect();
        patch_ept_table_host_phys(&mut tables, &nested_phys);
        assert!(tables.root_table.iter().any(|&byte| byte != 0));
        assert!(tables.root_table.chunks(8).any(|chunk| {
            u64::from_le_bytes(chunk.try_into().expect("entry")) & EPT_SYNTHETIC_TABLE_FLAG == 0
        }));
    }
}
