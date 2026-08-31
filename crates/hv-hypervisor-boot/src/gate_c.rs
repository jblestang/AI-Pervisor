//! Gate C initialization after Gate B boot validation.

use hv_boot_abi::{LayoutSnapshot, RequirementsSnapshot};
use hv_config_model::{FeatureRequirement, PlatformRequirements};
use hv_ept::{
    ept_init_required, init_ept, plan_ept_init, EptBackend, EptError, EptInitPlan,
    EptProgrammedTables, MockEptBackend, ProgrammingEptBackend,
};
use hv_platform_model::{PlatformWarning, StaticPlatformIR, ValidatedPlatform};
use hv_types::SHA256_DIGEST_BYTES;
use hv_vmx::{
    init_vmx, plan_vmx_init, vmx_init_required, MockVmxBackend, ProgrammingVmxBackend, VmxBackend,
    VmxError, VmxInitPlan, VmxonProgrammedRegion,
};
use hv_vtd::{
    init_vtd, plan_vtd_init, vtd_init_required, MockVtdBackend, ProgrammingVtdBackend, VtdBackend,
    VtdError, VtdInitPlan, VtdProgrammedTables,
};

#[cfg(feature = "vmx-launch")]
use hv_guest_boot::{build_guest_boot_info_for_partition, GUEST_SMOKE_IMAGE};
#[cfg(feature = "vmx-launch")]
use hv_vmx::{
    patch_guest_entry_in_fields, plan_vmx_launch, program_vmcs_fields,
    DEFAULT_SMOKE_GUEST_PARTITION_ID,
};
#[cfg(feature = "live-execution")]
use hv_x86_cpu::live_execution_environment_ready;
#[cfg(feature = "real-hw-execution")]
use hv_x86_cpu::{
    install_guest_image, PageAllocator, ResidentCpuSeamEptBackend, ResidentCpuSeamVmxBackend,
    ResidentCpuSeamVtdBackend,
};
#[cfg(feature = "vmx-launch")]
use hv_x86_cpu::{run_vmx_launch_cpu_seam, CpuSeamError, VmxLaunchCpuSeamOutcome};
#[cfg(feature = "cpu-seams")]
use hv_x86_cpu::{
    CpuSeamEptBackend, CpuSeamVmxBackend, CpuSeamVtdBackend, EptCpuSeamOutcome, VmxCpuSeamOutcome,
    VtdCpuSeamOutcome,
};

use crate::boot::boot_check;
use crate::error::{BootCheckError, BootCheckErrorKind};
use crate::snapshot::{
    platform_requirements_from_snapshot, static_platform_ir_from_layout_snapshot,
};
use crate::transfer::boot_from_transfer;

/// Result of Gate B validation followed by Gate C planning and mock-backed init.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCInitResult {
    /// Validated platform snapshot.
    pub validated: ValidatedPlatform,
    /// Non-fatal platform warnings from validation.
    pub warnings: alloc::vec::Vec<PlatformWarning>,
    /// VMX init plan derived from static layout metadata.
    pub vmx_plan: VmxInitPlan,
    /// EPT init plan derived from static layout metadata.
    pub ept_plan: EptInitPlan,
    /// VT-d init plan derived from static layout metadata.
    pub vtd_plan: VtdInitPlan,
}

/// Result of Gate C init using hardware programming backends (structure encoding, no CPU instructions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCProgrammingResult {
    /// Shared Gate C init output (plans + validated platform).
    pub init: GateCInitResult,
    /// Programmed VMXON region when VMX init ran.
    pub vmxon_region: Option<VmxonProgrammedRegion>,
    /// Programmed EPT tables when EPT init ran.
    pub ept_tables: Option<EptProgrammedTables>,
    /// Programmed VT-d tables when VT-d init ran.
    pub vtd_tables: Option<VtdProgrammedTables>,
}

