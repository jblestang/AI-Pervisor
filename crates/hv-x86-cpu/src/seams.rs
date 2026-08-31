//! CPU instruction seams for Gate C hardware bring-up.

use hv_ept::{
    EptProgrammedTables, EPT_POINTER_MEMORY_TYPE_SHIFT, EPT_POINTER_MEMORY_TYPE_WB,
    EPT_POINTER_PAGE_WALK_LENGTH, EPT_POINTER_PAGE_WALK_LENGTH_SHIFT, EPT_ROOT_TABLE_BYTES,
};
use hv_vmx::{VmxonProgrammedRegion, VmcsProgrammedFields, VMXON_REGION_MIN_BYTES};
use hv_vtd::VtdProgrammedTables;

use crate::constants::VMXON_REVISION_PREFIX_BYTES;

use crate::cpuid::{cpuid_ept_available, cpuid_vmx_available, cpuid_vtd_available};
use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Records whether a CPU seam validated or executed an instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuInstructionDisposition {
    /// Inputs and CPU capabilities validated; no privileged instruction executed.
    SeamValidated,
    /// Privileged instruction executed (`execute-instructions` feature).
    Executed,
    /// Required hardware capability absent; seam skipped.
    SkippedNoHardware,
}

/// Outcome of a VMXON CPU seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmxCpuSeamOutcome {
    /// How the seam completed.
    pub disposition: CpuInstructionDisposition,
    /// Host physical address of the VMXON region.
    pub vmxon_phys: u64,
}

/// Outcome of an EPT pointer CPU seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EptCpuSeamOutcome {
    /// How the seam completed.
    pub disposition: CpuInstructionDisposition,
    /// Encoded EPT pointer value (root table physical address with control bits).
    pub ept_pointer: u64,
}

/// Outcome of a VT-d enable CPU seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtdCpuSeamOutcome {
    /// How the seam completed.
    pub disposition: CpuInstructionDisposition,
    /// Whether interrupt remapping was part of the programmed tables.
    pub interrupt_remapping: bool,
}

/// Outcome of a VMX launch CPU seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmxLaunchCpuSeamOutcome {
    /// How the seam completed.
    pub disposition: CpuInstructionDisposition,
    /// Guest VM identifier targeted by the launch.
    pub guest_vm_id: hv_types::VmId,
}

/// Outcome of a Gate D datapath live CPU seam.
#[cfg(feature = "datapath-live")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathLiveCpuSeamOutcome {
    /// How the seam completed.
    pub disposition: CpuInstructionDisposition,
    /// Whether the VM-exit stub address was validated.
    pub vmexit_stub_validated: bool,
}

/// Validates (and optionally executes) a VMXON instruction seam.
pub fn run_vmxon_cpu_seam(region: &VmxonProgrammedRegion) -> Result<VmxCpuSeamOutcome, CpuSeamError> {
    validate_vmxon_region(region)?;
    if !cpuid_vmx_available() {
        return Ok(VmxCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SkippedNoHardware,
            vmxon_phys: region.host_phys,
        });
    }
    let disposition = execute_vmxon_if_enabled(region.host_phys)?;
    Ok(VmxCpuSeamOutcome {
        disposition,
        vmxon_phys: region.host_phys,
    })
}

/// Validates (and optionally executes) an EPT pointer load seam.
pub fn run_ept_pointer_cpu_seam(
    tables: &EptProgrammedTables,
    vmcs_phys: Option<u64>,
) -> Result<EptCpuSeamOutcome, CpuSeamError> {
    validate_ept_tables(tables)?;
    if !cpuid_ept_available() {
        return Ok(EptCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SkippedNoHardware,
            ept_pointer: encode_ept_pointer(tables.root_table_phys),
        });
    }
    let ept_pointer = encode_ept_pointer(tables.root_table_phys);
    let disposition = execute_ept_pointer_if_enabled(ept_pointer, vmcs_phys)?;
    Ok(EptCpuSeamOutcome {
        disposition,
        ept_pointer,
    })
}

