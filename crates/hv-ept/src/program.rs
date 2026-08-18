//! EPT table programming for hardware backend bring-up.

use alloc::vec;
use alloc::vec::Vec;

use crate::backend::EptBackend;
use crate::constants::{EPT_PAGE_SIZE_BYTES, EPT_ROOT_TABLE_BYTES};
use crate::error::{EptError, EptErrorKind};
use crate::plan::{EptIdentityMapping, EptInitPlan};

/// EPT permission and memory-type bits used for identity leaf entries (MODEL encoding).
pub const EPT_ENTRY_READ: u64 = 1 << 0;
/// EPT entry write permission bit.
pub const EPT_ENTRY_WRITE: u64 = 1 << 1;
/// EPT entry execute permission bit.
pub const EPT_ENTRY_EXECUTE: u64 = 1 << 2;
/// EPT entry write-back memory type field.
pub const EPT_ENTRY_MEMORY_TYPE_WB: u64 = 6 << 3;

/// One encoded identity EPT leaf entry derived from the init plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EptProgrammedMapping {
    /// Guest physical base.
    pub guest_phys: u64,
    /// Host physical base.
    pub host_phys: u64,
    /// Mapping size in bytes.
    pub size_bytes: u64,
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
}

/// Encodes a leaf EPT entry for an identity mapping.
pub fn encode_identity_ept_entry(host_phys: u64) -> u64 {
    let page_frame = host_phys >> 12;
    EPT_ENTRY_READ | EPT_ENTRY_WRITE | EPT_ENTRY_EXECUTE | EPT_ENTRY_MEMORY_TYPE_WB
        | (page_frame << 12)
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
    let mut root_table = vec![0u8; EPT_ROOT_TABLE_BYTES as usize];
    if let Some(first) = mappings.first() {
        let entry_bytes = first.encoded_entry.to_le_bytes();
        if let Some(slot) = root_table.get_mut(0..8) {
            slot.copy_from_slice(&entry_bytes);
        }
    }
    Ok(EptProgrammedTables {
        root_table_phys: plan.root_table_phys.raw(),
        root_table,
        mappings,
    })
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