/// Result of Gate C init using CPU instruction seams after structure programming.
#[cfg(feature = "cpu-seams")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCCpuSeamResult {
    /// Shared Gate C programming output (plans + programmed structures).
    pub programming: GateCProgrammingResult,
    /// VMXON CPU seam outcome when VMX init ran.
    pub vmx_seam: Option<VmxCpuSeamOutcome>,
    /// EPT pointer CPU seam outcome when EPT init ran.
    pub ept_seam: Option<EptCpuSeamOutcome>,
    /// VT-d enable CPU seam outcome when VT-d init ran.
    pub vtd_seam: Option<VtdCpuSeamOutcome>,
}

/// Runs transfer boot checks and Gate C CPU seam init using snapshot + layout metadata.
#[cfg(feature = "cpu-seams")]
pub fn boot_from_transfer_and_init_gate_c_cpu_seam(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
) -> Result<GateCCpuSeamResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_c_cpu_seam_from_validated(&requirements, layout, &validated, warnings)
}

/// Runs transfer boot checks and Gate C CPU seam init using embedded snapshots.
#[cfg(feature = "cpu-seams")]
pub fn boot_from_transfer_and_init_gate_c_cpu_seam_from_snapshots(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
) -> Result<GateCCpuSeamResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) = boot_from_transfer(
        transfer,
        &requirements.config_digest,
        &platform_requirements,
    )?;
    init_gate_c_cpu_seam_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
    )
}

/// Runs boot checks from raw inputs and Gate C CPU seam init.
#[cfg(feature = "cpu-seams")]
pub fn boot_check_and_init_gate_c_cpu_seam(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
) -> Result<GateCCpuSeamResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_c_cpu_seam_from_validated(requirements, layout, &validated, warnings)
}

/// Result of Gate C init with live privileged instruction execution enabled.
#[cfg(feature = "live-execution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCLiveExecutionResult {
    /// CPU seam output including programmed structures and seam dispositions.
    pub cpu_seam: GateCCpuSeamResult,
    /// Whether the host runtime environment permitted live instruction execution.
    pub live_environment_ready: bool,
}

/// Runs transfer boot checks and Gate C live execution init using snapshot + layout metadata.
#[cfg(feature = "live-execution")]
pub fn boot_from_transfer_and_init_gate_c_live_execution(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
) -> Result<GateCLiveExecutionResult, BootCheckError> {
    let cpu_seam = boot_from_transfer_and_init_gate_c_cpu_seam(transfer, snapshot, layout)?;
    Ok(wrap_gate_c_live_execution(cpu_seam))
}

/// Runs transfer boot checks and Gate C live execution init using embedded snapshots.
#[cfg(feature = "live-execution")]
pub fn boot_from_transfer_and_init_gate_c_live_execution_from_snapshots(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
) -> Result<GateCLiveExecutionResult, BootCheckError> {
    let cpu_seam =
        boot_from_transfer_and_init_gate_c_cpu_seam_from_snapshots(transfer, requirements, layout)?;
    Ok(wrap_gate_c_live_execution(cpu_seam))
}

/// Runs boot checks from raw inputs and Gate C live execution init.
#[cfg(feature = "live-execution")]
pub fn boot_check_and_init_gate_c_live_execution(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
) -> Result<GateCLiveExecutionResult, BootCheckError> {
    let cpu_seam = boot_check_and_init_gate_c_cpu_seam(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
        layout,
    )?;
    Ok(wrap_gate_c_live_execution(cpu_seam))
}

#[cfg(feature = "live-execution")]
fn wrap_gate_c_live_execution(cpu_seam: GateCCpuSeamResult) -> GateCLiveExecutionResult {
    GateCLiveExecutionResult {
        live_environment_ready: live_execution_environment_ready(),
        cpu_seam,
    }
}

/// Result of Gate C init with REAL_HW resident page installation and live execution.
#[cfg(feature = "real-hw-execution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCRealHwResult {
    /// Live execution output including CPU seam dispositions.
    pub live: GateCLiveExecutionResult,
    /// Host physical address of the installed VMCS region when EPT init ran.
    pub vmcs_phys: Option<u64>,
}

/// Runs transfer boot checks and REAL_HW Gate C init using embedded snapshots.
#[cfg(feature = "real-hw-execution")]
pub fn boot_from_transfer_and_init_gate_c_real_hw_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateCRealHwResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) = boot_from_transfer(
        transfer,
        &requirements.config_digest,
        &platform_requirements,
    )?;
    init_gate_c_real_hw_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and REAL_HW Gate C init using snapshot + layout metadata.
