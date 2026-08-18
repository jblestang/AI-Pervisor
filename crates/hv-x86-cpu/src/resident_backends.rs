//! REAL_HW CPU seam backends that install structures into host physical pages.

use hv_ept::{program_ept_tables, EptBackend, EptInitPlan, EptProgrammedTables, EptError, EptErrorKind};
use hv_vmx::{
    program_vmxon_region, VmxBackend, VmxInitPlan, VmxonProgrammedRegion, VmxError, VmxErrorKind,
};
use hv_vtd::{
    program_vtd_tables, VtdBackend, VtdInitPlan, VtdProgrammedTables, VtdError, VtdErrorKind,
};

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::resident::{
    install_ept_tables, install_vmxon_region, install_vmcs_region, resolve_vmxon_revision,
    PageAllocator,
};
use crate::seams::{
    run_ept_pointer_cpu_seam, run_vmxon_cpu_seam, run_vtd_enable_cpu_seam, EptCpuSeamOutcome,
    VmxCpuSeamOutcome, VtdCpuSeamOutcome,
};

/// REAL_HW VMX backend: install VMXON region then run the CPU seam.
pub struct ResidentCpuSeamVmxBackend<'a, A: PageAllocator> {
    allocator: &'a mut A,
    /// Number of successful enable calls.
    pub enable_calls: u32,
    /// Last installed VMXON region.
    pub last_region: Option<VmxonProgrammedRegion>,
    /// Last VMXON CPU seam outcome.
    pub last_seam: Option<VmxCpuSeamOutcome>,
}

impl<'a, A: PageAllocator> ResidentCpuSeamVmxBackend<'a, A> {
    /// Creates a resident VMX backend bound to the given page allocator.
    pub fn new(allocator: &'a mut A) -> Self {
        Self {
            allocator,
            enable_calls: 0,
            last_region: None,
            last_seam: None,
        }
    }
}

impl<A: PageAllocator> VmxBackend for ResidentCpuSeamVmxBackend<'_, A> {
    fn enable_vmx(&mut self, plan: &VmxInitPlan) -> Result<(), VmxError> {
        let revision = resolve_vmxon_revision();
        let programmed = program_vmxon_region(plan, revision)?;
        let installed = install_vmxon_region(self.allocator, &programmed).map_err(map_resident_to_vmx)?;
        let seam = run_vmxon_cpu_seam(&installed).map_err(map_cpu_seam_to_vmx)?;
        self.enable_calls = self.enable_calls.saturating_add(1);
        self.last_region = Some(installed);
        self.last_seam = Some(seam);
        Ok(())
    }
}

/// REAL_HW EPT backend: install tables and VMCS page then run the CPU seam.
pub struct ResidentCpuSeamEptBackend<'a, A: PageAllocator> {
    allocator: &'a mut A,
    /// Number of successful install calls.
    pub install_calls: u32,
    /// Last installed EPT tables.
    pub last_tables: Option<EptProgrammedTables>,
    /// Last installed VMCS region physical address.
    pub last_vmcs_phys: Option<u64>,
    /// Last EPT pointer CPU seam outcome.
    pub last_seam: Option<EptCpuSeamOutcome>,
}

impl<'a, A: PageAllocator> ResidentCpuSeamEptBackend<'a, A> {
    /// Creates a resident EPT backend bound to the given page allocator.
    pub fn new(allocator: &'a mut A) -> Self {
        Self {
            allocator,
            install_calls: 0,
            last_tables: None,
            last_vmcs_phys: None,
            last_seam: None,
        }
    }
}

impl<A: PageAllocator> EptBackend for ResidentCpuSeamEptBackend<'_, A> {
    fn install_ept(&mut self, plan: &EptInitPlan) -> Result<(), EptError> {
        let programmed = program_ept_tables(plan)?;
        let installed = install_ept_tables(self.allocator, &programmed).map_err(map_resident_to_ept)?;
        let vmcs_phys =
            install_vmcs_region(self.allocator).map_err(map_resident_to_ept)?;
        let seam = run_ept_pointer_cpu_seam(&installed, Some(vmcs_phys)).map_err(map_cpu_seam_to_ept)?;
        self.install_calls = self.install_calls.saturating_add(1);
        self.last_tables = Some(installed);
        self.last_vmcs_phys = Some(vmcs_phys);
        self.last_seam = Some(seam);
        Ok(())
    }
}

/// REAL_HW VT-d backend: programs tables then runs the CPU seam (intent only).
#[derive(Default)]
pub struct ResidentCpuSeamVtdBackend {
    /// Number of successful enable calls.
    pub enable_calls: u32,
    /// Last programmed VT-d tables.
    pub last_tables: Option<VtdProgrammedTables>,
    /// Last VT-d enable CPU seam outcome.
    pub last_seam: Option<VtdCpuSeamOutcome>,
}

impl VtdBackend for ResidentCpuSeamVtdBackend {
    fn enable_vtd(&mut self, plan: &VtdInitPlan) -> Result<(), VtdError> {
        let tables = program_vtd_tables(plan)?;
        let seam = run_vtd_enable_cpu_seam(&tables).map_err(map_cpu_seam_to_vtd)?;
        self.enable_calls = self.enable_calls.saturating_add(1);
        self.last_tables = Some(tables);
        self.last_seam = Some(seam);
        Ok(())
    }
}