/// Validates (and optionally executes) a VT-d enable seam.
pub fn run_vtd_enable_cpu_seam(
    tables: &VtdProgrammedTables,
) -> Result<VtdCpuSeamOutcome, CpuSeamError> {
    if tables.assignments.is_empty() && tables.interrupt_remapping {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VT-d interrupt remapping requires device assignments",
        ));
    }
    if !cpuid_vtd_available() {
        return Ok(VtdCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SkippedNoHardware,
            interrupt_remapping: tables.interrupt_remapping,
        });
    }
    let disposition = execute_vtd_enable_if_enabled(tables.interrupt_remapping)?;
    Ok(VtdCpuSeamOutcome {
        disposition,
        interrupt_remapping: tables.interrupt_remapping,
    })
}

/// Validates (and optionally executes) a VMX launch seam.
pub fn run_vmx_launch_cpu_seam(
    vmcs_phys: u64,
    fields: &VmcsProgrammedFields,
    guest_vm_id: hv_types::VmId,
) -> Result<VmxLaunchCpuSeamOutcome, CpuSeamError> {
    validate_vmx_launch_inputs(vmcs_phys, fields)?;
    if !cpuid_vmx_available() || !cpuid_ept_available() {
        return Ok(VmxLaunchCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SkippedNoHardware,
            guest_vm_id,
        });
    }
    let disposition = execute_vmx_launch_if_enabled(vmcs_phys, fields)?;
    Ok(VmxLaunchCpuSeamOutcome {
        disposition,
        guest_vm_id,
    })
}

/// Outcome of a multi-partition VMX launch CPU seam batch.
#[cfg(feature = "datapath-guests")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiVmxLaunchCpuSeamOutcome {
    /// Per-partition launch seam outcomes in planning order.
    pub launches: alloc::vec::Vec<VmxLaunchCpuSeamOutcome>,
}

/// Validates (and optionally executes) VMX launch seams for multiple partitions.
#[cfg(feature = "datapath-guests")]
pub fn run_multi_vmx_launch_cpu_seam(
    launches: &[(u64, VmcsProgrammedFields, hv_types::VmId)],
) -> Result<MultiVmxLaunchCpuSeamOutcome, CpuSeamError> {
    let mut outcomes = alloc::vec::Vec::with_capacity(launches.len());
    for (vmcs_phys, fields, guest_vm_id) in launches {
        outcomes.push(run_vmx_launch_cpu_seam(*vmcs_phys, fields, *guest_vm_id)?);
    }
    Ok(MultiVmxLaunchCpuSeamOutcome { launches: outcomes })
}

/// Outcome of a Gate D datapath runtime CPU seam batch.
#[cfg(feature = "datapath-runtime")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathRuntimeCpuSeamOutcome {
    /// How the seam batch completed.
    pub disposition: CpuInstructionDisposition,
    /// Whether VM-exit stub addresses were validated for all partitions.
    pub vmexit_stub_validated: bool,
    /// Number of partition launch contexts validated.
    pub partitions_validated: u32,
}

/// Validates (and optionally executes) Gate D datapath runtime seams for all partitions.
#[cfg(feature = "datapath-runtime")]
pub fn run_datapath_runtime_cpu_seam(
    launches: &[(u64, u64)],
) -> Result<DatapathRuntimeCpuSeamOutcome, CpuSeamError> {
    if launches.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "datapath runtime seam requires at least one partition launch",
        ));
    }
    for (vmcs_phys, host_exit_phys) in launches {
        validate_datapath_live_inputs(*vmcs_phys, *host_exit_phys)?;
    }
    if !cpuid_vmx_available() || !cpuid_ept_available() {
        return Ok(DatapathRuntimeCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SkippedNoHardware,
            vmexit_stub_validated: true,
            partitions_validated: launches.len() as u32,
        });
    }
    let disposition = execute_datapath_runtime_if_enabled(launches)?;
    Ok(DatapathRuntimeCpuSeamOutcome {
        disposition,
        vmexit_stub_validated: true,
        partitions_validated: launches.len() as u32,
    })
}

#[cfg(feature = "datapath-runtime")]
fn execute_datapath_runtime_if_enabled(
    launches: &[(u64, u64)],
) -> Result<CpuInstructionDisposition, CpuSeamError> {
    #[cfg(feature = "execute-instructions")]
    {
        if crate::instructions::live_execution_environment_ready() {
            let _ = launches;
            return Ok(CpuInstructionDisposition::SeamValidated);
        }
    }
    let _ = launches;
    Ok(CpuInstructionDisposition::SeamValidated)
}

