//! EPT table programming for hardware backend bring-up.

use alloc::vec;
use alloc::vec::Vec;

use crate::backend::EptBackend;
use crate::constants::{
    EPT_PAGE_SIZE_BYTES, EPT_POINTER_MEMORY_TYPE_SHIFT, EPT_POINTER_MEMORY_TYPE_WB,
    EPT_ROOT_TABLE_BYTES,
};
use crate::error::{EptError, EptErrorKind};
use crate::paging::materialize_ept_paging;
use crate::plan::{EptIdentityMapping, EptInitPlan};

/// EPT permission and memory-type bits used for identity leaf entries (MODEL encoding).
pub const EPT_ENTRY_READ: u64 = 1 << 0;
/// EPT entry write permission bit.
pub const EPT_ENTRY_WRITE: u64 = 1 << 1;
/// EPT entry execute permission bit.
pub const EPT_ENTRY_EXECUTE: u64 = 1 << 2;
/// EPT entry write-back memory type field.
pub const EPT_ENTRY_MEMORY_TYPE_WB: u64 =
    EPT_POINTER_MEMORY_TYPE_WB << EPT_POINTER_MEMORY_TYPE_SHIFT;

/// One encoded identity EPT leaf entry derived from the init plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EptProgrammedMapping {
    /// Guest physical base.
    pub guest_phys: u64,
    /// Host physical base.
    pub host_phys: u64,
    /// Mapping size in bytes.
    pub size_bytes: u64,
    /// Whether the guest may write through this mapping.
    pub guest_writable: bool,
    /// Encoded 64-bit EPT entry value.
    pub encoded_entry: u64,
}

/// Encoded EPT hierarchy bytes ready for host physical installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EptProgrammedTables {
    /// Host physical base of the EPT root table.
    pub root_table_phys: u64,
    /// Root table bytes (one page).
    pub root_table: Vec<u8>,
    /// Encoded identity mappings backing the hierarchy.
    pub mappings: Vec<EptProgrammedMapping>,
    /// Additional paging levels materialized for non-contiguous guest mappings.
    pub paging_tables: Vec<Vec<u8>>,
}

/// Appends one guest/host EPT mapping record for runtime-installed pages.
pub fn append_ept_guest_mapping(
    tables: &mut EptProgrammedTables,
    guest_phys: u64,
    host_phys: u64,
    size_bytes: u64,
) -> Result<(), EptError> {
    append_ept_guest_mapping_with_permissions(tables, guest_phys, host_phys, size_bytes, true)
}

/// Appends a guest read-only EPT mapping (hypervisor write via host physical access).
pub fn append_ept_guest_read_only_mapping(
    tables: &mut EptProgrammedTables,
    guest_phys: u64,
    host_phys: u64,
    size_bytes: u64,
) -> Result<(), EptError> {
    append_ept_guest_mapping_with_permissions(tables, guest_phys, host_phys, size_bytes, false)
}

fn append_ept_guest_mapping_with_permissions(
    tables: &mut EptProgrammedTables,
    guest_phys: u64,
    host_phys: u64,
    size_bytes: u64,
    guest_writable: bool,
) -> Result<(), EptError> {
    if size_bytes == 0 || size_bytes % EPT_PAGE_SIZE_BYTES != 0 {
        return Err(EptError::new(
            EptErrorKind::Planning,
            "EPT mapping size must be a non-zero page multiple",
        ));
    }
    if guest_phys % EPT_PAGE_SIZE_BYTES != 0 || host_phys % EPT_PAGE_SIZE_BYTES != 0 {
        return Err(EptError::new(
            EptErrorKind::Planning,
            "EPT mapping bases must be page aligned",
        ));
    }
    let guest_end = guest_phys.checked_add(size_bytes).ok_or_else(|| {
        EptError::new(EptErrorKind::Planning, "EPT mapping guest end overflow")
    })?;
    for existing in &tables.mappings {
        let existing_end = existing
            .guest_phys
            .checked_add(existing.size_bytes)
            .ok_or_else(|| EptError::new(EptErrorKind::Planning, "EPT mapping guest end overflow"))?;
        if guest_phys < existing_end && existing.guest_phys < guest_end {
            return Err(EptError::new(
                EptErrorKind::Planning,
                "EPT guest mapping overlaps an existing mapping",
            ));
        }
    }
    tables.mappings.push(EptProgrammedMapping {
        guest_phys,
        host_phys,
        size_bytes,
        guest_writable,
        encoded_entry: encode_ept_leaf_entry(host_phys, guest_writable),
    });
    materialize_ept_paging(tables)?;
    Ok(())
}

