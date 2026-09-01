//! Host physical page installation for REAL_HW Gate C bring-up.

use hv_ept::EptProgrammedTables;
use hv_vmx::{
    VmxonProgrammedRegion, REFERENCE_VMXON_REVISION, VMXON_REGION_ALIGNMENT_BYTES,
    VMXON_REGION_MIN_BYTES,
};

use crate::constants::VMXON_REVISION_PREFIX_BYTES;

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::instructions::{read_vmx_basic_msr, vmx_revision_from_basic_msr};

/// Minimum VMCS region size (one page).
pub const VMCS_REGION_BYTES: usize = VMXON_REGION_MIN_BYTES as usize;

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
    let host_phys = allocator.allocate_pages(region.bytes.len(), VMXON_REGION_ALIGNMENT_BYTES)?;
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
    use hv_ept::{materialize_ept_paging, patch_ept_table_host_phys};

    if tables.root_table.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "EPT root table bytes must not be empty",
        ));
    }
    let mut prepared = tables.clone();
    materialize_ept_paging(&mut prepared)
        .map_err(|err| CpuSeamError::new(CpuSeamErrorKind::InvalidInput, err.message))?;
    let mut nested_phys = alloc::vec::Vec::with_capacity(prepared.paging_tables.len());
    for table in &prepared.paging_tables {
        let phys = allocator.allocate_pages(table.len(), VMXON_REGION_ALIGNMENT_BYTES)?;
        allocator.copy_to_pages(phys, table)?;
        nested_phys.push(phys);
    }
    patch_ept_table_host_phys(&mut prepared, &nested_phys)
        .map_err(|err| CpuSeamError::new(CpuSeamErrorKind::InvalidInput, err.message))?;
    let root_table_phys =
        allocator.allocate_pages(prepared.root_table.len(), VMXON_REGION_ALIGNMENT_BYTES)?;
    allocator.copy_to_pages(root_table_phys, &prepared.root_table)?;
    Ok(EptProgrammedTables {
        root_table_phys,
        root_table: prepared.root_table,
        mappings: prepared.mappings,
        paging_tables: prepared.paging_tables,
    })
}

/// Allocates and clears a VMCS region page for EPT pointer programming.
pub fn install_vmcs_region<A: PageAllocator>(allocator: &mut A) -> Result<u64, CpuSeamError> {
    let revision = resolve_vmxon_revision();
    let mut bytes = alloc::vec![0u8; VMCS_REGION_BYTES];
    if let Some(prefix) = bytes.get_mut(0..VMXON_REVISION_PREFIX_BYTES) {
        prefix.copy_from_slice(&revision.to_le_bytes());
    }
    let host_phys = allocator.allocate_pages(bytes.len(), VMXON_REGION_ALIGNMENT_BYTES)?;
    allocator.copy_to_pages(host_phys, &bytes)?;
    Ok(host_phys)
}

/// Installs a guest image into freshly allocated host physical pages.
pub fn install_guest_image<A: PageAllocator>(
    allocator: &mut A,
    image: &[u8],
) -> Result<u64, CpuSeamError> {
    if image.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest image bytes must not be empty",
        ));
    }
    let size = core::cmp::max(image.len(), VMXON_REGION_MIN_BYTES as usize);
    let host_phys = allocator.allocate_pages(size, VMXON_REGION_ALIGNMENT_BYTES)?;
    allocator.copy_to_pages(host_phys, image)?;
    Ok(host_phys)
}