/// Outcome of a Gate D datapath guest execution CPU seam batch.
#[cfg(feature = "datapath-guest-execution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathGuestExecutionCpuSeamOutcome {
    /// How the seam batch completed.
    pub disposition: CpuInstructionDisposition,
    /// Whether VM-exit stub addresses were validated for all partitions.
    pub vmexit_stub_validated: bool,
    /// Number of partition launch contexts validated.
    pub partitions_validated: u32,
    /// Number of live VMLAUNCH attempts made when execution was enabled.
    pub vmlaunch_attempts: u32,
}

/// Validates (and optionally executes) live VMX guest code for all source-tree partitions.
#[cfg(feature = "datapath-guest-execution")]
pub fn run_datapath_guest_execution_cpu_seam(
    launches: &[(u64, &VmcsProgrammedFields, u64, hv_types::VmId)],
) -> Result<DatapathGuestExecutionCpuSeamOutcome, CpuSeamError> {
    if launches.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "datapath guest execution seam requires at least one partition launch",
        ));
    }
    for (vmcs_phys, fields, host_exit_phys, _) in launches {
        validate_vmx_launch_inputs(*vmcs_phys, fields)?;
        validate_datapath_live_inputs(*vmcs_phys, *host_exit_phys)?;
    }
    if !cpuid_vmx_available() || !cpuid_ept_available() {
        return Ok(DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SkippedNoHardware,
            vmexit_stub_validated: true,
            partitions_validated: launches.len() as u32,
            vmlaunch_attempts: 0,
        });
    }
    let (disposition, vmlaunch_attempts) = execute_datapath_guest_vmlaunch_fields_if_enabled(launches)?;
    Ok(DatapathGuestExecutionCpuSeamOutcome {
        disposition,
        vmexit_stub_validated: true,
        partitions_validated: launches.len() as u32,
        vmlaunch_attempts,
    })
}

#[cfg(feature = "datapath-guest-execution")]
fn execute_datapath_guest_vmlaunch_fields_if_enabled(
    launches: &[(u64, &VmcsProgrammedFields, u64, hv_types::VmId)],
) -> Result<(CpuInstructionDisposition, u32), CpuSeamError> {
    #[cfg(feature = "execute-instructions")]
    {
        if crate::instructions::live_execution_environment_ready() {
            let mut vmlaunch_attempts = 0u32;
            let mut all_executed = true;
            for (vmcs_phys, fields, _host_exit_phys, _vm_id) in launches {
                match crate::instructions::vmcs_fields::execute_vmcs_field_programming(*vmcs_phys, fields)
                {
                    Ok(()) => {}
                    Err(err)
                        if err.kind == CpuSeamErrorKind::Unavailable
                            || err.kind == CpuSeamErrorKind::ExecutionFailed => {
                        all_executed = false;
                        continue;
                    }
                    Err(err) => return Err(err),
                }
                vmlaunch_attempts = vmlaunch_attempts.saturating_add(1);
                match crate::instructions::vmlaunch::execute_vmlaunch(*vmcs_phys) {
                    Ok(()) => {}
                    Err(err)
                        if err.kind == CpuSeamErrorKind::Unavailable
                            || err.kind == CpuSeamErrorKind::ExecutionFailed => {
                        all_executed = false;
                    }
                    Err(err) => return Err(err),
                }
            }
            if all_executed && vmlaunch_attempts == launches.len() as u32 {
                return Ok((CpuInstructionDisposition::Executed, vmlaunch_attempts));
            }
            return Ok((CpuInstructionDisposition::SeamValidated, vmlaunch_attempts));
        }
    }
    let _ = launches;
    Ok((CpuInstructionDisposition::SeamValidated, 0))
}

/// Outcome of a Gate D datapath guest throughput CPU seam batch.
#[cfg(feature = "datapath-guest-throughput")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathGuestThroughputCpuSeamOutcome {
    /// How the seam batch completed.
    pub disposition: CpuInstructionDisposition,
    /// Whether VM-exit stub addresses were validated for all partitions.
    pub vmexit_stub_validated: bool,
    /// Number of partition launch contexts validated.
    pub partitions_validated: u32,
    /// Number of completed measurement runs validated.
    pub measurement_runs_validated: u32,
    /// Whether live sustained guest relay measurement validated under VMX execution.
    #[cfg(feature = "datapath-guest-relay-live")]
    pub live_relay_validated: bool,
    /// In-VM relay frames read from guest boot-info counters (Phase 29 measurement).
    #[cfg(feature = "datapath-guest-relay-measurement")]
    pub in_vm_relay_frames: u64,
}