#[cfg(feature = "real-hw-execution")]
pub fn boot_from_transfer_and_init_gate_c_real_hw<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateCRealHwResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_c_real_hw_from_validated(&requirements, layout, &validated, warnings, allocator)
}

/// Runs boot checks from raw inputs and REAL_HW Gate C init.
#[cfg(feature = "real-hw-execution")]
pub fn boot_check_and_init_gate_c_real_hw<A: PageAllocator>(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateCRealHwResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_c_real_hw_from_validated(requirements, layout, &validated, warnings, allocator)
}

#[cfg(feature = "real-hw-execution")]
fn init_gate_c_real_hw_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateCRealHwResult, BootCheckError> {
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).map_err(map_vmx_error)?;
    let ept_plan = plan_ept_init(layout, &vmx_plan).map_err(map_ept_error)?;
    let interrupt_remapping = matches!(
        requirements.interrupt_remapping,
        FeatureRequirement::Required | FeatureRequirement::Preferred
    );
    let vtd_plan = plan_vtd_init(layout, interrupt_remapping).map_err(map_vtd_error)?;

    let (vmxon_region, vmx_seam) = {
        let mut vmx_backend = ResidentCpuSeamVmxBackend::new(allocator);
        init_vmx_if_required(&mut vmx_backend, &vmx_plan, validated, requirements.vmx)?;
        (vmx_backend.last_region, vmx_backend.last_seam)
    };

    let (ept_tables, ept_seam, vmcs_phys) = {
        let mut ept_backend = ResidentCpuSeamEptBackend::new(allocator);
        init_ept_if_required(&mut ept_backend, &ept_plan, validated, requirements.ept)?;
        (
            ept_backend.last_tables,
            ept_backend.last_seam,
            ept_backend.last_vmcs_phys,
        )
    };

    let (vtd_tables, vtd_seam) = {
        let mut vtd_backend = ResidentCpuSeamVtdBackend::default();
        init_vtd_if_required(&mut vtd_backend, &vtd_plan, validated, requirements.vtd)?;
        (vtd_backend.last_tables, vtd_backend.last_seam)
    };

    let cpu_seam = GateCCpuSeamResult {
        programming: GateCProgrammingResult {
            init: GateCInitResult {
                validated: validated.clone(),
                warnings,
                vmx_plan,
                ept_plan,
                vtd_plan,
            },
            vmxon_region,
            ept_tables,
            vtd_tables,
        },
        vmx_seam,
        ept_seam,
        vtd_seam,
    };

    Ok(GateCRealHwResult {
        live: wrap_gate_c_live_execution(cpu_seam),
        vmcs_phys,
    })
}

/// Result of Gate C init with REAL_HW resident install and VMX launch bring-up.
#[cfg(feature = "vmx-launch")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCVmxLaunchResult {
    /// REAL_HW Gate C init output including CPU seam dispositions.
    pub real_hw: GateCRealHwResult,
    /// VMX launch CPU seam outcome when launch ran.
    pub launch_seam: Option<VmxLaunchCpuSeamOutcome>,
    /// Host physical guest entry after smoke image installation.
    pub guest_entry_phys: Option<u64>,
    /// Built guest boot info bytes for the smoke partition.
    pub guest_boot_info: Option<alloc::vec::Vec<u8>>,
}

/// Runs transfer boot checks and VMX launch Gate C init using embedded snapshots.
#[cfg(feature = "vmx-launch")]
pub fn boot_from_transfer_and_init_gate_c_vmx_launch_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateCVmxLaunchResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) = boot_from_transfer(
        transfer,
        &requirements.config_digest,
        &platform_requirements,
    )?;
    init_gate_c_vmx_launch_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and VMX launch Gate C init using snapshot + layout metadata.
#[cfg(feature = "vmx-launch")]
pub fn boot_from_transfer_and_init_gate_c_vmx_launch<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateCVmxLaunchResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_c_vmx_launch_from_validated(&requirements, layout, &validated, warnings, allocator)
}