/// Installs a parsed guest ELF image into freshly allocated host physical pages.
#[cfg(feature = "datapath-guests")]
pub fn install_guest_elf<A: PageAllocator>(
    allocator: &mut A,
    elf_bytes: &[u8],
) -> Result<u64, CpuSeamError> {
    use hv_guest_boot::parse_elf64;

    let image = parse_elf64(elf_bytes)
        .map_err(|err| CpuSeamError::new(CpuSeamErrorKind::InvalidInput, err.message))?;
    let mut image_end = 0u64;
    for segment in &image.load_segments {
        let end = segment
            .vaddr
            .checked_add(segment.bytes.len() as u64)
            .ok_or_else(|| {
                CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "elf segment overflow")
            })?;
        image_end = image_end.max(end);
    }
    let alloc_size = core::cmp::max(image_end as usize, VMXON_REGION_MIN_BYTES as usize);
    let host_phys = allocator.allocate_pages(alloc_size, VMXON_REGION_ALIGNMENT_BYTES)?;
    for segment in &image.load_segments {
        let dest = host_phys.checked_add(segment.vaddr).ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "elf segment address overflow",
            )
        })?;
        allocator.copy_to_pages(dest, &segment.bytes)?;
    }
    host_phys
        .checked_add(image.entry_vaddr)
        .ok_or_else(|| CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "elf entry overflow"))
}

/// Installed guest ELF entry and colocated boot-info addresses.
#[cfg(feature = "datapath-guest-live")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestElfWithBootInfoInstall {
    /// Guest physical entry address for the ELF image.
    pub entry_phys: u64,
    /// Guest physical address of the installed boot-info blob.
    pub boot_info_phys: u64,
}

/// Installs a guest ELF and colocated boot-info blob in one resident allocation.
#[cfg(feature = "datapath-guest-live")]
pub fn install_guest_elf_with_boot_info<A: PageAllocator>(
    allocator: &mut A,
    elf_bytes: &[u8],
    boot_info_blob: &[u8],
) -> Result<GuestElfWithBootInfoInstall, CpuSeamError> {
    use hv_guest_boot::parse_elf64;

    if boot_info_blob.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info blob must not be empty",
        ));
    }
    let image = parse_elf64(elf_bytes)
        .map_err(|err| CpuSeamError::new(CpuSeamErrorKind::InvalidInput, err.message))?;
    let mut image_end = 0u64;
    for segment in &image.load_segments {
        let end = segment
            .vaddr
            .checked_add(segment.bytes.len() as u64)
            .ok_or_else(|| {
                CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "elf segment overflow")
            })?;
        image_end = image_end.max(end);
    }
    let boot_offset =
        align_up_usize(image_end as usize, VMXON_REGION_ALIGNMENT_BYTES as usize)? as u64;
    let boot_end = boot_offset
        .checked_add(boot_info_blob.len() as u64)
        .ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "guest boot info size overflow",
            )
        })?;
    let mut alloc_size = align_up_usize(boot_end as usize, VMXON_REGION_ALIGNMENT_BYTES as usize)?;
    alloc_size = core::cmp::max(alloc_size, VMXON_REGION_MIN_BYTES as usize);
    let host_phys = allocator.allocate_pages(alloc_size, VMXON_REGION_ALIGNMENT_BYTES)?;
    for segment in &image.load_segments {
        let dest = host_phys.checked_add(segment.vaddr).ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "elf segment address overflow",
            )
        })?;
        allocator.copy_to_pages(dest, &segment.bytes)?;
    }
    let boot_info_phys = host_phys.checked_add(boot_offset).ok_or_else(|| {
        CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "guest boot info address overflow",
        )
    })?;
    allocator.copy_to_pages(boot_info_phys, boot_info_blob)?;
    let entry_phys = host_phys
        .checked_add(image.entry_vaddr)
        .ok_or_else(|| CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "elf entry overflow"))?;
    Ok(GuestElfWithBootInfoInstall {
        entry_phys,
        boot_info_phys,
    })
}

/// Installed hypervisor-owned relay measurement page addresses.
#[cfg(feature = "datapath-guest-relay-measurement")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayMeasurementPageInstall {
    /// Host physical base of the measurement page.
    pub host_phys: u64,
    /// Guest physical base mapped via EPT.
    pub guest_phys: u64,
}

