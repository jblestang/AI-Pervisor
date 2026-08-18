//! CPU instruction seams for Gate C hardware bring-up.

use hv_ept::{
    EptProgrammedTables, EPT_POINTER_MEMORY_TYPE_SHIFT, EPT_POINTER_MEMORY_TYPE_WB,
    EPT_POINTER_PAGE_WALK_LENGTH, EPT_POINTER_PAGE_WALK_LENGTH_SHIFT, EPT_ROOT_TABLE_BYTES,
};
use hv_vmx::{VmxonProgrammedRegion, VMXON_REGION_MIN_BYTES};
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
}