/// Runs boot checks from raw inputs and VMX launch Gate C init.
#[cfg(feature = "vmx-launch")]
pub fn boot_check_and_init_gate_c_vmx_launch<A: PageAllocator>(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateCVmxLaunchResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_c_vmx_launch_from_validated(requirements, layout, &validated, warnings, allocator)
}

#[cfg(feature = "vmx-launch")]
pub(crate) fn init_gate_c_vmx_launch_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateCVmxLaunchResult, BootCheckError> {
    let real_hw =
        init_gate_c_real_hw_from_validated(requirements, layout, validated, warnings, allocator)?;
    let vmcs_phys = real_hw.vmcs_phys.ok_or_else(|| {
        BootCheckError::new(
            BootCheckErrorKind::Platform,
            "VMX launch requires an installed VMCS region from REAL_HW EPT init",
        )
    })?;
    let guest_boot_info =
        build_guest_boot_info_for_partition(layout, DEFAULT_SMOKE_GUEST_PARTITION_ID)
            .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;
    let guest_entry_phys =
        install_guest_image(allocator, GUEST_SMOKE_IMAGE).map_err(map_cpu_seam_error)?;
    let launch_plan = plan_vmx_launch(
        layout,
        &real_hw.live.cpu_seam.programming.init.vmx_plan,
        DEFAULT_SMOKE_GUEST_PARTITION_ID,
    )
    .map_err(map_vmx_error)?;
    let mut vmcs_fields = program_vmcs_fields(&launch_plan);
    patch_guest_entry_in_fields(
        &mut vmcs_fields,
        guest_entry_phys,
        launch_plan.guest_stack_phys.raw(),
    );
    let launch_seam = Some(
        run_vmx_launch_cpu_seam(vmcs_phys, &vmcs_fields, launch_plan.vm_id)
            .map_err(map_cpu_seam_error)?,
    );
    Ok(GateCVmxLaunchResult {
        real_hw,
        launch_seam,
        guest_entry_phys: Some(guest_entry_phys),
        guest_boot_info: Some(guest_boot_info),
    })
}

#[cfg(feature = "vmx-launch")]
fn map_cpu_seam_error(err: CpuSeamError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::Platform, err.message)
}

/// Runs transfer boot checks and mock-backed Gate C init using embedded snapshots.
pub fn boot_from_transfer_and_init_gate_c_from_snapshots(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
) -> Result<GateCInitResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) = boot_from_transfer(
        transfer,
        &requirements.config_digest,
        &platform_requirements,
    )?;
    init_gate_c_from_validated(&platform_requirements, &static_layout, &validated, warnings)
}

/// Runs transfer boot checks and Gate C hardware programming init using snapshot + layout metadata.
pub fn boot_from_transfer_and_init_gate_c_programming(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
) -> Result<GateCProgrammingResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_c_programming_from_validated(&requirements, layout, &validated, warnings)
}

/// Runs transfer boot checks and Gate C hardware programming init using embedded snapshots.
pub fn boot_from_transfer_and_init_gate_c_programming_from_snapshots(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
) -> Result<GateCProgrammingResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) = boot_from_transfer(
        transfer,
        &requirements.config_digest,
        &platform_requirements,
    )?;
    init_gate_c_programming_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
    )
}

/// Runs boot checks from raw inputs and Gate C hardware programming init.
pub fn boot_check_and_init_gate_c_programming(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
) -> Result<GateCProgrammingResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_c_programming_from_validated(requirements, layout, &validated, warnings)
}

/// Runs transfer boot checks and mock-backed Gate C init using snapshot + layout metadata.
pub fn boot_from_transfer_and_init_gate_c(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
) -> Result<GateCInitResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_c_from_validated(&requirements, layout, &validated, warnings)
}

/// Runs boot checks from raw inputs and mock-backed Gate C init.
pub fn boot_check_and_init_gate_c(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
) -> Result<GateCInitResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_c_from_validated(requirements, layout, &validated, warnings)
}

