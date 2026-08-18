//! VMCS field planning and programming for VMX launch bring-up.

use alloc::vec;
use alloc::vec::Vec;

use hv_platform_model::{PlannedGuestMemory, StaticPlatformIR};
use hv_types::{ByteSize, HostPhysAddr};

use crate::constants::VMXON_REGION_MIN_BYTES;
use crate::error::{VmxError, VmxErrorKind};
use crate::launch_constants::{
    CPU_BASED_ACTIVATE_SECONDARY_CONTROLS, SECONDARY_ENABLE_EPT, VMCS_CPU_BASED_VM_EXEC_CONTROL,
    VMCS_GUEST_CR3, VMCS_GUEST_RIP, VMCS_GUEST_RSP, VMCS_HOST_CR3, VMCS_HOST_RIP,
    VMCS_HOST_RSP, VMCS_PIN_BASED_VM_EXEC_CONTROL, VMCS_SECONDARY_VM_EXEC_CONTROL,
    VMCS_VM_ENTRY_CONTROLS, VMCS_VM_EXIT_CONTROLS, VM_ENTRY_IA32E_MODE,
    VM_EXIT_HOST_ADDR_SPACE_SIZE, VMX_HOST_EXIT_STUB_OFFSET,
};
use crate::plan::VmxInitPlan;

/// Default smoke-guest partition id for single-partition launch bring-up.
pub const DEFAULT_SMOKE_GUEST_PARTITION_ID: &str = "in";

/// Planned guest entry and host exit state for VMX launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmxLaunchPlan {
    /// Target partition identifier.
    pub partition_id: alloc::string::String,
    /// Guest VM identifier.
    pub vm_id: hv_types::VmId,
    /// Guest physical entry address (identity-mapped to host).
    pub guest_entry_phys: HostPhysAddr,
    /// Guest initial stack pointer.
    pub guest_stack_phys: HostPhysAddr,
    /// Guest page tables root (identity bring-up uses flat mapping).
    pub guest_cr3: HostPhysAddr,
    /// Host physical VM-exit handler entry.
    pub host_exit_phys: HostPhysAddr,
    /// Host stack pointer for VM-exits.
    pub host_stack_phys: HostPhysAddr,
    /// Host CR3 for VM-exits.
    pub host_cr3: HostPhysAddr,
}

/// One encoded VMCS field assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmcsProgrammedField {
    /// Encoded VMCS field number.
    pub field: u32,
    /// Field value.
    pub value: u64,
}

/// Encoded VMCS fields ready for VMWRITE programming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmcsProgrammedFields {
    /// Ordered VMCS field assignments.
    pub fields: Vec<VmcsProgrammedField>,
}

/// Builds VMX launch plans for every guest partition in static platform layout order.
pub fn plan_vmx_launch_all_partitions(
    layout: &StaticPlatformIR,
    vmx_plan: &VmxInitPlan,
) -> Result<alloc::vec::Vec<VmxLaunchPlan>, VmxError> {
    let mut plans = alloc::vec::Vec::with_capacity(layout.guest_memory.len());
    for region in &layout.guest_memory {
        plans.push(plan_vmx_launch_for_region(layout, vmx_plan, region)?);
    }
    Ok(plans)
}

/// Builds a VMX launch plan for the given partition within static platform layout.
pub fn plan_vmx_launch(
    layout: &StaticPlatformIR,
    vmx_plan: &VmxInitPlan,
    partition_id: &str,
) -> Result<VmxLaunchPlan, VmxError> {
    let region = find_guest_region(layout, partition_id)?;
    plan_vmx_launch_for_region(layout, vmx_plan, region)
}

fn plan_vmx_launch_for_region(
    layout: &StaticPlatformIR,
    vmx_plan: &VmxInitPlan,
    region: &PlannedGuestMemory,
) -> Result<VmxLaunchPlan, VmxError> {
    let _ = layout;
    let guest_entry = region.host_phys;
    let guest_stack = advance_within_region(
        guest_entry,
        region.size,
        region.size.bytes().saturating_sub(16),
    )?;
    let host_exit = advance_within_reserve(
        vmx_plan.vmxon_region_phys,
        vmx_plan.vmxon_region_bytes,
        VMX_HOST_EXIT_STUB_OFFSET,
    )?;
    let host_stack = advance_within_reserve(
        vmx_plan.vmxon_region_phys,
        vmx_plan.vmxon_region_bytes,
        VMX_HOST_EXIT_STUB_OFFSET.saturating_add(0x100),
    )?;
    Ok(VmxLaunchPlan {
        partition_id: reference_partition_id_for_region(region),
        vm_id: region.vm_id,
        guest_entry_phys: guest_entry,
        guest_stack_phys: guest_stack,
        guest_cr3: guest_entry,
        host_exit_phys: host_exit,
        host_stack_phys: host_stack,
        host_cr3: vmx_plan.vmxon_region_phys,
    })
}

fn reference_partition_id_for_region(region: &PlannedGuestMemory) -> alloc::string::String {
    if !region.partition_id.is_empty() {
        return region.partition_id.clone();
    }
    match region.vm_id.raw() {
        0 => alloc::string::String::from("in"),
        1 => alloc::string::String::from("mid"),
        2 => alloc::string::String::from("out"),
        other => alloc::format!("vm{other}"),
    }
}

