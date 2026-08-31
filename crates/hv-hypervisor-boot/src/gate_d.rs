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
    /// Synthetic forward plan backing IPC queues (live mock skipped when `datapath-runtime` is enabled).
    pub forward_plan: hv_datapath::DatapathForwardPlan,
    /// Smoke-guest datapath live CPU seam outcome (`None` when superseded by runtime).
    pub live_seam: Option<hv_x86_cpu::DatapathLiveCpuSeamOutcome>,
    /// Live synthetic forward outcome (`None` when superseded by guest runtime).
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
pub(crate) fn init_gate_d_datapath_live_from_validated<A: PageAllocator>(
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

    let forward_plan = hv_datapath::plan_datapath_forward(layout).map_err(|err| {
        BootCheckError::new(BootCheckErrorKind::Platform, err.message)
    })?;

    #[cfg(not(feature = "datapath-runtime"))]
    let (forward_plan, live_seam, live_outcome) = {
        let mut forward_plan = forward_plan;
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
        let live_outcome = Some(hv_datapath::run_datapath_live_forward(&mut forward_plan).map_err(
            |err| BootCheckError::new(BootCheckErrorKind::Platform, err.message),
        )?);
        (forward_plan, live_seam, live_outcome)
    };

    #[cfg(feature = "datapath-runtime")]
    let (live_seam, live_outcome): (
        Option<hv_x86_cpu::DatapathLiveCpuSeamOutcome>,
        Option<hv_datapath::DatapathLiveOutcome>,
    ) = (None, None);

    Ok(GateDDatapathLiveResult {
        foundation,
        forward_plan,
        live_seam,
        live_outcome,
    })
}

#[cfg(any(feature = "datapath-live", feature = "datapath-guests", feature = "datapath-runtime", feature = "datapath-guest-execution", feature = "datapath-guest-throughput"))]
fn map_cpu_seam_error(err: hv_x86_cpu::CpuSeamError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::Platform, err.message)
}

#[cfg(all(
    feature = "datapath-guest-live",
    any(feature = "datapath-guest-execution", feature = "datapath-guest-throughput")
))]
fn build_guest_live_vmcs_fields(
    launch_plan: &hv_vmx::VmxLaunchPlan,
    guest_entry_phys: u64,
    boot_info_phys: u64,
) -> Result<hv_vmx::VmcsProgrammedFields, BootCheckError> {
    use hv_vmx::{
        guest_boot_info_rdi_programmed, patch_guest_boot_info_rdi, patch_guest_entry_in_fields,
        program_vmcs_fields,
    };

    let mut vmcs_fields = program_vmcs_fields(launch_plan);
    patch_guest_entry_in_fields(
        &mut vmcs_fields,
        guest_entry_phys,
        launch_plan.guest_stack_phys.raw(),
    );
    patch_guest_boot_info_rdi(&mut vmcs_fields, boot_info_phys);
    if !guest_boot_info_rdi_programmed(&vmcs_fields, boot_info_phys) {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest VMCS RDI was not programmed",
        ));
    }
    Ok(vmcs_fields)
}

/// Result of Gate D datapath malicious init atop datapath live.
#[cfg(feature = "datapath-malicious")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDDatapathMaliciousResult {
    /// Datapath live output including synthetic frame traversal.
    pub live: GateDDatapathLiveResult,
    /// Whether clean IPC queues passed integrity scans.
    pub integrity_checks_passed: bool,
    /// Number of reference compromised-guest scenarios blocked.
    pub compromised_scenarios_blocked: u32,
}

/// Runs transfer boot checks and Gate D datapath malicious init using embedded snapshots.
#[cfg(feature = "datapath-malicious")]
pub fn boot_from_transfer_and_init_gate_d_datapath_malicious_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathMaliciousResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_malicious_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D datapath malicious init using snapshot + layout metadata.
#[cfg(feature = "datapath-malicious")]
pub fn boot_from_transfer_and_init_gate_d_datapath_malicious<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathMaliciousResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_malicious_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs boot checks from raw inputs and Gate D datapath malicious init.
#[cfg(feature = "datapath-malicious")]
pub fn boot_check_and_init_gate_d_datapath_malicious<A: PageAllocator>(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathMaliciousResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_d_datapath_malicious_from_validated(
        requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

#[cfg(feature = "datapath-malicious")]
pub(crate) fn init_gate_d_datapath_malicious_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathMaliciousResult, BootCheckError> {
    let live = init_gate_d_datapath_live_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
    )?;

    let (integrity_checks_passed, compromised_scenarios_blocked) =
        hv_datapath::run_reference_compromised_scenarios(|| {
            hv_datapath::plan_datapath_forward(layout)
        })
        .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;

    Ok(GateDDatapathMaliciousResult {
        live,
        integrity_checks_passed,
        compromised_scenarios_blocked,
    })
}

/// One partition guest ELF install and VMX launch record.
#[cfg(feature = "datapath-guests")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionGuestLaunchRecord {
    /// Partition identifier.
    pub partition_id: alloc::string::String,
    /// Guest VM identifier.
    pub vm_id: VmId,
    /// Host physical guest entry after ELF installation.
    pub guest_entry_phys: u64,
    /// Installed VMCS region host physical address.
    pub vmcs_phys: u64,
    /// VMX launch CPU seam outcome for this partition.
    pub launch_seam: hv_x86_cpu::VmxLaunchCpuSeamOutcome,
    /// Guest physical address of installed boot-info blob when `datapath-guest-live` is enabled.
    pub boot_info_guest_phys: Option<u64>,
}