fn init_gate_c_from_validated(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
) -> Result<GateCInitResult, BootCheckError> {
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).map_err(map_vmx_error)?;
    let ept_plan = plan_ept_init(layout, &vmx_plan).map_err(map_ept_error)?;
    let interrupt_remapping = matches!(
        requirements.interrupt_remapping,
        FeatureRequirement::Required | FeatureRequirement::Preferred
    );
    let vtd_plan = plan_vtd_init(layout, interrupt_remapping).map_err(map_vtd_error)?;

    let mut vmx_backend = MockVmxBackend::default();
    init_vmx_if_required(&mut vmx_backend, &vmx_plan, validated, requirements.vmx)?;
    let mut ept_backend = MockEptBackend::default();
    init_ept_if_required(&mut ept_backend, &ept_plan, validated, requirements.ept)?;
    let mut vtd_backend = MockVtdBackend::default();
    init_vtd_if_required(&mut vtd_backend, &vtd_plan, validated, requirements.vtd)?;

    Ok(GateCInitResult {
        validated: validated.clone(),
        warnings,
        vmx_plan,
        ept_plan,
        vtd_plan,
    })
}

fn init_gate_c_programming_from_validated(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
) -> Result<GateCProgrammingResult, BootCheckError> {
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).map_err(map_vmx_error)?;
    let ept_plan = plan_ept_init(layout, &vmx_plan).map_err(map_ept_error)?;
    let interrupt_remapping = matches!(
        requirements.interrupt_remapping,
        FeatureRequirement::Required | FeatureRequirement::Preferred
    );
    let vtd_plan = plan_vtd_init(layout, interrupt_remapping).map_err(map_vtd_error)?;

    let mut vmx_backend = ProgrammingVmxBackend::default();
    init_vmx_if_required(&mut vmx_backend, &vmx_plan, validated, requirements.vmx)?;
    let mut ept_backend = ProgrammingEptBackend::default();
    init_ept_if_required(&mut ept_backend, &ept_plan, validated, requirements.ept)?;
    let mut vtd_backend = ProgrammingVtdBackend::default();
    init_vtd_if_required(&mut vtd_backend, &vtd_plan, validated, requirements.vtd)?;

    Ok(GateCProgrammingResult {
        init: GateCInitResult {
            validated: validated.clone(),
            warnings,
            vmx_plan,
            ept_plan,
            vtd_plan,
        },
        vmxon_region: vmx_backend.last_region,
        ept_tables: ept_backend.last_tables,
        vtd_tables: vtd_backend.last_tables,
    })
}

#[cfg(feature = "cpu-seams")]
fn init_gate_c_cpu_seam_from_validated(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
) -> Result<GateCCpuSeamResult, BootCheckError> {
    let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).map_err(map_vmx_error)?;
    let ept_plan = plan_ept_init(layout, &vmx_plan).map_err(map_ept_error)?;
    let interrupt_remapping = matches!(
        requirements.interrupt_remapping,
        FeatureRequirement::Required | FeatureRequirement::Preferred
    );
    let vtd_plan = plan_vtd_init(layout, interrupt_remapping).map_err(map_vtd_error)?;

    let mut vmx_backend = CpuSeamVmxBackend::default();
    init_vmx_if_required(&mut vmx_backend, &vmx_plan, validated, requirements.vmx)?;
    let mut ept_backend = CpuSeamEptBackend::default();
    init_ept_if_required(&mut ept_backend, &ept_plan, validated, requirements.ept)?;
    let mut vtd_backend = CpuSeamVtdBackend::default();
    init_vtd_if_required(&mut vtd_backend, &vtd_plan, validated, requirements.vtd)?;

    Ok(GateCCpuSeamResult {
        programming: GateCProgrammingResult {
            init: GateCInitResult {
                validated: validated.clone(),
                warnings,
                vmx_plan,
                ept_plan,
                vtd_plan,
            },
            vmxon_region: vmx_backend.last_region,
            ept_tables: ept_backend.last_tables,
            vtd_tables: vtd_backend.last_tables,
        },
        vmx_seam: vmx_backend.last_seam,
        ept_seam: ept_backend.last_seam,
        vtd_seam: vtd_backend.last_seam,
    })
}

