//! Gate C initialization after Gate B boot validation.

use hv_boot_abi::RequirementsSnapshot;
use hv_config_model::{FeatureRequirement, PlatformRequirements};
use hv_ept::{
    ept_init_required, init_ept, plan_ept_init, EptBackend, EptInitPlan, EptError, MockEptBackend,
};
use hv_platform_model::{PlatformWarning, StaticPlatformIR, ValidatedPlatform};
use hv_types::SHA256_DIGEST_BYTES;
use hv_vmx::{
    init_vmx, plan_vmx_init, vmx_init_required, MockVmxBackend, VmxBackend, VmxError, VmxInitPlan,
};
use hv_vtd::{
    init_vtd, plan_vtd_init, vtd_init_required, MockVtdBackend, VtdBackend, VtdError, VtdInitPlan,
};

use crate::boot::boot_check;
use crate::error::{BootCheckError, BootCheckErrorKind};
use crate::snapshot::platform_requirements_from_snapshot;
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
    init_vmx_if_required(
        &mut vmx_backend,
        &vmx_plan,
        validated,
        requirements.vmx,
    )?;
    let mut ept_backend = MockEptBackend::default();
    init_ept_if_required(
        &mut ept_backend,
        &ept_plan,
        validated,
        requirements.ept,
    )?;
    let mut vtd_backend = MockVtdBackend::default();
    init_vtd_if_required(
        &mut vtd_backend,
        &vtd_plan,
        validated,
        requirements.vtd,
    )?;

    Ok(GateCInitResult {
        validated: validated.clone(),
        warnings,
        vmx_plan,
        ept_plan,
        vtd_plan,
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
