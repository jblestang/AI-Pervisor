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
fn init_gate_d_datapath_foundation_from_validated<A: PageAllocator>(
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
