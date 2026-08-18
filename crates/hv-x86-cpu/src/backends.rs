//! CPU seam backends combining structure programming with instruction seams.

use hv_ept::{program_ept_tables, EptBackend, EptInitPlan, EptProgrammedTables, EptError, EptErrorKind};
use hv_vmx::{
    program_vmxon_region, VmxBackend, VmxInitPlan, VmxonProgrammedRegion, VmxError, VmxErrorKind,
    REFERENCE_VMXON_REVISION,
};
use hv_vtd::{
    program_vtd_tables, VtdBackend, VtdInitPlan, VtdProgrammedTables, VtdError, VtdErrorKind,
};

use crate::error::{CpuSeamError, CpuSeamErrorKind};
use crate::seams::{
    run_ept_pointer_cpu_seam, run_vmxon_cpu_seam, run_vtd_enable_cpu_seam, EptCpuSeamOutcome,
    VmxCpuSeamOutcome, VtdCpuSeamOutcome,
};

/// Backend that programs VMXON structures then runs the VMXON CPU seam.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CpuSeamVmxBackend {
    /// Number of successful enable calls.
    pub enable_calls: u32,
    /// Last programmed VMXON region.
    pub last_region: Option<VmxonProgrammedRegion>,
    /// Last VMXON CPU seam outcome.
    pub last_seam: Option<VmxCpuSeamOutcome>,
}

/// Backend that programs EPT structures then runs the EPT pointer CPU seam.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CpuSeamEptBackend {
    /// Number of successful install calls.
    pub install_calls: u32,
    /// Last programmed EPT tables.
    pub last_tables: Option<EptProgrammedTables>,
    /// Last EPT pointer CPU seam outcome.
    pub last_seam: Option<EptCpuSeamOutcome>,
}

/// Backend that programs VT-d structures then runs the VT-d enable CPU seam.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CpuSeamVtdBackend {
    /// Number of successful enable calls.
    pub enable_calls: u32,
    /// Last programmed VT-d tables.
    pub last_tables: Option<VtdProgrammedTables>,
    /// Last VT-d enable CPU seam outcome.
    pub last_seam: Option<VtdCpuSeamOutcome>,
}

impl VmxBackend for CpuSeamVmxBackend {
    fn enable_vmx(&mut self, plan: &VmxInitPlan) -> Result<(), VmxError> {
        let region = program_vmxon_region(plan, REFERENCE_VMXON_REVISION)?;
        let seam = run_vmxon_cpu_seam(&region).map_err(map_cpu_seam_to_vmx)?;
        self.enable_calls = self.enable_calls.saturating_add(1);
        self.last_region = Some(region);
        self.last_seam = Some(seam);
        Ok(())
    }
}

impl EptBackend for CpuSeamEptBackend {
    fn install_ept(&mut self, plan: &EptInitPlan) -> Result<(), EptError> {
        let tables = program_ept_tables(plan)?;
        let seam = run_ept_pointer_cpu_seam(&tables).map_err(map_cpu_seam_to_ept)?;
        self.install_calls = self.install_calls.saturating_add(1);
        self.last_tables = Some(tables);
        self.last_seam = Some(seam);
        Ok(())
    }
}

impl VtdBackend for CpuSeamVtdBackend {
    fn enable_vtd(&mut self, plan: &VtdInitPlan) -> Result<(), VtdError> {
        let tables = program_vtd_tables(plan)?;
        let seam = run_vtd_enable_cpu_seam(&tables).map_err(map_cpu_seam_to_vtd)?;
        self.enable_calls = self.enable_calls.saturating_add(1);
        self.last_tables = Some(tables);
        self.last_seam = Some(seam);
        Ok(())
    }
}

fn map_cpu_seam_to_vmx(err: CpuSeamError) -> VmxError {
    let kind = match err.kind {
        CpuSeamErrorKind::Unavailable | CpuSeamErrorKind::ExecutionFailed => VmxErrorKind::Backend,
        CpuSeamErrorKind::InvalidInput => VmxErrorKind::Planning,
    };
    VmxError::new(kind, err.message)
}

fn map_cpu_seam_to_ept(err: CpuSeamError) -> EptError {
    let kind = match err.kind {
        CpuSeamErrorKind::Unavailable | CpuSeamErrorKind::ExecutionFailed => EptErrorKind::Backend,
        CpuSeamErrorKind::InvalidInput => EptErrorKind::Planning,
    };
    EptError::new(kind, err.message)
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
    use hv_ept::EptErrorKind;
    use hv_platform_model::{plan_static_platform_ir, validate_platform, ValidatedPlatform};
    use hv_vmx::plan_vmx_init;
    use hv_vmx::VmxErrorKind;
    use hv_vtd::plan_vtd_init;
    use hv_vtd::VtdErrorKind;

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
    fn cpu_seam_vmx_backend_programs_and_runs_seam() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let mut backend = CpuSeamVmxBackend::default();
        backend.enable_vmx(&plan).expect("enable");
        assert_eq!(backend.enable_calls, 1);
        assert!(backend.last_region.is_some());
        assert!(backend.last_seam.is_some());
    }

    #[test]
    fn cpu_seam_ept_backend_programs_and_runs_seam() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
        let validated = reference_validated();
        let mut backend = CpuSeamEptBackend::default();
        hv_ept::init_ept(&mut backend, &plan, &validated).expect("init");
        assert_eq!(backend.install_calls, 1);
        assert!(backend.last_tables.is_some());
        assert!(backend.last_seam.is_some());
    }

    #[test]
    fn cpu_seam_vtd_backend_programs_and_runs_seam() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd");
        let validated = reference_validated();
        let mut backend = CpuSeamVtdBackend::default();
        hv_vtd::init_vtd(&mut backend, &plan, &validated).expect("init");
        assert_eq!(backend.enable_calls, 1);
        assert!(backend.last_tables.is_some());
        assert!(backend.last_seam.is_some());
    }

    #[test]
    fn cpu_seam_error_mapping_preserves_messages() {
        let err = CpuSeamError::new(CpuSeamErrorKind::InvalidInput, "bad seam");
        let vmx = map_cpu_seam_to_vmx(err.clone());
        let ept = map_cpu_seam_to_ept(err.clone());
        let vtd = map_cpu_seam_to_vtd(err);
        assert!(vmx.message.contains("bad seam"));
        assert!(ept.message.contains("bad seam"));
        assert!(vtd.message.contains("bad seam"));
    }

    #[test]
    fn cpu_seam_error_mapping_maps_unavailable_to_backend_errors() {
        let err = CpuSeamError::new(CpuSeamErrorKind::Unavailable, "missing");
        assert!(matches!(
            map_cpu_seam_to_vmx(err.clone()).kind,
            VmxErrorKind::Backend
        ));
        assert!(matches!(
            map_cpu_seam_to_ept(err.clone()).kind,
            EptErrorKind::Backend
        ));
        assert!(matches!(
            map_cpu_seam_to_vtd(err).kind,
            VtdErrorKind::Backend
        ));
    }

    #[test]
    fn cpu_seam_error_mapping_maps_execution_failed_to_backend_errors() {
        let err = CpuSeamError::new(CpuSeamErrorKind::ExecutionFailed, "vmxon failed");
        assert!(matches!(
            map_cpu_seam_to_vmx(err.clone()).kind,
            VmxErrorKind::Backend
        ));
        assert!(matches!(
            map_cpu_seam_to_ept(err.clone()).kind,
            EptErrorKind::Backend
        ));
        assert!(matches!(
            map_cpu_seam_to_vtd(err).kind,
            VtdErrorKind::Backend
        ));
    }
}
