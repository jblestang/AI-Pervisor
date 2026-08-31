//! VMX initialization after Gate B boot validation.

use hv_boot_abi::RequirementsSnapshot;
use hv_config_model::{FeatureRequirement, PlatformRequirements};
use hv_platform_model::{PlatformWarning, ValidatedPlatform};
use hv_types::SHA256_DIGEST_BYTES;
use hv_vmx::{
    init_vmx, plan_vmx_init, vmx_init_required, MockVmxBackend, VmxBackend, VmxError, VmxInitPlan,
};

use crate::boot::boot_check;
use crate::error::{BootCheckError, BootCheckErrorKind};
use crate::snapshot::platform_requirements_from_snapshot;
use crate::transfer::boot_from_transfer;

/// Result of boot validation followed by VMX init planning and mock-backed enablement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootAndVmxResult {
    /// Validated platform snapshot.
    pub validated: ValidatedPlatform,
    /// Non-fatal platform warnings from validation.
    pub warnings: alloc::vec::Vec<PlatformWarning>,
    /// VMX init plan derived from embedded layout metadata.
    pub vmx_plan: VmxInitPlan,
}

/// Runs transfer boot checks and mock-backed VMX init using snapshot layout metadata.
pub fn boot_from_transfer_and_init_vmx(
    transfer: &[u8],
    snapshot: &RequirementsSnapshot,
) -> Result<BootAndVmxResult, BootCheckError> {
    let requirements = platform_requirements_from_snapshot(snapshot)?;
    let (validated, warnings) =
        boot_from_transfer(transfer, &snapshot.config_digest, &requirements)?;
    let plan = vmx_plan_from_snapshot(snapshot).map_err(map_vmx_error)?;
    let mut backend = MockVmxBackend::default();
    init_vmx_if_required(&mut backend, &plan, &validated, requirements.vmx)?;
    Ok(BootAndVmxResult {
        validated,
        warnings,
        vmx_plan: plan,
    })
}

/// Runs boot checks from raw inputs and mock-backed VMX init.
pub fn boot_check_and_init_vmx(
    boot_info_bytes: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &PlatformRequirements,
    observation: &hv_platform_model::ObservationInputs,
    reserve_phys: u64,
    reserve_bytes: u64,
) -> Result<BootAndVmxResult, BootCheckError> {
    let (validated, warnings) = boot_check(
        boot_info_bytes,
        expected_config_digest,
        requirements,
        observation,
    )?;
    let plan = vmx_plan_from_reserve(reserve_phys, reserve_bytes).map_err(map_vmx_error)?;
    let mut backend = MockVmxBackend::default();
    init_vmx_if_required(&mut backend, &plan, &validated, requirements.vmx)?;
    Ok(BootAndVmxResult {
        validated,
        warnings,
        vmx_plan: plan,
    })
}

fn vmx_plan_from_snapshot(snapshot: &RequirementsSnapshot) -> Result<VmxInitPlan, VmxError> {
    vmx_plan_from_reserve(
        snapshot.hypervisor_reserve_phys,
        snapshot.hypervisor_reserve_bytes,
    )
}

fn vmx_plan_from_reserve(reserve_phys: u64, reserve_bytes: u64) -> Result<VmxInitPlan, VmxError> {
    use hv_platform_model::PlannedHypervisorReserve;
    use hv_types::{ByteSize, HostPhysAddr};
    plan_vmx_init(&PlannedHypervisorReserve {
        host_phys: HostPhysAddr::new(reserve_phys),
        size: ByteSize::new(reserve_bytes),
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

fn map_vmx_error(err: VmxError) -> BootCheckError {
    BootCheckError::new(BootCheckErrorKind::Platform, err.message)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::snapshot::requirements_snapshot_from_platform;
    use hv_config_model::compile_config_from_str;
    use hv_loader::{
        build_hypervisor_transfer, build_loader_handoff, encode_qemu_reference_firmware,
        LoaderHandoffInput,
    };
    use hv_observation_types::{
        CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
        CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
        CPUID_80000007_EDX_INVARIANT_TSC_BIT,
    };
    use hv_platform_model::plan_static_platform_ir;
    use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

    #[test]
    fn boot_from_transfer_and_init_vmx_accepts_reference_transfer() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let firmware = encode_qemu_reference_firmware();
        let handoff = build_loader_handoff(
            &LoaderHandoffInput::with_default_descriptor_size(
                compiled.digest.bytes,
                {
                    let mut memory_map = alloc::vec![0u8; 48];
                    memory_map[0..4]
                        .copy_from_slice(&hv_boot_abi::EFI_MEMORY_CONVENTIONAL.to_le_bytes());
                    memory_map[24..32].copy_from_slice(&(2_097_152u64).to_le_bytes());
                    memory_map
                },
                firmware
                    .bytes
                    .get(0x1000..0x1000 + 36)
                    .expect("rsdp")
                    .to_vec(),
                CpuidSnapshot {
                    leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
                    leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
                    leaf1_ebx: (4 << 16) | 4,
                    leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
                    leaf80000008_ecx: Some(3),
                    leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
                    leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
                },
                alloc::vec![
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(3),
                        PciFunction::new(0),
                    ),
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(4),
                        PciFunction::new(0),
                    ),
                ],
            ),
            &firmware,
        )
        .expect("handoff");
        let transfer = build_hypervisor_transfer(&handoff).expect("transfer");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let snapshot = requirements_snapshot_from_platform(
            &compiled.requirements,
            compiled.digest.bytes,
            layout.hypervisor_reserve.host_phys.raw(),
            layout.hypervisor_reserve.size.bytes(),
        )
        .expect("snapshot");
        let result = boot_from_transfer_and_init_vmx(&transfer, &snapshot).expect("boot+vmx");
        assert!(result.validated.observed.vmx);
    }
}