/// Validates (and optionally executes) live in-VM guest throughput measurement.
#[cfg(feature = "datapath-guest-throughput")]
pub fn run_datapath_guest_throughput_cpu_seam(
    execution_seam: &DatapathGuestExecutionCpuSeamOutcome,
    measurement_runs: u32,
    in_vm_relay_frames: u64,
    expected_relay_frames: u64,
) -> Result<DatapathGuestThroughputCpuSeamOutcome, CpuSeamError> {
    if execution_seam.partitions_validated == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "datapath guest throughput seam requires guest execution context",
        ));
    }
    if measurement_runs == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "datapath guest throughput seam requires at least one measurement run",
        ));
    }
    if !execution_seam.vmexit_stub_validated {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "datapath guest throughput seam requires validated VM-exit stubs",
        ));
    }
    let disposition = if !cpuid_vmx_available()
        || !cpuid_ept_available()
        || execution_seam.disposition == CpuInstructionDisposition::SkippedNoHardware
    {
        CpuInstructionDisposition::SkippedNoHardware
    } else {
        #[cfg(feature = "datapath-guest-relay-measurement")]
        {
            if execution_seam.disposition == CpuInstructionDisposition::Executed
                && expected_relay_frames > 0
                && in_vm_relay_frames >= expected_relay_frames
            {
                CpuInstructionDisposition::Executed
            } else {
                CpuInstructionDisposition::SeamValidated
            }
        }
        #[cfg(not(feature = "datapath-guest-relay-measurement"))]
        {
            // Live in-VM relay measurement is deferred; validate measurement plan only.
            CpuInstructionDisposition::SeamValidated
        }
    };
    Ok(DatapathGuestThroughputCpuSeamOutcome {
        disposition,
        vmexit_stub_validated: execution_seam.vmexit_stub_validated,
        partitions_validated: execution_seam.partitions_validated,
        measurement_runs_validated: measurement_runs,
        #[cfg(feature = "datapath-guest-relay-live")]
        live_relay_validated: execution_seam.disposition == CpuInstructionDisposition::Executed,
        #[cfg(feature = "datapath-guest-relay-measurement")]
        in_vm_relay_frames,
    })
}

/// Validates (and optionally executes) a Gate D datapath live seam.
#[cfg(feature = "datapath-live")]
pub fn run_datapath_live_cpu_seam(
    vmcs_phys: u64,
    host_exit_phys: u64,
) -> Result<DatapathLiveCpuSeamOutcome, CpuSeamError> {
    validate_datapath_live_inputs(vmcs_phys, host_exit_phys)?;
    if !cpuid_vmx_available() || !cpuid_ept_available() {
        return Ok(DatapathLiveCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SkippedNoHardware,
            vmexit_stub_validated: true,
        });
    }
    let disposition = execute_datapath_live_if_enabled(vmcs_phys, host_exit_phys)?;
    Ok(DatapathLiveCpuSeamOutcome {
        disposition,
        vmexit_stub_validated: true,
    })
}

#[cfg(feature = "datapath-live")]
fn validate_datapath_live_inputs(vmcs_phys: u64, host_exit_phys: u64) -> Result<(), CpuSeamError> {
    if vmcs_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "datapath live seam requires a non-zero VMCS address",
        ));
    }
    if host_exit_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "datapath live seam requires a non-zero host exit stub address",
        ));
    }
    Ok(())
}

#[cfg(feature = "datapath-live")]
fn execute_datapath_live_if_enabled(
    vmcs_phys: u64,
    host_exit_phys: u64,
) -> Result<CpuInstructionDisposition, CpuSeamError> {
    #[cfg(feature = "execute-instructions")]
    {
        if crate::instructions::live_execution_environment_ready() {
            let _ = (vmcs_phys, host_exit_phys);
            return Ok(CpuInstructionDisposition::SeamValidated);
        }
    }
    let _ = (vmcs_phys, host_exit_phys);
    Ok(CpuInstructionDisposition::SeamValidated)
}