/// Result of Gate D datapath guests init atop datapath malicious.
#[cfg(feature = "datapath-guests")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDDatapathGuestsResult {
    /// Datapath malicious output including integrity checks.
    pub malicious: GateDDatapathMaliciousResult,
    /// Per-partition ELF install and launch records.
    pub partition_launches: alloc::vec::Vec<PartitionGuestLaunchRecord>,
    /// Number of reference guest ELF images installed.
    pub elf_images_installed: u32,
    /// Multi-partition launch CPU seam batch outcome.
    pub multi_launch_seam: hv_x86_cpu::MultiVmxLaunchCpuSeamOutcome,
}

/// Runs transfer boot checks and Gate D datapath guests init using embedded snapshots.
#[cfg(feature = "datapath-guests")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guests_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathGuestsResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_guests_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Standard,
    )
}

/// Runs transfer boot checks and Gate D datapath guests init using snapshot + layout metadata.
#[cfg(feature = "datapath-guests")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guests<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathGuestsResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_guests_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Standard,
    )
}

/// Runs boot checks from raw inputs and Gate D datapath guests init.
#[cfg(feature = "datapath-guests")]
pub fn boot_check_and_init_gate_d_datapath_guests<A: PageAllocator>(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathGuestsResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_d_datapath_guests_from_validated(
        requirements,
        layout,
        &validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Standard,
    )
}