/// Allocates and initializes the hypervisor-owned relay measurement counter page.
#[cfg(feature = "datapath-guest-relay-measurement")]
pub fn install_relay_measurement_page<A: PageAllocator>(
    allocator: &mut A,
    guest_phys: u64,
) -> Result<RelayMeasurementPageInstall, CpuSeamError> {
    use core::mem::size_of;

    use hv_datapath::RELAY_MEASUREMENT_PAGE_BYTES;
    use hv_guest_abi::{
        GuestBootInfoRelayMeasurement, GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION,
        GUEST_RELAY_MEASUREMENT_MAGIC,
    };

    if guest_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "relay measurement page guest address must be non-zero",
        ));
    }
    let size = RELAY_MEASUREMENT_PAGE_BYTES as usize;
    let host_phys = allocator.allocate_pages(size, VMXON_REGION_ALIGNMENT_BYTES)?;
    let extension = GuestBootInfoRelayMeasurement {
        magic: GUEST_RELAY_MEASUREMENT_MAGIC,
        version: GUEST_RELAY_MEASUREMENT_EXTENSION_VERSION,
        frames_completed: 0,
        tsc_start: 0,
        tsc_end: 0,
        measurement_page_gpa: guest_phys,
    };
    let mut page = alloc::vec![0u8; size];
    let header_len = size_of::<GuestBootInfoRelayMeasurement>();
    if let Some(dest) = page.get_mut(0..4) {
        dest.copy_from_slice(&extension.magic.to_le_bytes());
    }
    if let Some(dest) = page.get_mut(4..8) {
        dest.copy_from_slice(&extension.version.to_le_bytes());
    }
    if let Some(dest) = page.get_mut(8..16) {
        dest.copy_from_slice(&extension.frames_completed.to_le_bytes());
    }
    if let Some(dest) = page.get_mut(16..24) {
        dest.copy_from_slice(&extension.tsc_start.to_le_bytes());
    }
    if let Some(dest) = page.get_mut(24..32) {
        dest.copy_from_slice(&extension.tsc_end.to_le_bytes());
    }
    if let Some(dest) = page.get_mut(32..header_len) {
        dest.copy_from_slice(&extension.measurement_page_gpa.to_le_bytes());
    }
    allocator.copy_to_pages(host_phys, &page)?;
    Ok(RelayMeasurementPageInstall {
        host_phys,
        guest_phys,
    })
}

/// Installed hypervisor-owned e1000 MMIO emulation state page.
#[cfg(feature = "datapath-guest-relay-measurement")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct E1000MmioStatePageInstall {
    /// Host physical base of the emulated MMIO state page.
    pub host_phys: u64,
}

/// Allocates and initializes a hypervisor-owned e1000 MMIO emulation state page.
#[cfg(feature = "datapath-guest-relay-measurement")]
pub fn install_e1000_mmio_state_page<A: PageAllocator>(
    allocator: &mut A,
) -> Result<E1000MmioStatePageInstall, CpuSeamError> {
    use hv_ept::EPT_PAGE_SIZE_BYTES;

    let size = EPT_PAGE_SIZE_BYTES as usize;
    let host_phys = allocator.allocate_pages(size, VMXON_REGION_ALIGNMENT_BYTES)?;
    let page = alloc::vec![0u8; size];
    allocator.copy_to_pages(host_phys, &page)?;
    Ok(E1000MmioStatePageInstall { host_phys })
}