fn validate_vmx_launch_inputs(
    vmcs_phys: u64,
    fields: &VmcsProgrammedFields,
) -> Result<(), CpuSeamError> {
    if vmcs_phys == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMX launch requires a non-zero VMCS address",
        ));
    }
    if fields.fields.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMX launch requires programmed VMCS fields",
        ));
    }
    Ok(())
}

fn execute_vmx_launch_if_enabled(
    vmcs_phys: u64,
    fields: &VmcsProgrammedFields,
) -> Result<CpuInstructionDisposition, CpuSeamError> {
    #[cfg(feature = "execute-instructions")]
    {
        match crate::instructions::vmcs_fields::execute_vmcs_field_programming(vmcs_phys, fields) {
            Ok(()) => {}
            Err(err) if err.kind == CpuSeamErrorKind::Unavailable => {}
            Err(err) => return Err(err),
        }
        match crate::instructions::vmlaunch::execute_vmlaunch(vmcs_phys) {
            Ok(()) => return Ok(CpuInstructionDisposition::Executed),
            Err(err) if err.kind == CpuSeamErrorKind::Unavailable => {}
            Err(err) => return Err(err),
        }
    }
    let _ = (vmcs_phys, fields);
    Ok(CpuInstructionDisposition::SeamValidated)
}

fn validate_vmxon_region(region: &VmxonProgrammedRegion) -> Result<(), CpuSeamError> {
    if region.bytes.len() < VMXON_REGION_MIN_BYTES as usize {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMXON region smaller than one page",
        ));
    }
    let revision = region
        .bytes
        .get(0..VMXON_REVISION_PREFIX_BYTES)
        .ok_or_else(|| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "VMXON region missing revision prefix",
            )
        })?
        .try_into()
        .map_err(|_| {
            CpuSeamError::new(
                CpuSeamErrorKind::InvalidInput,
                "VMXON revision prefix has invalid length",
            )
        })
        .map(u32::from_le_bytes)?;
    if revision == 0 {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "VMXON revision prefix must be non-zero",
        ));
    }
    Ok(())
}

fn validate_ept_tables(tables: &EptProgrammedTables) -> Result<(), CpuSeamError> {
    if tables.root_table.len() < EPT_ROOT_TABLE_BYTES as usize {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "EPT root table smaller than one page",
        ));
    }
    if tables.mappings.is_empty() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "EPT tables must contain at least one mapping",
        ));
    }
    Ok(())
}

fn encode_ept_pointer(root_table_phys: u64) -> u64 {
    let memory_type = EPT_POINTER_MEMORY_TYPE_WB << EPT_POINTER_MEMORY_TYPE_SHIFT;
    let page_walk_length = EPT_POINTER_PAGE_WALK_LENGTH << EPT_POINTER_PAGE_WALK_LENGTH_SHIFT;
    root_table_phys | memory_type | page_walk_length
}

fn execute_vmxon_if_enabled(host_phys: u64) -> Result<CpuInstructionDisposition, CpuSeamError> {
    #[cfg(feature = "execute-instructions")]
    {
        match crate::instructions::vmx::execute_vmxon(host_phys) {
            Ok(()) => return Ok(CpuInstructionDisposition::Executed),
            Err(err) if err.kind == CpuSeamErrorKind::Unavailable => {}
            Err(err) => return Err(err),
        }
    }
    let _ = host_phys;
    Ok(CpuInstructionDisposition::SeamValidated)
}

fn execute_ept_pointer_if_enabled(
    ept_pointer: u64,
    vmcs_phys: Option<u64>,
) -> Result<CpuInstructionDisposition, CpuSeamError> {
    #[cfg(feature = "execute-instructions")]
    if let Some(vmcs_phys) = vmcs_phys {
        match crate::instructions::ept::execute_ept_pointer_load(ept_pointer, vmcs_phys) {
            Ok(()) => return Ok(CpuInstructionDisposition::Executed),
            Err(err) if err.kind == CpuSeamErrorKind::Unavailable => {}
            Err(err) => return Err(err),
        }
    }
    let _ = (ept_pointer, vmcs_phys);
    Ok(CpuInstructionDisposition::SeamValidated)
}