#[cfg(feature = "datapath-guests")]
pub(crate) fn init_gate_d_datapath_guests_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
    elf_kind: hv_guest_boot::GuestElfKind,
) -> Result<GateDDatapathGuestsResult, BootCheckError> {
    use hv_guest_boot::{reference_guest_elf_for_kind, REFERENCE_GUEST_PARTITION_IDS};
    use hv_vmx::plan_vmx_launch_all_partitions;
    #[cfg(not(feature = "datapath-guest-execution"))]
    use hv_vmx::{patch_guest_entry_in_fields, program_vmcs_fields};
    #[cfg(all(feature = "datapath-guest-live", not(feature = "datapath-guest-execution")))]
    use hv_vmx::{guest_boot_info_rdi_programmed, patch_guest_boot_info_rdi};
    use hv_x86_cpu::{install_vmcs_region, run_multi_vmx_launch_cpu_seam};
    #[cfg(not(feature = "datapath-guest-live"))]
    use hv_x86_cpu::install_guest_elf;
    #[cfg(feature = "datapath-guest-live")]
    use hv_x86_cpu::install_guest_elf_with_boot_info;

    let malicious = init_gate_d_datapath_malicious_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
    )?;

    let vmx_plan = &malicious
        .live
        .foundation
        .vmx_launch
        .real_hw
        .live
        .cpu_seam
        .programming
        .init
        .vmx_plan;
    let launch_plans = plan_vmx_launch_all_partitions(layout, vmx_plan)
        .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;

    #[cfg(feature = "datapath-guest-live")]
    let partition_boot_infos = &malicious.live.foundation.partition_boot_infos;

    let mut partition_launches = alloc::vec::Vec::with_capacity(launch_plans.len());
    let mut seam_inputs = alloc::vec::Vec::with_capacity(launch_plans.len());
    let mut elf_images_installed = 0u32;

    for launch_plan in &launch_plans {
        let elf_bytes = reference_guest_elf_for_kind(&launch_plan.partition_id, elf_kind).ok_or_else(|| {
            BootCheckError::new(
                BootCheckErrorKind::Platform,
                "missing reference guest elf for partition",
            )
        })?;
        hv_guest_boot::parse_elf64(elf_bytes).map_err(|err| {
            BootCheckError::new(BootCheckErrorKind::Platform, err.message)
        })?;
        #[cfg(not(feature = "datapath-guest-live"))]
        let guest_entry_phys =
            install_guest_elf(allocator, elf_bytes).map_err(map_cpu_seam_error)?;
        #[cfg(feature = "datapath-guest-live")]
        let (guest_entry_phys, boot_info_guest_phys) = {
            let boot_info_blob = partition_boot_infos
                .iter()
                .find(|(vm_id, _)| *vm_id == launch_plan.vm_id)
                .map(|(_, blob)| blob.as_slice())
                .ok_or_else(|| {
                    BootCheckError::new(
                        BootCheckErrorKind::Platform,
                        "missing guest boot info blob for partition",
                    )
                })?;
            GuestBootInfoView::parse(boot_info_blob).map_err(|err| {
                BootCheckError::new(BootCheckErrorKind::Platform, err.message)
            })?;
            let install = install_guest_elf_with_boot_info(allocator, elf_bytes, boot_info_blob)
                .map_err(map_cpu_seam_error)?;
            (install.entry_phys, Some(install.boot_info_phys))
        };
        elf_images_installed = elf_images_installed.saturating_add(1);
        let vmcs_phys = install_vmcs_region(allocator).map_err(map_cpu_seam_error)?;
        #[cfg(all(feature = "datapath-guest-live", not(feature = "datapath-guest-execution")))]
        {
            let mut vmcs_fields = program_vmcs_fields(launch_plan);
            patch_guest_entry_in_fields(
                &mut vmcs_fields,
                guest_entry_phys,
                launch_plan.guest_stack_phys.raw(),
            );
            let boot_info_phys = boot_info_guest_phys.ok_or_else(|| {
                BootCheckError::new(
                    BootCheckErrorKind::Platform,
                    "guest boot info address missing after install",
                )
            })?;
            patch_guest_boot_info_rdi(&mut vmcs_fields, boot_info_phys);
            if !guest_boot_info_rdi_programmed(&vmcs_fields, boot_info_phys) {
                return Err(BootCheckError::new(
                    BootCheckErrorKind::Platform,
                    "guest boot info RDI was not programmed in VMCS fields",
                ));
            }
            seam_inputs.push((vmcs_phys, vmcs_fields, launch_plan.vm_id));
        }
        #[cfg(all(feature = "datapath-guest-live", feature = "datapath-guest-execution"))]
        {
            let boot_info_phys = boot_info_guest_phys.ok_or_else(|| {
                BootCheckError::new(
                    BootCheckErrorKind::Platform,
                    "guest boot info address missing after install",
                )
            })?;
            let vmcs_fields =
                build_guest_live_vmcs_fields(launch_plan, guest_entry_phys, boot_info_phys)?;
            seam_inputs.push((vmcs_phys, vmcs_fields, launch_plan.vm_id));
        }
        #[cfg(not(feature = "datapath-guest-live"))]
        {
            let mut vmcs_fields = program_vmcs_fields(launch_plan);
            patch_guest_entry_in_fields(
                &mut vmcs_fields,
                guest_entry_phys,
                launch_plan.guest_stack_phys.raw(),
            );
            seam_inputs.push((vmcs_phys, vmcs_fields, launch_plan.vm_id));
        }
        #[cfg(not(feature = "datapath-guest-live"))]
        let boot_info_guest_phys = None;
        partition_launches.push(PartitionGuestLaunchRecord {
            partition_id: launch_plan.partition_id.clone(),
            guest_entry_phys,
            vmcs_phys,
            vm_id: launch_plan.vm_id,
            launch_seam: hv_x86_cpu::VmxLaunchCpuSeamOutcome {
                disposition: hv_x86_cpu::CpuInstructionDisposition::SeamValidated,
                guest_vm_id: launch_plan.vm_id,
            },
            boot_info_guest_phys,
        });
    }

    if elf_images_installed != REFERENCE_GUEST_PARTITION_IDS.len() as u32 {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest elf install count mismatch with reference partitions",
        ));
    }

    let multi_launch_seam = run_multi_vmx_launch_cpu_seam(&seam_inputs).map_err(map_cpu_seam_error)?;
    for (record, outcome) in partition_launches
        .iter_mut()
        .zip(multi_launch_seam.launches.iter())
    {
        record.launch_seam = outcome.clone();
    }

    Ok(GateDDatapathGuestsResult {
        malicious,
        partition_launches,
        elf_images_installed,
        multi_launch_seam,
    })
}

/// Result of Gate D datapath benchmark init atop datapath guests.
#[cfg(feature = "datapath-benchmark")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDDatapathBenchmarkResult {
    /// Datapath guests output including ELF install and multi-partition launch.
    pub guests: GateDDatapathGuestsResult,
    /// Mock datapath throughput benchmark outcome.
    pub benchmark: hv_datapath::DatapathBenchmarkResult,
}

