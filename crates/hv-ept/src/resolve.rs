//! Guest physical to host physical resolution via programmed EPT identity mappings.

use crate::error::{EptError, EptErrorKind};
use crate::program::EptProgrammedTables;

/// Resolves a guest physical address to host physical using programmed identity mappings.
pub fn resolve_guest_phys_to_host(
    tables: &EptProgrammedTables,
    guest_phys: u64,
) -> Result<u64, EptError> {
    resolve_guest_phys_range_to_host(tables, guest_phys, 1)
}

/// Resolves the host physical base for a guest physical byte range.
pub fn resolve_guest_phys_range_to_host(
    tables: &EptProgrammedTables,
    guest_phys: u64,
    len: usize,
) -> Result<u64, EptError> {
    if len == 0 {
        return Err(EptError::new(
            EptErrorKind::Planning,
            "guest physical read length must be non-zero",
        ));
    }
    let guest_end = guest_phys.checked_add(len as u64).ok_or_else(|| {
        EptError::new(EptErrorKind::Planning, "guest physical read end overflow")
    })?;
    for mapping in &tables.mappings {
        let start = mapping.guest_phys;
        let end = start.checked_add(mapping.size_bytes).ok_or_else(|| {
            EptError::new(EptErrorKind::Planning, "EPT mapping end overflow")
        })?;
        if guest_phys >= start && guest_end <= end {
            let offset = guest_phys - start;
            return mapping
                .host_phys
                .checked_add(offset)
                .ok_or_else(|| EptError::new(EptErrorKind::Planning, "resolved host phys overflow"));
        }
    }
    Err(EptError::new(
        EptErrorKind::Planning,
        "guest physical range not fully covered by programmed EPT mappings",
    ))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::vec;
    use crate::program::{encode_identity_ept_entry, EptProgrammedMapping};

    fn sample_tables() -> EptProgrammedTables {
        EptProgrammedTables {
            root_table_phys: 0x2000,
            root_table: vec![0u8; 4096],
            mappings: vec![EptProgrammedMapping {
                guest_phys: 0x1_0000,
                host_phys: 0x10_0000,
                size_bytes: 0x20_0000,
                encoded_entry: encode_identity_ept_entry(0x10_0000),
            }],
        }
    }

    #[test]
    fn resolve_guest_phys_to_host_maps_within_identity_region() {
        let tables = sample_tables();
        let host = resolve_guest_phys_to_host(&tables, 0x1_2000).expect("resolve");
        assert_eq!(host, 0x10_2000);
    }

    #[test]
    fn resolve_guest_phys_range_to_host_rejects_partially_mapped_range() {
        let tables = sample_tables();
        assert!(resolve_guest_phys_range_to_host(&tables, 0x1_0000, 0x20_0001).is_err());
    }

    #[test]
    fn resolve_guest_phys_range_to_host_accepts_in_range_read() {
        let tables = sample_tables();
        let host = resolve_guest_phys_range_to_host(&tables, 0x1_2000, 80).expect("resolve");
        assert_eq!(host, 0x10_2000);
    }
}
