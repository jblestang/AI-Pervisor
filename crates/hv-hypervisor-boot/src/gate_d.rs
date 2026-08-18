//! Gate D datapath foundation orchestration atop Gate C VMX launch.

use hv_boot_abi::{LayoutSnapshot, RequirementsSnapshot};
use hv_config_model::PlatformRequirements;
use hv_datapath::{plan_datapath_for_vm_id, DatapathPartitionPlan};
use hv_guest_boot::{
    build_guest_boot_infos_all_partitions, GuestBootInfoView,
};
use hv_platform_model::{PlatformWarning, StaticPlatformIR, ValidatedPlatform};
use hv_types::{SHA256_DIGEST_BYTES, VmId};
use hv_x86_cpu::PageAllocator;

use crate::boot::boot_check;
use crate::error::{BootCheckError, BootCheckErrorKind};
use crate::gate_c::{init_gate_c_vmx_launch_from_validated, GateCVmxLaunchResult};
use crate::snapshot::{
    platform_requirements_from_snapshot, static_platform_ir_from_layout_snapshot,
};
use crate::transfer::boot_from_transfer;

/// Result of Gate D datapath foundation init atop VMX launch.
#[cfg(feature = "datapath-foundation")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDDatapathFoundationResult {
    /// VMX launch Gate C output including smoke guest install.
    pub vmx_launch: GateCVmxLaunchResult,
    /// Guest boot info blobs keyed by VM id.
    pub partition_boot_infos: alloc::vec::Vec<(VmId, alloc::vec::Vec<u8>)>,
    /// Datapath plans keyed by VM id.
    pub datapath_plans: alloc::vec::Vec<(VmId, DatapathPartitionPlan)>,
}

/// Runs transfer boot checks and Gate D datapath foundation init using embedded snapshots.
#[cfg(feature = "datapath-foundation")]
pub fn boot_from_transfer_and_init_gate_d_datapath_foundation_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathFoundationResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_foundation_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D datapath foundation init using snapshot + layout metadata.
#[cfg(feature = "datapath-foundation")]
pub fn boot_from_transfer_and_init_gate_d_datapath_foundation<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathFoundationResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_foundation_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs boot checks from raw inputs and Gate D datapath foundation init.
#[cfg(feature = "datapath-foundation")]
pub fn boot_check_and_init_gate_d_datapath_foundation<A: PageAllocator>(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathFoundationResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_d_datapath_foundation_from_validated(
        requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

#[cfg(feature = "datapath-foundation")]
pub(crate) fn init_gate_d_datapath_foundation_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathFoundationResult, BootCheckError> {
    let vmx_launch = init_gate_c_vmx_launch_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
    )?;

    let partition_boot_infos = build_guest_boot_infos_all_partitions(layout).map_err(|err| {
        BootCheckError::new(BootCheckErrorKind::Platform, err.message)
    })?;
    if partition_boot_infos.len() != layout.guest_memory.len() {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest boot info count mismatch with layout guest memory regions",
        ));
    }

    let mut datapath_plans = alloc::vec::Vec::with_capacity(layout.guest_memory.len());
    for (vm_id, blob) in &partition_boot_infos {
        GuestBootInfoView::parse(blob).map_err(|err| {
            BootCheckError::new(BootCheckErrorKind::Platform, err.message)
        })?;
        let plan = plan_datapath_for_vm_id(layout, *vm_id).map_err(|err| {
            BootCheckError::new(BootCheckErrorKind::Platform, err.message)
        })?;
        datapath_plans.push((*vm_id, plan));
    }

    Ok(GateDDatapathFoundationResult {
        vmx_launch,
        partition_boot_infos,
        datapath_plans,
    })
}

/// Result of Gate D datapath live init atop datapath foundation.
#[cfg(feature = "datapath-live")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDDatapathLiveResult {
    /// Datapath foundation output including guest boot info for all partitions.
    pub foundation: GateDDatapathFoundationResult,
    /// Mock/live forward plan used for synthetic frame traversal.
    pub forward_plan: hv_datapath::DatapathForwardPlan,
    /// Datapath live CPU seam outcome when the seam ran.
    pub live_seam: Option<hv_x86_cpu::DatapathLiveCpuSeamOutcome>,
    /// Mock datapath runtime outcome when forwarding ran.
    pub live_outcome: Option<hv_datapath::DatapathLiveOutcome>,
}

/// Runs transfer boot checks and Gate D datapath live init using embedded snapshots.
#[cfg(feature = "datapath-live")]
pub fn boot_from_transfer_and_init_gate_d_datapath_live_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathLiveResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_live_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D datapath live init using snapshot + layout metadata.
#[cfg(feature = "datapath-live")]
pub fn boot_from_transfer_and_init_gate_d_datapath_live<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathLiveResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_live_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs boot checks from raw inputs and Gate D datapath live init.
#[cfg(feature = "datapath-live")]
pub fn boot_check_and_init_gate_d_datapath_live<A: PageAllocator>(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathLiveResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_d_datapath_live_from_validated(
        requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

#[cfg(feature = "datapath-live")]
fn init_gate_d_datapath_live_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathLiveResult, BootCheckError> {
    let foundation = init_gate_d_datapath_foundation_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
    )?;

    let mut forward_plan = hv_datapath::plan_datapath_forward(layout).map_err(|err| {
        BootCheckError::new(BootCheckErrorKind::Platform, err.message)
    })?;

    let vmcs_phys = foundation.vmx_launch.real_hw.vmcs_phys.ok_or_else(|| {
        BootCheckError::new(
            BootCheckErrorKind::Platform,
            "datapath live requires an installed VMCS region from REAL_HW EPT init",
        )
    })?;

    let launch_plan = hv_vmx::plan_vmx_launch(
        layout,
        &foundation.vmx_launch.real_hw.live.cpu_seam.programming.init.vmx_plan,
        hv_vmx::DEFAULT_SMOKE_GUEST_PARTITION_ID,
    )
    .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;

    let live_seam = Some(
        hv_x86_cpu::run_datapath_live_cpu_seam(vmcs_phys, launch_plan.host_exit_phys.raw())
            .map_err(map_cpu_seam_error)?,
    );

    let mut backend = hv_datapath::MockDatapathBackend::new(forward_plan.clone());
    let live_outcome = Some(backend.run_live().map_err(|err| {
        BootCheckError::new(BootCheckErrorKind::Platform, err.message)
    })?);
    forward_plan = backend.forward_plan;

    Ok(GateDDatapathLiveResult {
        foundation,
        forward_plan,
        live_seam,
        live_outcome,
    })
}

#[cfg(feature = "datapath-live")]
fn map_cpu_seam_error(err: hv_x86_cpu::CpuSeamError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::Platform, err.message)
}