/// Runs transfer boot checks and Gate D datapath benchmark init using embedded snapshots.
#[cfg(feature = "datapath-benchmark")]
pub fn boot_from_transfer_and_init_gate_d_datapath_benchmark_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathBenchmarkResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_benchmark_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Standard,
    )
}

/// Runs transfer boot checks and Gate D datapath benchmark init using snapshot + layout metadata.
#[cfg(feature = "datapath-benchmark")]
pub fn boot_from_transfer_and_init_gate_d_datapath_benchmark<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathBenchmarkResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_benchmark_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Standard,
    )
}

/// Runs boot checks from raw inputs and Gate D datapath benchmark init.
#[cfg(feature = "datapath-benchmark")]
pub fn boot_check_and_init_gate_d_datapath_benchmark<A: PageAllocator>(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathBenchmarkResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_d_datapath_benchmark_from_validated(
        requirements,
        layout,
        &validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Standard,
    )
}

#[cfg(feature = "datapath-benchmark")]
pub(crate) fn init_gate_d_datapath_benchmark_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
    elf_kind: hv_guest_boot::GuestElfKind,
) -> Result<GateDDatapathBenchmarkResult, BootCheckError> {
    let guests = init_gate_d_datapath_guests_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
        elf_kind,
    )?;

    let benchmark = hv_datapath::run_mock_datapath_benchmark(
        layout,
        &hv_datapath::DatapathBenchmarkConfig::default(),
    )
    .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;

    if !benchmark.target_met {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "datapath benchmark target not met",
        ));
    }

    Ok(GateDDatapathBenchmarkResult { guests, benchmark })
}

/// Result of Gate D datapath runtime init atop datapath benchmark.
#[cfg(feature = "datapath-runtime")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDDatapathRuntimeResult {
    /// Datapath benchmark output including mock throughput validation.
    pub benchmark: GateDDatapathBenchmarkResult,
    /// Guest-driven datapath runtime outcome.
    pub runtime: hv_datapath::DatapathRuntimeOutcome,
    /// Multi-partition datapath runtime CPU seam outcome.
    pub runtime_seam: hv_x86_cpu::DatapathRuntimeCpuSeamOutcome,
    /// Number of datapath-capable guest ELF images installed.
    pub datapath_elf_images_installed: u32,
}

/// Runs transfer boot checks and Gate D datapath runtime init using embedded snapshots.
#[cfg(feature = "datapath-runtime")]
pub fn boot_from_transfer_and_init_gate_d_datapath_runtime_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathRuntimeResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_runtime_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D datapath runtime init using snapshot + layout metadata.
#[cfg(feature = "datapath-runtime")]
pub fn boot_from_transfer_and_init_gate_d_datapath_runtime<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathRuntimeResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_runtime_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs boot checks from raw inputs and Gate D datapath runtime init.
#[cfg(feature = "datapath-runtime")]
pub fn boot_check_and_init_gate_d_datapath_runtime<A: PageAllocator>(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathRuntimeResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    init_gate_d_datapath_runtime_from_validated(
        requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

#[cfg(feature = "datapath-runtime")]
pub(crate) fn init_gate_d_datapath_runtime_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathRuntimeResult, BootCheckError> {
    init_gate_d_datapath_runtime_with_elf_kind_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Datapath,
    )
}

#[cfg(any(feature = "datapath-runtime", feature = "datapath-guest-sources"))]
pub(crate) fn init_gate_d_datapath_runtime_with_elf_kind_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
    elf_kind: hv_guest_boot::GuestElfKind,
) -> Result<GateDDatapathRuntimeResult, BootCheckError> {
    use hv_vmx::plan_vmx_launch_all_partitions;
    use hv_x86_cpu::run_datapath_runtime_cpu_seam;

    let benchmark = init_gate_d_datapath_benchmark_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
        elf_kind,
    )?;
    let datapath_elf_images_installed = benchmark.guests.elf_images_installed;

    let vmx_plan = &benchmark
        .guests
        .malicious
        .live
        .foundation
        .vmx_launch
        .real_hw
        .live
        .cpu_seam
        .programming
        .init
        .vmx_plan;
    let launch_plans = plan_vmx_launch_all_partitions(layout, vmx_plan)
        .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;

    let (_forward_plan, runtime) = hv_datapath::run_guest_datapath_runtime(layout).map_err(|err| {
        BootCheckError::new(BootCheckErrorKind::Platform, err.message)
    })?;

    if !runtime.guest_frame_forwarded {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest datapath runtime did not forward a frame",
        ));
    }

    let mut seam_inputs = alloc::vec::Vec::with_capacity(benchmark.guests.partition_launches.len());
    for record in &benchmark.guests.partition_launches {
        let launch_plan = launch_plans
            .iter()
            .find(|plan| plan.vm_id == record.vm_id)
            .ok_or_else(|| {
                BootCheckError::new(
                    BootCheckErrorKind::Platform,
                    "missing launch plan for datapath runtime seam",
                )
            })?;
        seam_inputs.push((record.vmcs_phys, launch_plan.host_exit_phys.raw()));
    }

    let runtime_seam = run_datapath_runtime_cpu_seam(&seam_inputs).map_err(map_cpu_seam_error)?;

    Ok(GateDDatapathRuntimeResult {
        benchmark,
        runtime,
        runtime_seam,
        datapath_elf_images_installed,
    })
}