fn execute_vtd_enable_if_enabled(
    interrupt_remapping: bool,
) -> Result<CpuInstructionDisposition, CpuSeamError> {
    #[cfg(feature = "execute-instructions")]
    {
        match crate::instructions::vtd::execute_vtd_enable(interrupt_remapping) {
            Ok(()) => return Ok(CpuInstructionDisposition::Executed),
            Err(err) if err.kind == CpuSeamErrorKind::Unavailable => {}
            Err(err) => return Err(err),
        }
    }
    let _ = interrupt_remapping;
    Ok(CpuInstructionDisposition::SeamValidated)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use crate::cpuid::{cpuid_ept_available, cpuid_vtd_available};
    use hv_ept::{
        plan_ept_init, program_ept_tables, EptProgrammedMapping, EptProgrammedTables,
        EPT_PAGE_OFFSET_MASK, EPT_PAGE_SIZE_BYTES,
    };
    use hv_platform_model::plan_static_platform_ir;
    use hv_vmx::{plan_vmx_init, program_vmxon_region, REFERENCE_VMXON_REVISION};
    use hv_vtd::{plan_vtd_init, program_vtd_tables};

    fn reference_vmxon_region() -> VmxonProgrammedRegion {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        program_vmxon_region(&plan, REFERENCE_VMXON_REVISION).expect("program")
    }

    fn reference_ept_tables() -> EptProgrammedTables {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
        program_ept_tables(&plan).expect("program")
    }

    fn reference_vtd_tables() -> VtdProgrammedTables {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_vtd_init(&layout, true).expect("vtd");
        program_vtd_tables(&plan).expect("program")
    }

    #[test]
    fn run_vmxon_cpu_seam_validates_reference_region() {
        let region = reference_vmxon_region();
        let outcome = run_vmxon_cpu_seam(&region).expect("seam");
        if cpuid_vmx_available() {
            assert_eq!(outcome.disposition, CpuInstructionDisposition::SeamValidated);
        } else {
            assert_eq!(
                outcome.disposition,
                CpuInstructionDisposition::SkippedNoHardware
            );
        }
        assert_eq!(outcome.vmxon_phys, region.host_phys);
    }

    #[test]
    fn run_ept_pointer_cpu_seam_with_vmcs_phys_covers_execute_path() {
        let tables = reference_ept_tables();
        let outcome = run_ept_pointer_cpu_seam(&tables, Some(0x4000)).expect("seam");
        if cpuid_ept_available() {
            assert_eq!(outcome.disposition, CpuInstructionDisposition::SeamValidated);
        } else {
            assert_eq!(
                outcome.disposition,
                CpuInstructionDisposition::SkippedNoHardware
            );
        }
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn run_ept_pointer_cpu_seam_propagates_execution_failure_with_live_env() {
        use crate::instructions::environment::test_force_live_environment_ready;
        let tables = reference_ept_tables();
        test_force_live_environment_ready(true);
        let result = run_ept_pointer_cpu_seam(&tables, Some(0x5000));
        test_force_live_environment_ready(false);
        if cpuid_ept_available() {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
        }
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn run_vmxon_cpu_seam_propagates_execution_failure_in_live_env() {
        use crate::instructions::environment::test_force_live_environment_ready;
        let region = reference_vmxon_region();
        test_force_live_environment_ready(true);
        let result = run_vmxon_cpu_seam(&region);
        test_force_live_environment_ready(false);
        if cpuid_vmx_available() {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn run_vmxon_cpu_seam_rejects_zero_revision() {
        let mut region = reference_vmxon_region();
        if let Some(prefix) = region.bytes.get_mut(0..VMXON_REVISION_PREFIX_BYTES) {
            prefix.copy_from_slice(&0u32.to_le_bytes());
        }
        assert!(run_vmxon_cpu_seam(&region).is_err());
    }

    #[test]
    fn run_ept_pointer_cpu_seam_validates_reference_tables() {
        let tables = reference_ept_tables();
        let outcome = run_ept_pointer_cpu_seam(&tables, None).expect("seam");
        if cpuid_ept_available() {
            assert_eq!(outcome.disposition, CpuInstructionDisposition::SeamValidated);
            assert_ne!(outcome.ept_pointer & EPT_PAGE_OFFSET_MASK, 0);
        } else {
            assert_eq!(
                outcome.disposition,
                CpuInstructionDisposition::SkippedNoHardware
            );
        }
    }

    #[test]
    fn run_vtd_enable_cpu_seam_accepts_reference_tables() {
        let tables = reference_vtd_tables();
        let outcome = run_vtd_enable_cpu_seam(&tables).expect("seam");
        if cpuid_vtd_available() {
            assert_eq!(outcome.disposition, CpuInstructionDisposition::SeamValidated);
        } else {
            assert_eq!(
                outcome.disposition,
                CpuInstructionDisposition::SkippedNoHardware
            );
        }
        assert!(outcome.interrupt_remapping);
    }

    #[test]
    fn run_vmxon_cpu_seam_rejects_short_region() {
        let region = VmxonProgrammedRegion {
            host_phys: 0x1000,
            bytes: alloc::vec![0u8; 512],
        };
        assert!(run_vmxon_cpu_seam(&region).is_err());
    }

    #[test]
    fn run_ept_pointer_cpu_seam_rejects_empty_mappings() {
        let tables = EptProgrammedTables {
            root_table_phys: 0x2000,
            root_table: alloc::vec![0u8; EPT_ROOT_TABLE_BYTES as usize],
            mappings: alloc::vec::Vec::new(),
        };
        assert!(run_ept_pointer_cpu_seam(&tables, None).is_err());
    }

    #[test]
    fn run_ept_pointer_cpu_seam_rejects_short_root_table() {
        let tables = EptProgrammedTables {
            root_table_phys: 0x2000,
            root_table: alloc::vec![0u8; 512],
            mappings: alloc::vec![EptProgrammedMapping {
                guest_phys: 0,
                host_phys: 0,
                size_bytes: EPT_PAGE_SIZE_BYTES,
                encoded_entry: 1,
            }],
        };
        assert!(run_ept_pointer_cpu_seam(&tables, None).is_err());
    }

    #[test]
    fn run_vtd_enable_cpu_seam_rejects_empty_assignments_with_ir() {
        let tables = VtdProgrammedTables {
            interrupt_remapping: true,
            assignments: alloc::vec::Vec::new(),
        };
        assert!(run_vtd_enable_cpu_seam(&tables).is_err());
    }

    #[test]
    fn cpu_seams_skip_when_cpuid_reports_unavailable() {
        use crate::cpuid::{
            test_force_ept_unavailable, test_force_vmx_unavailable, test_force_vtd_unavailable,
        };

        let region = reference_vmxon_region();
        test_force_vmx_unavailable(true);
        let vmx = run_vmxon_cpu_seam(&region).expect("vmx seam");
        assert_eq!(vmx.disposition, CpuInstructionDisposition::SkippedNoHardware);
        test_force_vmx_unavailable(false);

        let tables = reference_ept_tables();
        test_force_ept_unavailable(true);
        let ept = run_ept_pointer_cpu_seam(&tables, None).expect("ept seam");
        assert_eq!(ept.disposition, CpuInstructionDisposition::SkippedNoHardware);
        test_force_ept_unavailable(false);

        let vtd_tables = reference_vtd_tables();
        test_force_vtd_unavailable(true);
        let vtd = run_vtd_enable_cpu_seam(&vtd_tables).expect("vtd seam");
        assert_eq!(vtd.disposition, CpuInstructionDisposition::SkippedNoHardware);
        test_force_vtd_unavailable(false);
    }

    #[test]
    fn ept_and_vtd_cpu_seams_fall_back_without_live_environment() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = hv_config_model::compile_config_from_str(yaml).expect("compile");
        let layout = hv_platform_model::plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let ept_plan = plan_ept_init(&layout, &vmx_plan).expect("ept");
        let ept_tables = program_ept_tables(&ept_plan).expect("program ept");
        let vtd_plan = plan_vtd_init(&layout, true).expect("vtd");
        let vtd_tables = program_vtd_tables(&vtd_plan).expect("program vtd");

        let ept = run_ept_pointer_cpu_seam(&ept_tables, None).expect("ept seam");
        if cpuid_ept_available() {
            assert_eq!(ept.disposition, CpuInstructionDisposition::SeamValidated);
        } else {
            assert_eq!(ept.disposition, CpuInstructionDisposition::SkippedNoHardware);
        }
        let vtd = run_vtd_enable_cpu_seam(&vtd_tables).expect("vtd seam");
        if cpuid_vtd_available() {
            assert_eq!(vtd.disposition, CpuInstructionDisposition::SeamValidated);
        } else {
            assert_eq!(vtd.disposition, CpuInstructionDisposition::SkippedNoHardware);
        }
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn execute_instruction_feature_falls_back_without_live_environment() {
        let region = reference_vmxon_region();
        let outcome = run_vmxon_cpu_seam(&region).expect("seam");
        if cpuid_vmx_available() {
            assert_eq!(outcome.disposition, CpuInstructionDisposition::SeamValidated);
        }
    }

    #[cfg(feature = "datapath-guest-execution")]
    #[test]
    fn run_datapath_guest_execution_cpu_seam_rejects_empty_batch() {
        assert!(run_datapath_guest_execution_cpu_seam(&[]).is_err());
    }

    #[cfg(all(feature = "datapath-guest-execution", feature = "execute-instructions"))]
    #[test]
    fn run_datapath_guest_execution_cpu_seam_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        use hv_vmx::program_vmcs_fields;
        use hv_vmx::plan_vmx_launch;
        use hv_vmx::DEFAULT_SMOKE_GUEST_PARTITION_ID;

        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let launch_plan =
            plan_vmx_launch(&layout, &vmx_plan, DEFAULT_SMOKE_GUEST_PARTITION_ID).expect("launch");
        let fields = program_vmcs_fields(&launch_plan);
        let launches = [(0x5000_u64, &fields, 0x6000_u64, hv_types::VmId::new(0))];
        test_force_live_environment_ready(true);
        let outcome = run_datapath_guest_execution_cpu_seam(&launches).expect("guest execution");
        test_force_live_environment_ready(false);
        assert_eq!(outcome.partitions_validated, 1);
        assert_ne!(outcome.disposition, CpuInstructionDisposition::Executed);
    }

    #[cfg(feature = "datapath-guest-throughput")]
    #[test]
    fn run_datapath_guest_throughput_cpu_seam_rejects_missing_execution_context() {
        let execution = DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SeamValidated,
            vmexit_stub_validated: true,
            partitions_validated: 0,
            vmlaunch_attempts: 0,
        };
        assert!(run_datapath_guest_throughput_cpu_seam(&execution, 1, 0, 0).is_err());
    }

    #[cfg(feature = "datapath-guest-throughput")]
    #[test]
    fn run_datapath_guest_throughput_cpu_seam_rejects_zero_measurement_runs() {
        let execution = DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SeamValidated,
            vmexit_stub_validated: true,
            partitions_validated: 1,
            vmlaunch_attempts: 0,
        };
        assert!(run_datapath_guest_throughput_cpu_seam(&execution, 0, 0, 0).is_err());
    }

    #[cfg(feature = "datapath-guest-throughput")]
    #[test]
    fn run_datapath_guest_throughput_cpu_seam_validates_measurement_plan() {
        let execution = DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::SeamValidated,
            vmexit_stub_validated: true,
            partitions_validated: 3,
            vmlaunch_attempts: 0,
        };
        let outcome =
            run_datapath_guest_throughput_cpu_seam(&execution, 5, 0, 0).expect("guest throughput");
        assert_eq!(outcome.partitions_validated, 3);
        assert_eq!(outcome.measurement_runs_validated, 5);
        assert!(outcome.vmexit_stub_validated);
        assert_ne!(outcome.disposition, CpuInstructionDisposition::Executed);
    }

    #[cfg(all(feature = "datapath-guest-throughput", feature = "datapath-guest-relay-measurement"))]
    #[test]
    fn run_datapath_guest_throughput_cpu_seam_executed_with_in_vm_relay_frames() {
        use crate::cpuid::{cpuid_ept_available, cpuid_vmx_available};

        if !cpuid_vmx_available() || !cpuid_ept_available() {
            return;
        }
        let execution = DatapathGuestExecutionCpuSeamOutcome {
            disposition: CpuInstructionDisposition::Executed,
            vmexit_stub_validated: true,
            partitions_validated: 3,
            vmlaunch_attempts: 3,
        };
        let outcome = run_datapath_guest_throughput_cpu_seam(&execution, 5, 64, 64)
            .expect("guest throughput measurement");
        assert_eq!(outcome.disposition, CpuInstructionDisposition::Executed);
        assert_eq!(outcome.in_vm_relay_frames, 64);
    }
}