fn init_vmx_if_required<B: VmxBackend>(
    backend: &mut B,
    plan: &VmxInitPlan,
    validated: &ValidatedPlatform,
    requirement: FeatureRequirement,
) -> Result<(), BootCheckError> {
    if vmx_init_required(requirement) {
        init_vmx(backend, plan, validated).map_err(map_vmx_error)?;
    }
    Ok(())
}

fn init_ept_if_required<B: EptBackend>(
    backend: &mut B,
    plan: &EptInitPlan,
    validated: &ValidatedPlatform,
    requirement: FeatureRequirement,
) -> Result<(), BootCheckError> {
    if ept_init_required(requirement) {
        init_ept(backend, plan, validated).map_err(map_ept_error)?;
    }
    Ok(())
}

fn init_vtd_if_required<B: VtdBackend>(
    backend: &mut B,
    plan: &VtdInitPlan,
    validated: &ValidatedPlatform,
    requirement: FeatureRequirement,
) -> Result<(), BootCheckError> {
    if vtd_init_required(requirement) {
        init_vtd(backend, plan, validated).map_err(map_vtd_error)?;
    }
    Ok(())
}

fn map_vmx_error(err: VmxError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::Platform, err.message)
}

fn map_ept_error(err: EptError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::Platform, err.message)
}

fn map_vtd_error(err: VtdError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::Platform, err.message)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod gate_c_map_tests {
    use super::*;
    use hv_ept::EptErrorKind;
    use hv_vmx::VmxErrorKind;
    use hv_vtd::VtdErrorKind;

    #[test]
    fn map_error_helpers_wrap_platform_failures() {
        let vmx = map_vmx_error(VmxError::new(VmxErrorKind::Backend, "vmx backend"));
        let ept = map_ept_error(EptError::new(EptErrorKind::Planning, "ept plan"));
        let vtd = map_vtd_error(VtdError::new(VtdErrorKind::Backend, "vtd backend"));
        assert_eq!(vmx.kind, BootCheckErrorKind::Platform);
        assert_eq!(ept.kind, BootCheckErrorKind::Platform);
        assert_eq!(vtd.kind, BootCheckErrorKind::Platform);
        assert!(vmx.message.contains("vmx backend"));
        assert!(ept.message.contains("ept plan"));
        assert!(vtd.message.contains("vtd backend"));
    }

    #[test]
    fn init_vmx_if_required_skips_when_feature_optional() {
        use hv_config_model::compile_config_from_str;
        use hv_config_model::FeatureRequirement;
        use hv_ept::{plan_ept_init, MockEptBackend};
        use hv_platform_model::{plan_static_platform_ir, validate_platform};
        use hv_vmx::{plan_vmx_init, MockVmxBackend};
        use hv_vtd::{plan_vtd_init, MockVtdBackend};

        let observed =
            include_str!("../../hv-platform-model/tests/fixtures/observed/qemu_reference.json");
        let observed = hv_platform_model::parse_observed_platform_json(observed).expect("parse");
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let validated = validate_platform(&compiled.requirements, &observed)
            .expect("validate")
            .0;
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let ept_plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
        let vtd_plan = plan_vtd_init(&layout, true).expect("vtd");
        let mut vmx_backend = MockVmxBackend::default();
        init_vmx_if_required(
            &mut vmx_backend,
            &vmx_plan,
            &validated,
            FeatureRequirement::Optional,
        )
        .expect("skip vmx");
        assert_eq!(vmx_backend.enable_calls, 0);
        let mut ept_backend = MockEptBackend::default();
        init_ept_if_required(
            &mut ept_backend,
            &ept_plan,
            &validated,
            FeatureRequirement::Optional,
        )
        .expect("skip ept");
        assert_eq!(ept_backend.install_calls, 0);
        let mut vtd_backend = MockVtdBackend::default();
        init_vtd_if_required(
            &mut vtd_backend,
            &vtd_plan,
            &validated,
            FeatureRequirement::Optional,
        )
        .expect("skip vtd");
        assert_eq!(vtd_backend.enable_calls, 0);
    }
}