/// Result of Gate D datapath guest-sources init using built `guests/` ELFs.
#[cfg(feature = "datapath-guest-sources")]
pub struct GateDDatapathGuestSourcesResult {
    /// Datapath runtime output using source-tree guest ELFs.
    pub runtime: GateDDatapathRuntimeResult,
}

/// Runs transfer boot checks and Gate D guest-sources init using embedded snapshots.
#[cfg(feature = "datapath-guest-sources")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_sources_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathGuestSourcesResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_guest_sources_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D guest-sources init.
#[cfg(feature = "datapath-guest-sources")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_sources<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathGuestSourcesResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_guest_sources_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

#[cfg(feature = "datapath-guest-sources")]
pub(crate) fn init_gate_d_datapath_guest_sources_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathGuestSourcesResult, BootCheckError> {
    if !hv_guest_boot::GUEST_SOURCE_ELFS_AVAILABLE {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest source ELFs not embedded; run cargo xtask build-guests first",
        ));
    }
    let runtime = init_gate_d_datapath_runtime_with_elf_kind_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Source,
    )?;
    Ok(GateDDatapathGuestSourcesResult { runtime })
}

/// Result of Gate D datapath guest-live init with boot-info install and RDI patching.
#[cfg(feature = "datapath-guest-live")]
pub struct GateDDatapathGuestLiveResult {
    /// Datapath guest-sources output using built `guests/` ELFs.
    pub sources: GateDDatapathGuestSourcesResult,
    /// Number of guest boot-info blobs installed and wired to VMCS RDI.
    pub boot_infos_installed: u32,
}

/// Runs transfer boot checks and Gate D guest-live init using embedded snapshots.
#[cfg(feature = "datapath-guest-live")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_live_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathGuestLiveResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_guest_live_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D guest-live init.
#[cfg(feature = "datapath-guest-live")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_live<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathGuestLiveResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_guest_live_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

#[cfg(feature = "datapath-guest-live")]
pub(crate) fn init_gate_d_datapath_guest_live_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathGuestLiveResult, BootCheckError> {
    use hv_guest_boot::REFERENCE_GUEST_PARTITION_IDS;

    let sources = init_gate_d_datapath_guest_sources_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
    )?;
    let boot_infos_installed = sources
        .runtime
        .benchmark
        .guests
        .partition_launches
        .iter()
        .filter(|record| record.boot_info_guest_phys.is_some())
        .count() as u32;
    if boot_infos_installed != REFERENCE_GUEST_PARTITION_IDS.len() as u32 {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest boot info install count mismatch with reference partitions",
        ));
    }
    Ok(GateDDatapathGuestLiveResult {
        sources,
        boot_infos_installed,
    })
}

/// Result of Gate D datapath guest execution init with live VMX source-tree guest code.
#[cfg(feature = "datapath-guest-execution")]
pub struct GateDDatapathGuestExecutionResult {
    /// Datapath guest-live output including boot-info install and RDI patching.
    pub live: GateDDatapathGuestLiveResult,
    /// Live VMX guest execution seam outcome for all source-tree partitions.
    pub execution_seam: hv_x86_cpu::DatapathGuestExecutionCpuSeamOutcome,
}

/// Runs transfer boot checks and Gate D guest execution init using embedded snapshots.
#[cfg(feature = "datapath-guest-execution")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_execution_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathGuestExecutionResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_guest_execution_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D guest execution init.
#[cfg(feature = "datapath-guest-execution")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_execution<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathGuestExecutionResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_guest_execution_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

