//! Host physical page installation for REAL_HW Gate C bring-up.

use hv_ept::EptProgrammedTables;
use hv_vmx::{VmxonProgrammedRegion, REFERENCE_VMXON_REVISION};

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::instructions::{read_vmx_basic_msr, vmx_revision_from_basic_msr};

/// Minimum VMCS region size (one page).
pub const VMCS_REGION_BYTES: usize = 4096;

/// Installs programmed bytes into host physical pages.
pub trait PageAllocator {
    /// Allocates contiguous host physical pages and returns the base address.
    fn allocate_pages(&mut self, size: usize, align: u64) -> Result<u64, CpuSeamError>;

    /// Copies bytes into a previously allocated host physical region.
    fn copy_to_pages(&mut self, host_phys: u64, bytes: &[u8]) -> Result<(), CpuSeamError>;
}

/// Resolves the VMX revision identifier for VMXON region programming.
pub fn resolve_vmxon_revision() -> u32 {
    read_vmx_basic_msr()
        .map(vmx_revision_from_basic_msr)
        .unwrap_or(REFERENCE_VMXON_REVISION)
}

/// Installs a programmed VMXON region into host physical memory.
pub fn install_vmxon_region<A: PageAllocator>(
    allocator: &mut A,
    region: &VmxonProgrammedRegion,
) -> Result<VmxonProgrammedRegion, CpuSeamError> {
    if region.bytes.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMXON region bytes must not be empty",
        ));
    }
    let host_phys = allocator.allocate_pages(region.bytes.len(), 4096)?;
    allocator.copy_to_pages(host_phys, &region.bytes)?;
    Ok(VmxonProgrammedRegion {
        host_phys,
        bytes: region.bytes.clone(),
    })
}

/// Installs programmed EPT tables into host physical memory.
pub fn install_ept_tables<A: PageAllocator>(
    allocator: &mut A,
    tables: &EptProgrammedTables,
) -> Result<EptProgrammedTables, CpuSeamError> {
    if tables.root_table.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "EPT root table bytes must not be empty",
        ));
    }
    let root_table_phys = allocator.allocate_pages(tables.root_table.len(), 4096)?;
    allocator.copy_to_pages(root_table_phys, &tables.root_table)?;
    Ok(EptProgrammedTables {
        root_table_phys,
        root_table: tables.root_table.clone(),
        mappings: tables.mappings.clone(),
    })
}

/// Allocates and clears a VMCS region page for EPT pointer programming.
pub fn install_vmcs_region<A: PageAllocator>(
    allocator: &mut A,
) -> Result<u64, CpuSeamError> {
    let revision = resolve_vmxon_revision();
    let mut bytes = alloc::vec![0u8; VMCS_REGION_BYTES];
    if let Some(prefix) = bytes.get_mut(0..4) {
        prefix.copy_from_slice(&revision.to_le_bytes());
    }
    let host_phys = allocator.allocate_pages(bytes.len(), 4096)?;
    allocator.copy_to_pages(host_phys, &bytes)?;
    Ok(host_phys)
}

/// Mock page allocator for host tests: records installs without real mapping.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MockPageAllocator {
    next_phys: u64,
    /// Recorded `(host_phys, size)` allocation pairs.
    pub allocations: alloc::vec::Vec<(u64, usize)>,
    /// Recorded `(host_phys, byte_len)` copy operations.
    pub copies: alloc::vec::Vec<(u64, usize)>,
}

impl MockPageAllocator {
    /// Creates a mock allocator with a deterministic physical base.
    pub fn new(base_phys: u64) -> Self {
        Self {
            next_phys: base_phys,
            allocations: alloc::vec::Vec::new(),
            copies: alloc::vec::Vec::new(),
        }
    }
}

