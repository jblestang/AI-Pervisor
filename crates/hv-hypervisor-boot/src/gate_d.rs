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

#[cfg(any(feature = "datapath-live", feature = "datapath-guests", feature = "datapath-runtime"))]
fn map_cpu_seam_error(err: hv_x86_cpu::CpuSeamError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::Platform, err.message)
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
    use hv_vmx::{patch_guest_entry_in_fields, plan_vmx_launch_all_partitions, program_vmcs_fields};
    use hv_x86_cpu::{install_guest_elf, install_vmcs_region, run_multi_vmx_launch_cpu_seam};

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
        let guest_entry_phys =
            install_guest_elf(allocator, elf_bytes).map_err(map_cpu_seam_error)?;
        elf_images_installed = elf_images_installed.saturating_add(1);
        let vmcs_phys = install_vmcs_region(allocator).map_err(map_cpu_seam_error)?;
        let mut vmcs_fields = program_vmcs_fields(launch_plan);
        patch_guest_entry_in_fields(
            &mut vmcs_fields,
            guest_entry_phys,
            launch_plan.guest_stack_phys.raw(),
        );
        seam_inputs.push((vmcs_phys, vmcs_fields, launch_plan.vm_id));
        partition_launches.push(PartitionGuestLaunchRecord {
            partition_id: launch_plan.partition_id.clone(),
            guest_entry_phys,
            vmcs_phys,
            vm_id: launch_plan.vm_id,
            launch_seam: hv_x86_cpu::VmxLaunchCpuSeamOutcome {
                disposition: hv_x86_cpu::CpuInstructionDisposition::SeamValidated,
                guest_vm_id: launch_plan.vm_id,
            },
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
    )
}

#[cfg(feature = "datapath-benchmark")]
pub(crate) fn init_gate_d_datapath_benchmark_from_validated<A: PageAllocator>(
    requirements: &PlatformRequirements,
    layout: &StaticPlatformIR,
    validated: &ValidatedPlatform,
    warnings: alloc::vec::Vec<PlatformWarning>,
    allocator: &mut A,
) -> Result<GateDDatapathBenchmarkResult, BootCheckError> {
    let guests = init_gate_d_datapath_guests_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
        hv_guest_boot::GuestElfKind::Standard,
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
    use hv_guest_boot::GuestElfKind;
    use hv_vmx::plan_vmx_launch_all_partitions;
    use hv_x86_cpu::run_datapath_runtime_cpu_seam;

    let guests = init_gate_d_datapath_guests_from_validated(
        requirements,
        layout,
        validated,
        warnings,
        allocator,
        GuestElfKind::Datapath,
    )?;
    let datapath_elf_images_installed = guests.elf_images_installed;

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

    let (_forward_plan, runtime) = hv_datapath::run_guest_datapath_runtime(layout).map_err(|err| {
        BootCheckError::new(BootCheckErrorKind::Platform, err.message)
    })?;

    if !runtime.guest_frame_forwarded {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest datapath runtime did not forward a frame",
        ));
    }

    let mut seam_inputs = alloc::vec::Vec::with_capacity(guests.partition_launches.len());
    for record in &guests.partition_launches {
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
        benchmark: GateDDatapathBenchmarkResult { guests, benchmark },
        runtime,
        runtime_seam,
        datapath_elf_images_installed,
    })
}