/// Encodes a leaf EPT entry for an identity mapping.
pub fn encode_identity_ept_entry(host_phys: u64) -> u64 {
    encode_ept_leaf_entry(host_phys, true)
}

/// Encodes a leaf EPT entry with optional guest write permission.
pub fn encode_ept_leaf_entry(host_phys: u64, guest_writable: bool) -> u64 {
    let page_frame = host_phys >> 12;
    let mut entry = EPT_ENTRY_READ | EPT_ENTRY_EXECUTE | EPT_ENTRY_MEMORY_TYPE_WB | (page_frame << 12);
    if guest_writable {
        entry |= EPT_ENTRY_WRITE;
    }
    entry
}

/// Returns whether an encoded leaf entry grants guest write permission.
pub fn ept_leaf_entry_guest_writable(entry: u64) -> bool {
    entry & EPT_ENTRY_WRITE != 0
}

/// Programs EPT root table bytes and mapping records from an init plan.
pub fn program_ept_tables(plan: &EptInitPlan) -> Result<EptProgrammedTables, EptError> {
    if plan.root_table_bytes.bytes() < EPT_ROOT_TABLE_BYTES {
        return Err(EptError::new(
            EptErrorKind::Planning,
            "EPT root table size below minimum page",
        ));
    }
    let mut mappings = Vec::with_capacity(plan.identity_mappings.len());
    for mapping in &plan.identity_mappings {
        mappings.push(program_identity_mapping(mapping)?);
    }
    let root_table = vec![0u8; EPT_ROOT_TABLE_BYTES as usize];
    let mut tables = EptProgrammedTables {
        root_table_phys: plan.root_table_phys.raw(),
        root_table,
        mappings,
        paging_tables: Vec::new(),
    };
    materialize_ept_paging(&mut tables)?;
    Ok(tables)
}

fn program_identity_mapping(mapping: &EptIdentityMapping) -> Result<EptProgrammedMapping, EptError> {
    if mapping.size_bytes.bytes() == 0 {
        return Err(EptError::new(
            EptErrorKind::Planning,
            "EPT mapping size must not be zero",
        ));
    }
    if mapping.guest_phys.raw() % EPT_PAGE_SIZE_BYTES != 0 {
        return Err(EptError::new(
            EptErrorKind::Planning,
            "EPT guest mapping base is not page aligned",
        ));
    }
    Ok(EptProgrammedMapping {
        guest_phys: mapping.guest_phys.raw(),
        host_phys: mapping.host_phys.raw(),
        size_bytes: mapping.size_bytes.bytes(),
        guest_writable: true,
        encoded_entry: encode_identity_ept_entry(mapping.host_phys.raw()),
    })
}

/// Backend that encodes EPT tables without executing EPT-capable VMX instructions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgrammingEptBackend {
    /// Number of successful programming calls.
    pub program_calls: u32,
    /// Last programmed EPT hierarchy.
    pub last_tables: Option<EptProgrammedTables>,
}

impl EptBackend for ProgrammingEptBackend {
    fn install_ept(&mut self, plan: &EptInitPlan) -> Result<(), EptError> {
        let tables = program_ept_tables(plan)?;
        self.program_calls = self.program_calls.saturating_add(1);
        self.last_tables = Some(tables);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use hv_vmx::plan_vmx_init;
    use crate::plan::plan_ept_init;

    #[test]
    fn program_ept_tables_builds_root_and_mappings_for_reference_plan() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
        let tables = program_ept_tables(&plan).expect("program");
        assert_eq!(tables.root_table.len(), EPT_ROOT_TABLE_BYTES as usize);
        assert_eq!(tables.mappings.len(), plan.identity_mappings.len());
        assert!(tables.mappings.iter().all(|entry| entry.encoded_entry & EPT_ENTRY_READ != 0));
    }
}
