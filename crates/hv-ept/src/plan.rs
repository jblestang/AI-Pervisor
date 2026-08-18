//! EPT init plan derived from static platform layout.

use alloc::vec::Vec;

use hv_datapath::{plan_e1000_mmio_guest_phys, E1000_MMIO_SIZE_BYTES};
use hv_platform_model::StaticPlatformIR;
use hv_types::{ByteSize, HostPhysAddr};
use hv_vmx::{VMXON_REGION_MIN_BYTES, VmxInitPlan};

use crate::constants::{EPT_PAGE_SIZE_BYTES, EPT_ROOT_TABLE_BYTES};
use crate::error::{EptError, EptErrorKind};

/// One identity-mapped guest/host region in the EPT plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EptIdentityMapping {
    /// Guest physical base address.
    pub guest_phys: HostPhysAddr,
    /// Host physical base address.
    pub host_phys: HostPhysAddr,
    /// Mapping size in bytes.
    pub size_bytes: ByteSize,
}

/// Planned EPT hierarchy metadata for backend initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EptInitPlan {
    /// Identity mappings covering guest private and IPC shared regions.
    pub identity_mappings: Vec<EptIdentityMapping>,
    /// Host physical base of the EPT root table inside the hypervisor reserve.
    pub root_table_phys: HostPhysAddr,
    /// EPT root table size in bytes.
    pub root_table_bytes: ByteSize,
}

/// Builds an EPT init plan from static platform layout and the VMX init plan.
pub fn plan_ept_init(
    layout: &StaticPlatformIR,
    vmx_plan: &VmxInitPlan,
) -> Result<EptInitPlan, EptError> {
    let mut identity_mappings = Vec::new();
    for region in &layout.guest_memory {
        push_identity_mapping(
            &mut identity_mappings,
            region.host_phys,
            region.host_phys,
            region.size,
        )?;
    }
    for region in &layout.ipc_memory {
        push_identity_mapping(
            &mut identity_mappings,
            region.host_phys,
            region.host_phys,
            region.size,
        )?;
    }
    for device in &layout.pci_devices {
        if device.kind == "nic_e1000" {
            let guest_phys = plan_e1000_mmio_guest_phys(device.vm_id).map_err(|_| {
                planning_error("failed to plan e1000 mmio guest mapping")
            })?;
            let host_phys = HostPhysAddr::new(guest_phys.raw());
            push_identity_mapping(
                &mut identity_mappings,
                host_phys,
                host_phys,
                ByteSize::new(E1000_MMIO_SIZE_BYTES),
            )?;
        }
    }

    let table_end = VMXON_REGION_MIN_BYTES
        .checked_add(EPT_ROOT_TABLE_BYTES)
        .ok_or(planning_error("EPT root table end offset overflow"))?;
    if table_end > vmx_plan.vmxon_region_bytes.bytes() {
        return Err(planning_error(
            "hypervisor reserve too small for VMXON and EPT root table",
        ));
    }
    let root_table_phys = vmx_plan
        .vmxon_region_phys
        .raw()
        .checked_add(VMXON_REGION_MIN_BYTES)
        .ok_or(planning_error("EPT root table address overflow"))?;

    Ok(EptInitPlan {
        identity_mappings,
        root_table_phys: HostPhysAddr::new(root_table_phys),
        root_table_bytes: ByteSize::new(EPT_ROOT_TABLE_BYTES),
    })
}

fn push_identity_mapping(
    mappings: &mut Vec<EptIdentityMapping>,
    guest_phys: HostPhysAddr,
    host_phys: HostPhysAddr,
    size: ByteSize,
) -> Result<(), EptError> {
    if size.bytes() == 0 {
        return Err(planning_error("EPT mapping size must not be zero"));
    }
    if guest_phys.raw() % EPT_PAGE_SIZE_BYTES != 0 {
        return Err(planning_error("EPT guest mapping base is not page aligned"));
    }
    if host_phys.raw() % EPT_PAGE_SIZE_BYTES != 0 {
        return Err(planning_error("EPT host mapping base is not page aligned"));
    }
    if size.bytes() % EPT_PAGE_SIZE_BYTES != 0 {
        return Err(planning_error("EPT mapping size is not page aligned"));
    }
    mappings.push(EptIdentityMapping {
        guest_phys,
        host_phys,
        size_bytes: size,
    });
    Ok(())
}

fn planning_error(message: &'static str) -> EptError {
    EptError::new(EptErrorKind::Planning, message)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;
    use hv_vmx::plan_vmx_init;

    #[test]
    fn plan_ept_init_builds_identity_mappings_for_reference_layout() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx plan");
        let ept_plan = plan_ept_init(&layout, &vmx_plan).expect("ept plan");
        assert_eq!(
            ept_plan.identity_mappings.len(),
            layout.guest_memory.len() + layout.ipc_memory.len() + layout.pci_devices.len()
        );
        assert!(ept_plan.root_table_bytes.bytes() >= EPT_ROOT_TABLE_BYTES);
    }

    #[test]
    fn plan_ept_init_rejects_reserve_too_small_for_tables() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let mut layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        layout.hypervisor_reserve.size = ByteSize::new(VMXON_REGION_MIN_BYTES);
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx plan");
        assert!(plan_ept_init(&layout, &vmx_plan).is_err());
    }
}