#[cfg(feature = "datapath-guest-execution")]
pub(crate) fn init_gate_d_datapath_guest_execution_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathGuestExecutionResult, BootCheckError> {
    use hv_datapath::{
        apply_runtime_disposition, runtime_disposition_for_guest_execution_seam,
        DatapathRuntimeDisposition,
    };
    use hv_guest_boot::REFERENCE_GUEST_PARTITION_IDS;
    use hv_vmx::plan_vmx_launch_all_partitions;
    use hv_x86_cpu::{run_datapath_guest_execution_cpu_seam, CpuInstructionDisposition};

    let mut live = init_gate_d_datapath_guest_live_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
    )?;

    let guests = &mut live.sources.runtime.benchmark.guests;
    let vmx_plan = &guests
        .malicious
        .live
        .foundation
        .vmx_launch
        .real_hw
        .live
        .cpu_seam
        .programming
        .init
        .vmx_plan;
    let launch_plans = plan_vmx_launch_all_partitions(layout, vmx_plan)
        .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;

    let mut vmcs_fields_store = alloc::vec::Vec::with_capacity(guests.partition_launches.len());
    let mut execution_launches = alloc::vec::Vec::with_capacity(guests.partition_launches.len());
    for record in &guests.partition_launches {
        let launch_plan = launch_plans
            .iter()
            .find(|plan| plan.vm_id == record.vm_id)
            .ok_or_else(|| {
                BootCheckError::new(
                    BootCheckErrorKind::Platform,
                    "missing launch plan for guest execution seam",
                )
            })?;
        let boot_info_phys = record.boot_info_guest_phys.ok_or_else(|| {
            BootCheckError::new(
                BootCheckErrorKind::Platform,
                "guest execution requires installed boot info for partition",
            )
        })?;
        let vmcs_fields = build_guest_live_vmcs_fields(
            launch_plan,
            record.guest_entry_phys,
            boot_info_phys,
        )?;
        execution_launches.push((
            record.vmcs_phys,
            vmcs_fields_store.len(),
            launch_plan.host_exit_phys.raw(),
            launch_plan.vm_id,
        ));
        vmcs_fields_store.push(vmcs_fields);
    }

    if execution_launches.len() != REFERENCE_GUEST_PARTITION_IDS.len() {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest execution launch count mismatch with reference partitions",
        ));
    }

    let seam_inputs = execution_launches
        .iter()
        .map(|(vmcs_phys, field_index, host_exit_phys, vm_id)| {
            let fields = vmcs_fields_store.get(*field_index).ok_or_else(|| {
                BootCheckError::new(
                    BootCheckErrorKind::Platform,
                    "guest execution VMCS field index out of range",
                )
            })?;
            Ok((
                *vmcs_phys,
                fields,
                *host_exit_phys,
                *vm_id,
            ))
        })
        .collect::<Result<alloc::vec::Vec<_>, BootCheckError>>()?;
    let execution_seam =
        run_datapath_guest_execution_cpu_seam(&seam_inputs).map_err(map_cpu_seam_error)?;

    let executed = execution_seam.disposition == CpuInstructionDisposition::Executed;
    let skipped_no_hardware =
        execution_seam.disposition == CpuInstructionDisposition::SkippedNoHardware;
    let runtime_disposition =
        runtime_disposition_for_guest_execution_seam(executed, skipped_no_hardware);
    if executed && execution_seam.vmlaunch_attempts != execution_seam.partitions_validated {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest execution reported Executed without a VMLAUNCH per partition",
        ));
    }
    if !live.sources.runtime.runtime.guest_frame_forwarded {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest execution requires a forwarded datapath frame",
        ));
    }
    live.sources.runtime.runtime = apply_runtime_disposition(
        live.sources.runtime.runtime.clone(),
        runtime_disposition,
    );
    if (runtime_disposition == DatapathRuntimeDisposition::Executed) != executed {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest execution runtime disposition mismatch",
        ));
    }
    if executed {
        for record in &mut guests.partition_launches {
            record.launch_seam.disposition = CpuInstructionDisposition::Executed;
        }
    }

    Ok(GateDDatapathGuestExecutionResult {
        live,
        execution_seam,
    })
}

/// Result of Gate D datapath guest throughput init with in-VM benchmark measurement.
#[cfg(feature = "datapath-guest-throughput")]
pub struct GateDDatapathGuestThroughputResult {
    /// Datapath guest execution output including live VMX scaffolding.
    pub execution: GateDDatapathGuestExecutionResult,
    /// In-VM guest throughput benchmark outcome.
    pub throughput: hv_datapath::GuestThroughputBenchmarkResult,
    /// Live guest throughput CPU seam outcome.
    pub throughput_seam: hv_x86_cpu::DatapathGuestThroughputCpuSeamOutcome,
    /// Sustained guest relay frames validated on the host runtime path.
    #[cfg(feature = "datapath-guest-relay-live")]
    pub sustained_relay_frames: u64,
}