fn map_resident_to_vmx(err: CpuSeamError) -> VmxError {
    let kind = match err.kind {
        CpuSeamErrorKind::Unavailable | CpuSeamErrorKind::ExecutionFailed => VmxErrorKind::Backend,
        CpuSeamErrorKind::InvalidInput => VmxErrorKind::Planning,
    };
    VmxError::new(kind, err.message)
}

fn map_resident_to_ept(err: CpuSeamError) -> EptError {
    let kind = match err.kind {
        CpuSeamErrorKind::Unavailable | CpuSeamErrorKind::ExecutionFailed => EptErrorKind::Backend,
        CpuSeamErrorKind::InvalidInput => EptErrorKind::Planning,
    };
    EptError::new(kind, err.message)
}

fn map_cpu_seam_to_vmx(err: CpuSeamError) -> VmxError {
    map_resident_to_vmx(err)
}

fn map_cpu_seam_to_ept(err: CpuSeamError) -> EptError {
    map_resident_to_ept(err)
}

fn map_cpu_seam_to_vtd(err: CpuSeamError) -> VtdError {
    let kind = match err.kind {
        CpuSeamErrorKind::Unavailable | CpuSeamErrorKind::ExecutionFailed => VtdErrorKind::Backend,
        CpuSeamErrorKind::InvalidInput => VtdErrorKind::Planning,
    };
    VtdError::new(kind, err.message)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_ept::plan_ept_init;
    use hv_platform_model::{plan_static_platform_ir, validate_platform, ValidatedPlatform};
    use hv_vmx::plan_vmx_init;
    use hv_vtd::plan_vtd_init;
    use crate::resident::{install_ept_tables, MockPageAllocator};
    use crate::error::CpuSeamError;

    fn reference_validated() -> ValidatedPlatform {
        let observed =
            include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let observed = hv_platform_model::parse_observed_platform_json(observed).expect("parse");
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        validate_platform(&compiled.requirements, &observed)
            .expect("validate")
            .0
    }

    #[test]
    fn resident_vmx_backend_installs_and_runs_seam() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let mut allocator = MockPageAllocator::new(0x0000_0000_0400_0000);
        let mut backend = ResidentCpuSeamVmxBackend::new(&mut allocator);
        backend.enable_vmx(&plan).expect("enable");
        assert_eq!(backend.enable_calls, 1);
        assert!(backend.last_region.is_some());
        assert!(backend.last_seam.is_some());
    }

    #[test]
    fn resident_ept_backend_installs_vmcs_and_runs_seam() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
        let validated = reference_validated();
        let mut allocator = MockPageAllocator::new(0x0000_0000_0500_0000);
        let mut backend = ResidentCpuSeamEptBackend::new(&mut allocator);
        hv_ept::init_ept(&mut backend, &plan, &validated).expect("init");
        assert_eq!(backend.install_calls, 1);
        assert!(backend.last_vmcs_phys.is_some());
        assert!(backend.last_seam.is_some());
    }

    #[test]
    fn resident_vtd_backend_runs_seam() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd");
        let validated = reference_validated();
        let mut backend = ResidentCpuSeamVtdBackend::default();
        hv_vtd::init_vtd(&mut backend, &plan, &validated).expect("init");
        assert_eq!(backend.enable_calls, 1);
        assert!(backend.last_seam.is_some());
    }

    #[test]
    fn resident_error_mapping_preserves_messages() {
        let err = CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "bad resident");
        assert!(map_resident_to_vmx(err.clone()).message.contains("bad resident"));
        assert!(map_resident_to_ept(err).message.contains("bad resident"));
    }

    #[test]
    fn resident_error_mapping_covers_backend_kinds() {
        let unavailable = CpuSeamError::new(CpuSeamErrorKind::Unavailable, "unavailable");
        let execution = CpuSeamError::new(CpuSeamErrorKind::ExecutionFailed, "execution failed");
        let invalid = CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "bad input");
        assert!(map_resident_to_vmx(unavailable.clone()).message.contains("unavailable"));
        assert!(map_resident_to_ept(execution.clone()).message.contains("execution failed"));
        assert!(map_cpu_seam_to_vmx(invalid.clone()).message.contains("bad input"));
        assert!(map_cpu_seam_to_ept(unavailable.clone()).message.contains("unavailable"));
        assert!(map_cpu_seam_to_vtd(unavailable).message.contains("unavailable"));
        assert!(map_cpu_seam_to_vtd(execution).message.contains("execution failed"));
    }

    #[test]
    fn install_ept_tables_rejects_empty_root_table() {
        use hv_ept::{EptProgrammedTables, EPT_PAGE_SIZE_BYTES};
        let tables = EptProgrammedTables {
            root_table_phys: 0x2000,
            root_table: alloc::vec::Vec::new(),
            mappings: alloc::vec![hv_ept::EptProgrammedMapping {
                guest_phys: 0,
                host_phys: 0,
                size_bytes: EPT_PAGE_SIZE_BYTES,
                encoded_entry: 1,
            }],
        };
        let mut allocator = MockPageAllocator::new(0x1000);
        assert!(install_ept_tables(&mut allocator, &tables).is_err());
    }
}