fn align_up_usize(value: usize, alignment: usize) -> Result<usize, CpuSeamError> {
    if alignment == 0 || alignment & (alignment - 1) != 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "alignment must be a power of two",
        ));
    }
    value
        .checked_add(alignment - 1)
        .map(|v| v & !(alignment - 1))
        .ok_or_else(|| CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "alignment overflow"))
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
        if !self.allocations.iter().any(|(phys, size)| {
            let end = phys.saturating_add(*size as u64);
            host_phys >= *phys && host_phys.saturating_add(bytes.len() as u64) <= end
        }) {
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
    use hv_ept::{EPT_PAGE_SIZE_BYTES, EPT_ROOT_TABLE_BYTES};
    use hv_platform_model::plan_static_platform_ir;
    use hv_vmx::{plan_vmx_init, program_vmxon_region};

    #[test]
    fn install_vmxon_region_rebinds_host_phys() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let region = program_vmxon_region(&plan, REFERENCE_VMXON_REVISION).expect("program");
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
        assert_eq!(hv_ept::count_synthetic_entries(&installed), 0);
    }

    #[test]
    #[cfg(feature = "datapath-guest-relay-measurement")]
    fn install_ept_tables_resolves_measurement_page_mapping() {
        use hv_datapath::RELAY_MEASUREMENT_PAGE_GUEST_PHYS;
        use hv_ept::{append_ept_guest_mapping, resolve_guest_phys_to_host, EPT_PAGE_SIZE_BYTES};

        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
        let mut tables = program_ept_tables(&plan).expect("program");
        append_ept_guest_mapping(
            &mut tables,
            RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
            0x7000,
            EPT_PAGE_SIZE_BYTES,
        )
        .expect("append");
        let mut allocator = MockPageAllocator::new(0x0000_0000_0300_0000);
        let installed = install_ept_tables(&mut allocator, &tables).expect("install");
        let host = resolve_guest_phys_to_host(&installed, RELAY_MEASUREMENT_PAGE_GUEST_PHYS)
            .expect("resolve");
        assert_eq!(host, 0x7000);
    }

    #[test]
    fn install_vmcs_region_allocates_revision_prefix_page() {
        let mut allocator = MockPageAllocator::new(0x0000_0000_0300_0000);
        let vmcs_phys = install_vmcs_region(&mut allocator).expect("install vmcs");
        assert_eq!(vmcs_phys & (EPT_PAGE_SIZE_BYTES - 1), 0);
        assert_eq!(allocator.copies.len(), 1);
    }

    #[test]
    fn resident_mock_allocator_rejects_zero_size() {
        let mut allocator = MockPageAllocator::new(0x1000);
        assert!(allocator
            .allocate_pages(0, VMXON_REGION_ALIGNMENT_BYTES)
            .is_err());
    }

    #[test]
    fn resident_mock_copy_rejects_zero_target() {
        let mut allocator = MockPageAllocator::new(0x1000);
        let phys = allocator
            .allocate_pages(EPT_ROOT_TABLE_BYTES as usize, VMXON_REGION_ALIGNMENT_BYTES)
            .expect("allocate");
        assert!(allocator.copy_to_pages(0, &[0u8; 16]).is_err());
        assert!(allocator.copy_to_pages(phys, &[0u8; 16]).is_ok());
    }

    #[test]
    fn resident_mock_allocator_rejects_invalid_alignment() {
        let mut allocator = MockPageAllocator::new(0x1000);
        assert!(allocator
            .allocate_pages(EPT_ROOT_TABLE_BYTES as usize, 0)
            .is_err());
        assert!(allocator
            .allocate_pages(EPT_ROOT_TABLE_BYTES as usize, 3)
            .is_err());
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

    #[cfg(feature = "datapath-guest-live")]
    #[test]
    fn install_guest_elf_with_boot_info_keeps_boot_info_in_elf_allocation() {
        use hv_config_model::compile_config_from_str;
        use hv_guest_boot::{
            build_guest_boot_info_for_partition, reference_guest_elf_for_kind, GuestElfKind,
        };
        use hv_platform_model::plan_static_platform_ir;

        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let boot_info = build_guest_boot_info_for_partition(&layout, "in").expect("boot info");
        let mut allocator = MockPageAllocator::new(0x0000_0000_0500_0000);
        let elf_bytes = reference_guest_elf_for_kind("in", GuestElfKind::Datapath).expect("elf");
        let install = install_guest_elf_with_boot_info(&mut allocator, elf_bytes, &boot_info)
            .expect("install");
        assert!(install.boot_info_phys > install.entry_phys);
        assert!(allocator
            .copies
            .iter()
            .any(|(phys, len)| { *phys == install.boot_info_phys && *len == boot_info.len() }));
        let elf_allocation = allocator
            .allocations
            .iter()
            .find(|(base, size)| {
                install.entry_phys >= *base
                    && install
                        .boot_info_phys
                        .saturating_add(boot_info.len() as u64)
                        <= base.saturating_add(*size as u64)
            })
            .expect("single allocation covers elf and boot info");
        assert!(elf_allocation.1 >= boot_info.len());
    }
}