/// Runs transfer boot checks and Gate D guest throughput init using embedded snapshots.
#[cfg(feature = "datapath-guest-throughput")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_throughput_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathGuestThroughputResult, BootCheckError> {
    let platform_requirements = platform_requirements_from_snapshot(requirements)?;
    let static_layout = static_platform_ir_from_layout_snapshot(layout, requirements)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &requirements.config_digest, &platform_requirements)?;
    init_gate_d_datapath_guest_throughput_from_validated(
        &platform_requirements,
        &static_layout,
        &validated,
        warnings,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D guest throughput init.
#[cfg(feature = "datapath-guest-throughput")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_throughput<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathGuestThroughputResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    init_gate_d_datapath_guest_throughput_from_validated(
        &requirements,
        layout,
        &validated,
        warnings,
        allocator,
    )
}

#[cfg(feature = "datapath-guest-throughput")]
pub(crate) fn init_gate_d_datapath_guest_throughput_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathGuestThroughputResult, BootCheckError> {
    use hv_datapath::{
        run_mock_guest_throughput_benchmark, DatapathBenchmarkConfig, GuestThroughputDisposition,
    };
    #[cfg(not(feature = "datapath-guest-relay-live"))]
    use hv_datapath::{apply_guest_throughput_disposition, guest_throughput_disposition_for_seam};
    use hv_x86_cpu::{run_datapath_guest_throughput_cpu_seam, CpuInstructionDisposition};

    let execution = init_gate_d_datapath_guest_execution_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
    )?;

    let benchmark_config = DatapathBenchmarkConfig::default();
    let mut throughput =
        run_mock_guest_throughput_benchmark(layout, &benchmark_config).map_err(|err| {
            BootCheckError::new(BootCheckErrorKind::Platform, err.message)
        })?;
    if !throughput.benchmark.target_met {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest throughput benchmark target not met",
        ));
    }

    #[cfg(feature = "datapath-guest-relay-live")]
    let expected_relay_frames = {
        use hv_datapath::GUEST_RELAY_BENCHMARK_FRAMES;
        u64::from(GUEST_RELAY_BENCHMARK_FRAMES)
    };
    #[cfg(not(feature = "datapath-guest-relay-live"))]
    let expected_relay_frames = 0u64;

    #[cfg(feature = "datapath-guest-relay-measurement")]
    let in_vm_relay_frames = {
        use hv_guest_boot::GuestBootInfoView;
        use hv_x86_cpu::{
            measure_in_vm_relay_frames_from_boot_infos, GuestBootInfoMeasurementSite,
        };

        let guests = &execution.live.sources.runtime.benchmark.guests;
        let partition_boot_infos = &guests.malicious.live.foundation.partition_boot_infos;
        let mut sites = alloc::vec::Vec::with_capacity(guests.partition_launches.len());
        for record in &guests.partition_launches {
            let boot_info_phys = record.boot_info_guest_phys.ok_or_else(|| {
                BootCheckError::new(
                    BootCheckErrorKind::Platform,
                    "relay measurement requires installed guest boot info",
                )
            })?;
            let blob = partition_boot_infos
                .iter()
                .find(|(vm_id, _)| *vm_id == record.vm_id)
                .map(|(_, blob)| blob.as_slice())
                .ok_or_else(|| {
                    BootCheckError::new(
                        BootCheckErrorKind::Platform,
                        "relay measurement missing boot info blob for partition",
                    )
                })?;
            let view = GuestBootInfoView::parse(blob).map_err(|err| {
                BootCheckError::new(BootCheckErrorKind::Platform, err.message)
            })?;
            if !hv_guest_abi::guest_boot_info_has_relay_measurement_tail(view.header()) {
                return Err(BootCheckError::new(
                    BootCheckErrorKind::Platform,
                    "relay measurement requires ABI v2 boot info tail",
                ));
            }
            sites.push(GuestBootInfoMeasurementSite {
                vm_id: record.vm_id,
                host_boot_info_phys: boot_info_phys,
                boot_info_size: view.header().size,
            });
        }
        measure_in_vm_relay_frames_from_boot_infos(
            &execution.execution_seam,
            &sites,
            expected_relay_frames,
        )
        .map_err(map_cpu_seam_error)?
    };
    #[cfg(all(feature = "datapath-guest-relay-live", not(feature = "datapath-guest-relay-measurement")))]
    let in_vm_relay_frames = 0u64;
    #[cfg(not(feature = "datapath-guest-relay-live"))]
    let in_vm_relay_frames = 0u64;

    let throughput_seam = run_datapath_guest_throughput_cpu_seam(
        &execution.execution_seam,
        throughput.benchmark.runs_completed,
        in_vm_relay_frames,
        expected_relay_frames,
    )
    .map_err(map_cpu_seam_error)?;

    if throughput_seam.partitions_validated != execution.execution_seam.partitions_validated {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest throughput partition count mismatch with execution seam",
        ));
    }
    if !throughput_seam.vmexit_stub_validated {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest throughput requires validated VM-exit stubs",
        ));
    }
    if throughput_seam.measurement_runs_validated != throughput.benchmark.runs_completed {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest throughput measurement run count mismatch",
        ));
    }

    let skipped_no_hardware =
        throughput_seam.disposition == CpuInstructionDisposition::SkippedNoHardware;

    #[cfg(feature = "datapath-guest-relay-live")]
    let sustained_relay_frames = {
        use hv_datapath::{
            guest_throughput_result_with_live_relay, validate_sustained_host_relay_benchmark,
            GUEST_RELAY_BENCHMARK_FRAMES,
        };

        let relay_frames = validate_sustained_host_relay_benchmark(
            layout,
            GUEST_RELAY_BENCHMARK_FRAMES,
            &benchmark_config,
        )
        .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;
        let guest_execution_executed =
            execution.execution_seam.disposition == CpuInstructionDisposition::Executed;
        throughput = guest_throughput_result_with_live_relay(
            throughput,
            guest_execution_executed,
            in_vm_relay_frames,
            expected_relay_frames,
            &benchmark_config,
            skipped_no_hardware,
        )
        .map_err(|err| BootCheckError::new(BootCheckErrorKind::Platform, err.message))?;
        #[cfg(feature = "datapath-guest-relay-measurement")]
        if throughput_seam.in_vm_relay_frames != in_vm_relay_frames {
            return Err(BootCheckError::new(
                BootCheckErrorKind::Platform,
                "guest relay measurement seam frame count mismatch",
            ));
        }
        if throughput_seam.live_relay_validated
            != {
                #[cfg(feature = "datapath-guest-relay-measurement")]
                {
                    guest_execution_executed && in_vm_relay_frames >= expected_relay_frames
                }
                #[cfg(not(feature = "datapath-guest-relay-measurement"))]
                {
                    guest_execution_executed
                }
            }
        {
            return Err(BootCheckError::new(
                BootCheckErrorKind::Platform,
                "guest relay live seam mismatch with execution outcome",
            ));
        }
        if throughput.disposition == GuestThroughputDisposition::Executed
            && in_vm_relay_frames < expected_relay_frames
        {
            return Err(BootCheckError::new(
                BootCheckErrorKind::Platform,
                "guest relay live Executed requires in-VM relay measurement stats",
            ));
        }
        relay_frames
    };

    #[cfg(not(feature = "datapath-guest-relay-live"))]
    {
        let live_measurement_completed = false;
        let throughput_disposition =
            guest_throughput_disposition_for_seam(live_measurement_completed, skipped_no_hardware);
        throughput = apply_guest_throughput_disposition(throughput, throughput_disposition);
        if throughput.disposition == GuestThroughputDisposition::Executed {
            return Err(BootCheckError::new(
                BootCheckErrorKind::Platform,
                "guest throughput Executed requires live in-VM measurement stats",
            ));
        }
        if (throughput.disposition == GuestThroughputDisposition::Unavailable) != skipped_no_hardware
        {
            return Err(BootCheckError::new(
                BootCheckErrorKind::Platform,
                "guest throughput disposition mismatch",
            ));
        }
    }

    #[cfg(feature = "datapath-guest-relay-live")]
    if (throughput.disposition == GuestThroughputDisposition::Unavailable) != skipped_no_hardware {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest relay live disposition mismatch",
        ));
    }

    Ok(GateDDatapathGuestThroughputResult {
        execution,
        throughput,
        throughput_seam,
        #[cfg(feature = "datapath-guest-relay-live")]
        sustained_relay_frames,
    })
}

/// Result of Gate D datapath guest relay live init (extends guest throughput).
#[cfg(feature = "datapath-guest-relay-live")]
pub type GateDDatapathGuestRelayLiveResult = GateDDatapathGuestThroughputResult;

/// Runs transfer boot checks and Gate D guest relay live init using embedded snapshots.
#[cfg(feature = "datapath-guest-relay-live")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_relay_live_from_snapshots<A: PageAllocator>(
    transfer: &[u8],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<GateDDatapathGuestRelayLiveResult, BootCheckError> {
    boot_from_transfer_and_init_gate_d_datapath_guest_throughput_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
}

/// Runs transfer boot checks and Gate D guest relay live init.
#[cfg(feature = "datapath-guest-relay-live")]
pub fn boot_from_transfer_and_init_gate_d_datapath_guest_relay_live<A: PageAllocator>(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
    layout: &StaticPlatformIR,
    allocator: &mut A,
) -> Result<GateDDatapathGuestRelayLiveResult, BootCheckError> {
    boot_from_transfer_and_init_gate_d_datapath_guest_throughput(transfer, snapshot, layout, allocator)
}