/// Encodes VMCS fields for a launch plan without executing VMWRITE.
pub fn program_vmcs_fields(plan: &VmxLaunchPlan) -> VmcsProgrammedFields {
    let fields = vec![
        VmcsProgrammedField {
            field: VMCS_PIN_BASED_VM_EXEC_CONTROL,
            value: 0,
        },
        VmcsProgrammedField {
            field: VMCS_CPU_BASED_VM_EXEC_CONTROL,
            value: CPU_BASED_ACTIVATE_SECONDARY_CONTROLS,
        },
        VmcsProgrammedField {
            field: VMCS_SECONDARY_VM_EXEC_CONTROL,
            value: SECONDARY_ENABLE_EPT,
        },
        VmcsProgrammedField {
            field: VMCS_VM_EXIT_CONTROLS,
            value: VM_EXIT_HOST_ADDR_SPACE_SIZE,
        },
        VmcsProgrammedField {
            field: VMCS_VM_ENTRY_CONTROLS,
            value: VM_ENTRY_IA32E_MODE,
        },
        VmcsProgrammedField {
            field: VMCS_GUEST_CR3,
            value: plan.guest_cr3.raw(),
        },
        VmcsProgrammedField {
            field: VMCS_GUEST_RSP,
            value: plan.guest_stack_phys.raw(),
        },
        VmcsProgrammedField {
            field: VMCS_GUEST_RIP,
            value: plan.guest_entry_phys.raw(),
        },
        VmcsProgrammedField {
            field: VMCS_HOST_CR3,
            value: plan.host_cr3.raw(),
        },
        VmcsProgrammedField {
            field: VMCS_HOST_RSP,
            value: plan.host_stack_phys.raw(),
        },
        VmcsProgrammedField {
            field: VMCS_HOST_RIP,
            value: plan.host_exit_phys.raw(),
        },
    ];
    VmcsProgrammedFields { fields }
}

/// Updates guest entry/stack/CR3 fields after resident guest image installation.
pub fn patch_guest_entry_in_fields(
    fields: &mut VmcsProgrammedFields,
    guest_entry_phys: u64,
    guest_stack_phys: u64,
) {
    for field in &mut fields.fields {
        match field.field {
            VMCS_GUEST_RIP => field.value = guest_entry_phys,
            VMCS_GUEST_RSP => field.value = guest_stack_phys,
            VMCS_GUEST_CR3 => field.value = guest_entry_phys & !0xFFF,
            _ => {}
        }
    }
}

fn find_guest_region<'a>(
    layout: &'a StaticPlatformIR,
    partition_id: &str,
) -> Result<&'a PlannedGuestMemory, VmxError> {
    if let Some(region) = layout
        .guest_memory
        .iter()
        .find(|region| region.partition_id == partition_id)
    {
        return Ok(region);
    }
    layout
        .guest_memory
        .iter()
        .min_by_key(|region| region.vm_id.raw())
        .ok_or_else(|| {
            VmxError::new(
                VmxErrorKind::Planning,
                "launch partition not found in static platform layout",
            )
        })
}

fn advance_within_region(
    base: HostPhysAddr,
    size: ByteSize,
    offset: u64,
) -> Result<HostPhysAddr, VmxError> {
    if offset >= size.bytes() {
        return Err(VmxError::new(
            VmxErrorKind::Planning,
            "launch offset exceeds guest region size",
        ));
    }
    let address = base
        .raw()
        .checked_add(offset)
        .ok_or_else(|| VmxError::new(VmxErrorKind::Planning, "guest region address overflow"))?;
    Ok(HostPhysAddr::new(address))
}

fn advance_within_reserve(
    base: HostPhysAddr,
    size: ByteSize,
    offset: u64,
) -> Result<HostPhysAddr, VmxError> {
    if offset >= size.bytes() {
        return Err(VmxError::new(
            VmxErrorKind::Planning,
            "launch offset exceeds hypervisor reserve size",
        ));
    }
    if offset < VMXON_REGION_MIN_BYTES {
        return Err(VmxError::new(
            VmxErrorKind::Planning,
            "host exit stub must not overlap VMXON region",
        ));
    }
    let address = base
        .raw()
        .checked_add(offset)
        .ok_or_else(|| {
            VmxError::new(
                VmxErrorKind::Planning,
                "hypervisor reserve address overflow",
            )
        })?;
    Ok(HostPhysAddr::new(address))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    use crate::plan::plan_vmx_init;

    #[test]
    fn plan_vmx_launch_accepts_reference_in_partition() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let launch = plan_vmx_launch(&layout, &vmx_plan, DEFAULT_SMOKE_GUEST_PARTITION_ID)
            .expect("launch plan");
        assert_eq!(launch.partition_id, DEFAULT_SMOKE_GUEST_PARTITION_ID);
        assert!(launch.guest_entry_phys.raw() > 0);
    }

    #[test]
    fn program_vmcs_fields_encodes_guest_and_host_state() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let launch = plan_vmx_launch(&layout, &vmx_plan, DEFAULT_SMOKE_GUEST_PARTITION_ID)
            .expect("launch plan");
        let fields = program_vmcs_fields(&launch);
        assert!(fields.fields.len() >= 10);
        assert!(fields
            .fields
            .iter()
            .any(|field| field.field == VMCS_GUEST_RIP && field.value == launch.guest_entry_phys.raw()));
    }

    #[test]
    fn plan_vmx_launch_all_partitions_covers_reference_topology() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let launches = plan_vmx_launch_all_partitions(&layout, &vmx_plan).expect("all");
        assert_eq!(launches.len(), 3);
    }

    #[test]
    fn plan_vmx_launch_falls_back_to_first_guest_for_unknown_partition_id() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let vmx_plan = plan_vmx_init(&layout.hypervisor_reserve).expect("vmx");
        let launch = plan_vmx_launch(&layout, &vmx_plan, "missing").expect("fallback launch");
        assert_eq!(launch.vm_id, hv_types::VmId::new(0));
    }
}