impl PageAllocator for MockPageAllocator {
    fn allocate_pages(&mut self, size: usize, align: u64) -> Result<u64, CpuSeamError> {
        if size == 0 {
            return Err(CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "allocation size must be non-zero",
            ));
        }
        if align == 0 || align & (align - 1) != 0 {
            return Err(CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "allocation alignment must be a power of two",
            ));
        }
        let mask = align - 1;
        self.next_phys = (self.next_phys + mask) & !mask;
        let host_phys = self.next_phys;
        self.allocations.push((host_phys, size));
        self.next_phys = self.next_phys.saturating_add(size as u64);
        Ok(host_phys)
    }

    fn copy_to_pages(&mut self, host_phys: u64, bytes: &[u8]) -> Result<(), CpuSeamError> {
        if host_phys == 0 {
            return Err(CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "copy target address must be non-zero",
            ));
        }
        if !self
            .allocations
            .iter()
            .any(|(phys, size)| *phys == host_phys && *size >= bytes.len())
        {
            return Err(CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "copy target was not allocated by this allocator",
            ));
        }
        self.copies.push((host_phys, bytes.len()));
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_ept::{plan_ept_init, program_ept_tables};
    use hv_platform_model::plan_static_platform_ir;
    use hv_vmx::{plan_vmx_init, program_vmxon_region};

    #[test]
    fn install_vmxon_region_rebinds_host_phys() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let region =
            program_vmxon_region(&plan, REFERENCE_VMXON_REVISION).expect("program");
        let planner_phys = region.host_phys;
        let mut allocator = MockPageAllocator::new(0x0000_0000_0100_0000);
        let installed = install_vmxon_region(&mut allocator, &region).expect("install");
        assert_ne!(installed.host_phys, planner_phys);
        assert_eq!(installed.bytes, region.bytes);
        assert!(!allocator.allocations.is_empty());
    }

    #[test]
    fn install_ept_tables_rebinds_root_table_phys() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
        let tables = program_ept_tables(&plan).expect("program");
        let planner_phys = tables.root_table_phys;
        let mut allocator = MockPageAllocator::new(0x0000_0000_0200_0000);
        let installed = install_ept_tables(&mut allocator, &tables).expect("install");
        assert_ne!(installed.root_table_phys, planner_phys);
    }

    #[test]
    fn install_vmcs_region_allocates_revision_prefix_page() {
        let mut allocator = MockPageAllocator::new(0x0000_0000_0300_0000);
        let vmcs_phys = install_vmcs_region(&mut allocator).expect("install vmcs");
        assert_eq!(vmcs_phys & 0xFFF, 0);
        assert_eq!(allocator.copies.len(), 1);
    }

    #[test]
    fn resident_mock_allocator_rejects_zero_size() {
        let mut allocator = MockPageAllocator::new(0x1000);
        assert!(allocator.allocate_pages(0, 4096).is_err());
    }

    #[test]
    fn resident_mock_copy_rejects_zero_target() {
        let mut allocator = MockPageAllocator::new(0x1000);
        let phys = allocator.allocate_pages(4096, 4096).expect("allocate");
        assert!(allocator.copy_to_pages(0, &[0u8; 16]).is_err());
        assert!(allocator.copy_to_pages(phys, &[0u8; 16]).is_ok());
    }

    #[test]
    fn resident_mock_allocator_rejects_invalid_alignment() {
        let mut allocator = MockPageAllocator::new(0x1000);
        assert!(allocator.allocate_pages(4096, 0).is_err());
        assert!(allocator.allocate_pages(4096, 3).is_err());
    }

    #[test]
    fn resident_mock_copy_rejects_unknown_target() {
        let mut allocator = MockPageAllocator::new(0x1000);
        assert!(allocator.copy_to_pages(0x2000, &[0u8; 16]).is_err());
    }

    #[test]
    fn install_vmxon_region_rejects_empty_bytes() {
        let region = VmxonProgrammedRegion {
            host_phys: 0x1000,
            bytes: alloc::vec::Vec::new(),
        };
        let mut allocator = MockPageAllocator::new(0x1000);
        assert!(install_vmxon_region(&mut allocator, &region).is_err());
    }
}
