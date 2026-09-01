//! Requirements snapshot conversion for embedded hypervisor images.

use alloc::string::String;

use hv_boot_abi::{
    ExpectedPciSnapshot, LayoutGuestRegionSnapshot, LayoutHostNetworkSnapshot,
    LayoutIpcRegionSnapshot, LayoutPciSnapshot, LayoutSnapshot, RequirementsSnapshot,
    FEATURE_DISABLED, FEATURE_OPTIONAL, FEATURE_PREFERRED, FEATURE_REQUIRED,
    LAYOUT_DEVICE_KIND_NIC_E1000, LAYOUT_DEVICE_ROLE_DATAPATH_IN, LAYOUT_DEVICE_ROLE_DATAPATH_OUT,
    LAYOUT_DEVICE_ROLE_NONE, LAYOUT_HOST_NETWORK_BACKEND_TAP, LAYOUT_HOST_NETWORK_BACKEND_USER,
    LAYOUT_HOST_NETWORK_IFNAME_LEN, LAYOUT_HOST_NETWORK_NETDEV_LEN, MAX_LAYOUT_GUEST_REGIONS,
    MAX_LAYOUT_HOST_NETWORK_INTERFACES, MAX_LAYOUT_IPC_REGIONS, MAX_LAYOUT_PARTITION_ID_LEN,
    MAX_LAYOUT_PCI_DEVICES, MAX_REQUIREMENTS_PAGE_SIZES, MAX_REQUIREMENTS_PCI_DEVICES,
    REQUIREMENTS_ARCH_X86_64, SMT_POLICY_ALLOW_CROSS_PARTITION, SMT_POLICY_DISABLED,
    SMT_POLICY_EXCLUSIVE_CORE, SMT_POLICY_SAME_PARTITION_SIBLINGS,
};
use hv_config_model::{
    ExpectedPciDevice, FeatureRequirement, PageSizeSet, PlatformRequirements, SmtPolicy,
};
use hv_platform_model::{
    HostNetworkInterface, HostNetworkPlan, PlannedGuestMemory, PlannedHypervisorReserve,
    PlannedIpcMemory, PlannedPciDevice, StaticPlatformIR,
};
use hv_types::{
    ByteSize, HostPhysAddr, PciBdf, PciBus, PciDevice, PciFunction, PciSegment, VmId,
    SHA256_DIGEST_BYTES,
};

use crate::error::{BootCheckError, BootCheckErrorKind};

/// Converts an embedded requirements snapshot into runtime requirements.
pub fn platform_requirements_from_snapshot(
    snapshot: &RequirementsSnapshot,
) -> Result<PlatformRequirements, BootCheckError> {
    if snapshot.arch != REQUIREMENTS_ARCH_X86_64 {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "unsupported requirements snapshot architecture",
        ));
    }
    if snapshot.page_size_count as usize > MAX_REQUIREMENTS_PAGE_SIZES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "requirements snapshot page size count exceeds maximum",
        ));
    }
    if snapshot.expected_pci_count as usize > MAX_REQUIREMENTS_PCI_DEVICES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "requirements snapshot pci device count exceeds maximum",
        ));
    }

    let page_sizes = snapshot
        .page_sizes
        .get(0..snapshot.page_size_count as usize)
        .ok_or(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "requirements snapshot page sizes out of bounds",
        ))?
        .to_vec();
    let expected_pci_devices = snapshot
        .expected_pci
        .get(0..snapshot.expected_pci_count as usize)
        .ok_or(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "requirements snapshot pci devices out of bounds",
        ))?
        .iter()
        .map(expected_pci_from_snapshot)
        .collect();

    Ok(PlatformRequirements {
        arch: hv_config_model::ArchRequirement::X86_64,
        vmx: feature_from_snapshot(snapshot.vmx)?,
        ept: feature_from_snapshot(snapshot.ept)?,
        vtd: feature_from_snapshot(snapshot.vtd)?,
        min_physical_cores: snapshot.min_physical_cores,
        smt_policy: smt_policy_from_snapshot(snapshot.smt_policy)?,
        min_ram_bytes: ByteSize::new(snapshot.min_ram_bytes),
        interrupt_remapping: feature_from_snapshot(snapshot.interrupt_remapping)?,
        x2apic: feature_from_snapshot(snapshot.x2apic)?,
        invariant_tsc: feature_from_snapshot(snapshot.invariant_tsc)?,
        vpid: feature_from_snapshot(snapshot.vpid)?,
        vmx_preemption_timer: feature_from_snapshot(snapshot.vmx_preemption_timer)?,
        nx: feature_from_snapshot(snapshot.nx)?,
        page_sizes: PageSizeSet { sizes: page_sizes },
        expected_pci_devices,
    })
}

/// Builds a requirements snapshot from compiled platform requirements and layout metadata.
pub fn requirements_snapshot_from_platform(
    requirements: &PlatformRequirements,
    config_digest: [u8; SHA256_DIGEST_BYTES],
    hypervisor_reserve_phys: u64,
    hypervisor_reserve_bytes: u64,
) -> Result<RequirementsSnapshot, BootCheckError> {
    if requirements.page_sizes.sizes.len() > MAX_REQUIREMENTS_PAGE_SIZES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "page size count exceeds snapshot capacity",
        ));
    }
    if requirements.expected_pci_devices.len() > MAX_REQUIREMENTS_PCI_DEVICES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "expected pci device count exceeds snapshot capacity",
        ));
    }

    let mut page_sizes = [0u64; MAX_REQUIREMENTS_PAGE_SIZES];
    for (index, size) in requirements.page_sizes.sizes.iter().enumerate() {
        if let Some(slot) = page_sizes.get_mut(index) {
            *slot = *size;
        }
    }

    let mut expected_pci = [ExpectedPciSnapshot {
        vm_id: 0,
        segment: 0,
        bus: 0,
        device: 0,
        function: 0,
        reserved: [0; 3],
    }; MAX_REQUIREMENTS_PCI_DEVICES];
    for (index, device) in requirements.expected_pci_devices.iter().enumerate() {
        if let Some(slot) = expected_pci.get_mut(index) {
            *slot = expected_pci_to_snapshot(device);
        }
    }

    Ok(RequirementsSnapshot {
        arch: REQUIREMENTS_ARCH_X86_64,
        vmx: feature_to_snapshot(requirements.vmx),
        ept: feature_to_snapshot(requirements.ept),
        vtd: feature_to_snapshot(requirements.vtd),
        min_physical_cores: requirements.min_physical_cores,
        smt_policy: smt_policy_to_snapshot(requirements.smt_policy),
        min_ram_bytes: requirements.min_ram_bytes.bytes(),
        interrupt_remapping: feature_to_snapshot(requirements.interrupt_remapping),
        x2apic: feature_to_snapshot(requirements.x2apic),
        invariant_tsc: feature_to_snapshot(requirements.invariant_tsc),
        vpid: feature_to_snapshot(requirements.vpid),
        vmx_preemption_timer: feature_to_snapshot(requirements.vmx_preemption_timer),
        nx: feature_to_snapshot(requirements.nx),
        page_size_count: requirements.page_sizes.sizes.len() as u32,
        page_sizes,
        expected_pci_count: requirements.expected_pci_devices.len() as u32,
        expected_pci,
        hypervisor_reserve_phys,
        hypervisor_reserve_bytes,
        config_digest,
    })
}

fn expected_pci_from_snapshot(snapshot: &ExpectedPciSnapshot) -> ExpectedPciDevice {
    ExpectedPciDevice {
        vm_id: VmId::new(snapshot.vm_id),
        partition_id: String::new(),
        bdf: PciBdf::new(
            PciSegment::new(snapshot.segment),
            PciBus::new(snapshot.bus),
            PciDevice::new(snapshot.device),
            PciFunction::new(snapshot.function),
        ),
        kind: String::new(),
    }
}

fn expected_pci_to_snapshot(device: &ExpectedPciDevice) -> ExpectedPciSnapshot {
    ExpectedPciSnapshot {
        vm_id: device.vm_id.raw(),
        segment: device.bdf.segment.raw(),
        bus: device.bdf.bus.raw(),
        device: device.bdf.device.raw(),
        function: device.bdf.function.raw(),
        reserved: [0; 3],
    }
}

fn feature_from_snapshot(value: u32) -> Result<FeatureRequirement, BootCheckError> {
    match value {
        FEATURE_REQUIRED => Ok(FeatureRequirement::Required),
        FEATURE_PREFERRED => Ok(FeatureRequirement::Preferred),
        FEATURE_OPTIONAL => Ok(FeatureRequirement::Optional),
        FEATURE_DISABLED => Ok(FeatureRequirement::Disabled),
        _ => Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "invalid feature requirement discriminator",
        )),
    }
}

fn feature_to_snapshot(value: FeatureRequirement) -> u32 {
    match value {
        FeatureRequirement::Required => FEATURE_REQUIRED,
        FeatureRequirement::Preferred => FEATURE_PREFERRED,
        FeatureRequirement::Optional => FEATURE_OPTIONAL,
        FeatureRequirement::Disabled => FEATURE_DISABLED,
    }
}

fn smt_policy_from_snapshot(value: u32) -> Result<SmtPolicy, BootCheckError> {
    match value {
        SMT_POLICY_DISABLED => Ok(SmtPolicy::Disabled),
        SMT_POLICY_EXCLUSIVE_CORE => Ok(SmtPolicy::ExclusiveCore),
        SMT_POLICY_SAME_PARTITION_SIBLINGS => Ok(SmtPolicy::SamePartitionSiblings),
        SMT_POLICY_ALLOW_CROSS_PARTITION => Ok(SmtPolicy::AllowCrossPartition),
        _ => Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "invalid smt policy discriminator",
        )),
    }
}

fn smt_policy_to_snapshot(value: SmtPolicy) -> u32 {
    match value {
        SmtPolicy::Disabled => SMT_POLICY_DISABLED,
        SmtPolicy::ExclusiveCore => SMT_POLICY_EXCLUSIVE_CORE,
        SmtPolicy::SamePartitionSiblings => SMT_POLICY_SAME_PARTITION_SIBLINGS,
        SmtPolicy::AllowCrossPartition => SMT_POLICY_ALLOW_CROSS_PARTITION,
    }
}

/// Builds a layout snapshot from a planned static platform IR.
pub fn layout_snapshot_from_platform_ir(
    layout: &StaticPlatformIR,
) -> Result<LayoutSnapshot, BootCheckError> {
    if layout.guest_memory.len() > MAX_LAYOUT_GUEST_REGIONS {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "guest region count exceeds layout snapshot capacity",
        ));
    }
    if layout.ipc_memory.len() > MAX_LAYOUT_IPC_REGIONS {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "ipc region count exceeds layout snapshot capacity",
        ));
    }
    if layout.pci_devices.len() > MAX_LAYOUT_PCI_DEVICES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "pci device count exceeds layout snapshot capacity",
        ));
    }
    if layout.host_network.interfaces.len() > MAX_LAYOUT_HOST_NETWORK_INTERFACES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "host network interface count exceeds layout snapshot capacity",
        ));
    }

    let mut guest_regions = [LayoutGuestRegionSnapshot {
        vm_id: 0,
        host_phys: 0,
        size_bytes: 0,
        partition_id_len: 0,
        partition_id: [0; MAX_LAYOUT_PARTITION_ID_LEN],
    }; MAX_LAYOUT_GUEST_REGIONS];
    for (index, region) in layout.guest_memory.iter().enumerate() {
        if let Some(slot) = guest_regions.get_mut(index) {
            let (partition_id_len, partition_id) =
                encode_fixed_string::<MAX_LAYOUT_PARTITION_ID_LEN>(&region.partition_id)?;
            *slot = LayoutGuestRegionSnapshot {
                vm_id: region.vm_id.raw(),
                host_phys: region.host_phys.raw(),
                size_bytes: region.size.bytes(),
                partition_id_len,
                partition_id,
            };
        }
    }

    let mut ipc_regions = [LayoutIpcRegionSnapshot {
        channel_id: 0,
        producer_vm_id: 0,
        consumer_vm_id: 0,
        host_phys: 0,
        size_bytes: 0,
    }; MAX_LAYOUT_IPC_REGIONS];
    for (index, region) in layout.ipc_memory.iter().enumerate() {
        if let Some(slot) = ipc_regions.get_mut(index) {
            *slot = LayoutIpcRegionSnapshot {
                channel_id: region.channel_id.raw(),
                producer_vm_id: region.producer_vm_id.raw(),
                consumer_vm_id: region.consumer_vm_id.raw(),
                host_phys: region.host_phys.raw(),
                size_bytes: region.size.bytes(),
            };
        }
    }

    let mut pci_devices = [LayoutPciSnapshot {
        vm_id: 0,
        segment: 0,
        bus: 0,
        device: 0,
        function: 0,
        device_role: LAYOUT_DEVICE_ROLE_NONE,
        reserved: 0,
        device_kind: 0,
    }; MAX_LAYOUT_PCI_DEVICES];
    for (index, device) in layout.pci_devices.iter().enumerate() {
        if let Some(slot) = pci_devices.get_mut(index) {
            *slot = layout_pci_to_snapshot(device);
        }
    }

    let mut host_network_interfaces = [LayoutHostNetworkSnapshot {
        vm_id: 0,
        segment: 0,
        bus: 0,
        device: 0,
        function: 0,
        backend: LAYOUT_HOST_NETWORK_BACKEND_USER,
        tap_ifname_len: 0,
        tap_ifname: [0; LAYOUT_HOST_NETWORK_IFNAME_LEN],
        netdev_id_len: 0,
        reserved: 0,
        netdev_id: [0; LAYOUT_HOST_NETWORK_NETDEV_LEN],
    }; MAX_LAYOUT_HOST_NETWORK_INTERFACES];
    for (index, interface) in layout.host_network.interfaces.iter().enumerate() {
        if let Some(slot) = host_network_interfaces.get_mut(index) {
            *slot = layout_host_network_to_snapshot(interface, &layout.host_network.backend)?;
        }
    }

    Ok(LayoutSnapshot {
        guest_region_count: layout.guest_memory.len() as u32,
        guest_regions,
        ipc_region_count: layout.ipc_memory.len() as u32,
        ipc_regions,
        pci_device_count: layout.pci_devices.len() as u32,
        pci_devices,
        hypervisor_reserve_phys: layout.hypervisor_reserve.host_phys.raw(),
        hypervisor_reserve_bytes: layout.hypervisor_reserve.size.bytes(),
        host_network_enabled: u8::from(layout.host_network.enabled),
        host_network_backend: host_network_backend_to_snapshot(&layout.host_network.backend),
        host_network_reserved: [0; 2],
        host_network_interface_count: layout.host_network.interfaces.len() as u32,
        host_network_interfaces,
    })
}

/// Reconstructs static platform IR from embedded layout and requirements snapshots.
pub fn static_platform_ir_from_layout_snapshot(
    layout: &LayoutSnapshot,
    requirements: &RequirementsSnapshot,
) -> Result<StaticPlatformIR, BootCheckError> {
    validate_layout_counts(layout)?;
    validate_layout_reserve_matches_requirements(layout, requirements)?;

    let mut guest_memory = alloc::vec::Vec::with_capacity(layout.guest_region_count as usize);
    for region in layout
        .guest_regions
        .get(0..layout.guest_region_count as usize)
        .ok_or(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot guest regions out of bounds",
        ))?
    {
        guest_memory.push(PlannedGuestMemory {
            partition_id: decode_fixed_string::<MAX_LAYOUT_PARTITION_ID_LEN>(
                region.partition_id_len,
                &region.partition_id,
            )?,
            vm_id: VmId::new(region.vm_id),
            host_phys: HostPhysAddr::new(region.host_phys),
            size: ByteSize::new(region.size_bytes),
        });
    }

    let mut ipc_memory = alloc::vec::Vec::with_capacity(layout.ipc_region_count as usize);
    for region in layout
        .ipc_regions
        .get(0..layout.ipc_region_count as usize)
        .ok_or(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot ipc regions out of bounds",
        ))?
    {
        ipc_memory.push(PlannedIpcMemory {
            channel_name: String::new(),
            channel_id: hv_types::IpcChannelId::new(region.channel_id),
            producer_vm_id: VmId::new(region.producer_vm_id),
            consumer_vm_id: VmId::new(region.consumer_vm_id),
            host_phys: HostPhysAddr::new(region.host_phys),
            size: ByteSize::new(region.size_bytes),
        });
    }

    let mut pci_devices = alloc::vec::Vec::with_capacity(layout.pci_device_count as usize);
    for device in layout
        .pci_devices
        .get(0..layout.pci_device_count as usize)
        .ok_or(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot pci devices out of bounds",
        ))?
    {
        pci_devices.push(planned_pci_from_layout_snapshot(device));
    }

    let mut host_network_interfaces =
        alloc::vec::Vec::with_capacity(layout.host_network_interface_count as usize);
    for interface in layout
        .host_network_interfaces
        .get(0..layout.host_network_interface_count as usize)
        .ok_or(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot host network interfaces out of bounds",
        ))?
    {
        host_network_interfaces.push(host_network_from_layout_snapshot(interface)?);
    }
    for interface in &mut host_network_interfaces {
        if let Some(guest) = guest_memory
            .iter()
            .find(|guest| guest.vm_id == interface.vm_id)
        {
            interface.partition_id = guest.partition_id.clone();
        }
    }

    Ok(StaticPlatformIR {
        platform_name: String::new(),
        guest_memory,
        ipc_memory,
        hypervisor_reserve: PlannedHypervisorReserve {
            host_phys: HostPhysAddr::new(layout.hypervisor_reserve_phys),
            size: ByteSize::new(layout.hypervisor_reserve_bytes),
        },
        pci_devices,
        host_network: HostNetworkPlan {
            enabled: layout.host_network_enabled != 0,
            backend: host_network_backend_from_snapshot(layout.host_network_backend),
            interfaces: host_network_interfaces,
        },
    })
}

fn validate_layout_counts(layout: &LayoutSnapshot) -> Result<(), BootCheckError> {
    if layout.guest_region_count as usize > MAX_LAYOUT_GUEST_REGIONS {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot guest region count exceeds maximum",
        ));
    }
    if layout.ipc_region_count as usize > MAX_LAYOUT_IPC_REGIONS {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot ipc region count exceeds maximum",
        ));
    }
    if layout.pci_device_count as usize > MAX_LAYOUT_PCI_DEVICES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot pci device count exceeds maximum",
        ));
    }
    if layout.host_network_interface_count as usize > MAX_LAYOUT_HOST_NETWORK_INTERFACES {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot host network interface count exceeds maximum",
        ));
    }
    Ok(())
}

fn validate_layout_reserve_matches_requirements(
    layout: &LayoutSnapshot,
    requirements: &RequirementsSnapshot,
) -> Result<(), BootCheckError> {
    if layout.hypervisor_reserve_phys != requirements.hypervisor_reserve_phys {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot reserve phys mismatch with requirements snapshot",
        ));
    }
    if layout.hypervisor_reserve_bytes != requirements.hypervisor_reserve_bytes {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot reserve bytes mismatch with requirements snapshot",
        ));
    }
    Ok(())
}

fn layout_pci_to_snapshot(device: &PlannedPciDevice) -> LayoutPciSnapshot {
    LayoutPciSnapshot {
        vm_id: device.vm_id.raw(),
        segment: device.bdf.segment.raw(),
        bus: device.bdf.bus.raw(),
        device: device.bdf.device.raw(),
        function: device.bdf.function.raw(),
        device_role: device_role_to_snapshot(device.role.as_deref()),
        reserved: 0,
        device_kind: device_kind_to_snapshot(&device.kind),
    }
}

fn planned_pci_from_layout_snapshot(device: &LayoutPciSnapshot) -> PlannedPciDevice {
    PlannedPciDevice {
        bdf: PciBdf::new(
            PciSegment::new(device.segment),
            PciBus::new(device.bus),
            PciDevice::new(device.device),
            PciFunction::new(device.function),
        ),
        vm_id: VmId::new(device.vm_id),
        kind: device_kind_from_snapshot(device.device_kind),
        role: device_role_from_snapshot(device.device_role),
    }
}

fn layout_host_network_to_snapshot(
    interface: &HostNetworkInterface,
    backend: &str,
) -> Result<LayoutHostNetworkSnapshot, BootCheckError> {
    let (tap_ifname_len, tap_ifname) = encode_fixed_string::<LAYOUT_HOST_NETWORK_IFNAME_LEN>(
        interface.tap_ifname.as_deref().unwrap_or_default(),
    )?;
    let (netdev_id_len, netdev_id) =
        encode_fixed_string::<LAYOUT_HOST_NETWORK_NETDEV_LEN>(&interface.netdev_id)?;
    Ok(LayoutHostNetworkSnapshot {
        vm_id: interface.vm_id.raw(),
        segment: interface.bdf.segment.raw(),
        bus: interface.bdf.bus.raw(),
        device: interface.bdf.device.raw(),
        function: interface.bdf.function.raw(),
        backend: host_network_backend_to_snapshot(backend),
        tap_ifname_len,
        tap_ifname,
        netdev_id_len,
        reserved: 0,
        netdev_id,
    })
}

fn host_network_from_layout_snapshot(
    interface: &LayoutHostNetworkSnapshot,
) -> Result<HostNetworkInterface, BootCheckError> {
    let tap_ifname = decode_fixed_string::<LAYOUT_HOST_NETWORK_IFNAME_LEN>(
        interface.tap_ifname_len,
        &interface.tap_ifname,
    )?;
    let netdev_id = decode_fixed_string::<LAYOUT_HOST_NETWORK_NETDEV_LEN>(
        interface.netdev_id_len,
        &interface.netdev_id,
    )?;
    Ok(HostNetworkInterface {
        partition_id: String::new(),
        vm_id: VmId::new(interface.vm_id),
        bdf: PciBdf::new(
            PciSegment::new(interface.segment),
            PciBus::new(interface.bus),
            PciDevice::new(interface.device),
            PciFunction::new(interface.function),
        ),
        pci_addr: String::new(),
        netdev_id,
        tap_ifname: if tap_ifname.is_empty() {
            None
        } else {
            Some(tap_ifname)
        },
    })
}

fn encode_fixed_string<const N: usize>(value: &str) -> Result<(u8, [u8; N]), BootCheckError> {
    if value.len() > N {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot string exceeds fixed capacity",
        ));
    }
    let mut out = [0u8; N];
    if let Some(slice) = out.get_mut(0..value.len()) {
        slice.copy_from_slice(value.as_bytes());
    }
    Ok((value.len() as u8, out))
}

fn decode_fixed_string<const N: usize>(len: u8, bytes: &[u8; N]) -> Result<String, BootCheckError> {
    if usize::from(len) > N {
        return Err(BootCheckError::new(
            BootCheckErrorKind::Platform,
            "layout snapshot string length out of bounds",
        ));
    }
    let slice = bytes.get(0..usize::from(len)).ok_or(BootCheckError::new(
        BootCheckErrorKind::Platform,
        "layout snapshot string unreadable",
    ))?;
    core::str::from_utf8(slice)
        .map(String::from)
        .map_err(|_| {
            BootCheckError::new(
                BootCheckErrorKind::Platform,
                "layout snapshot string is not valid UTF-8",
            )
        })
}

fn device_role_to_snapshot(role: Option<&str>) -> u8 {
    match role {
        Some(hv_platform_model::DATAPATH_ROLE_IN) => LAYOUT_DEVICE_ROLE_DATAPATH_IN,
        Some(hv_platform_model::DATAPATH_ROLE_OUT) => LAYOUT_DEVICE_ROLE_DATAPATH_OUT,
        _ => LAYOUT_DEVICE_ROLE_NONE,
    }
}

fn device_role_from_snapshot(role: u8) -> Option<String> {
    match role {
        LAYOUT_DEVICE_ROLE_DATAPATH_IN => Some(String::from(hv_platform_model::DATAPATH_ROLE_IN)),
        LAYOUT_DEVICE_ROLE_DATAPATH_OUT => Some(String::from(hv_platform_model::DATAPATH_ROLE_OUT)),
        _ => None,
    }
}

fn host_network_backend_to_snapshot(backend: &str) -> u8 {
    if backend == "tap" {
        LAYOUT_HOST_NETWORK_BACKEND_TAP
    } else {
        LAYOUT_HOST_NETWORK_BACKEND_USER
    }
}

fn host_network_backend_from_snapshot(backend: u8) -> String {
    if backend == LAYOUT_HOST_NETWORK_BACKEND_TAP {
        String::from("tap")
    } else {
        String::from("user")
    }
}

fn device_kind_to_snapshot(kind: &str) -> u32 {
    if kind == "nic_e1000" {
        LAYOUT_DEVICE_KIND_NIC_E1000
    } else {
        0
    }
}

fn device_kind_from_snapshot(kind: u32) -> String {
    if kind == LAYOUT_DEVICE_KIND_NIC_E1000 {
        String::from("nic_e1000")
    } else {
        String::new()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    fn reference_reserve() -> (u64, u64) {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        (
            layout.hypervisor_reserve.host_phys.raw(),
            layout.hypervisor_reserve.size.bytes(),
        )
    }

    #[test]
    fn requirements_snapshot_roundtrip_from_reference_config() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let (reserve_phys, reserve_bytes) = reference_reserve();
        let snapshot = requirements_snapshot_from_platform(
            &compiled.requirements,
            compiled.digest.bytes,
            reserve_phys,
            reserve_bytes,
        )
        .expect("snapshot");
        let restored = platform_requirements_from_snapshot(&snapshot).expect("restore");
        assert_eq!(restored.arch, compiled.requirements.arch);
        assert_eq!(restored.vmx, compiled.requirements.vmx);
        assert_eq!(
            restored.min_physical_cores,
            compiled.requirements.min_physical_cores
        );
        assert_eq!(restored.page_sizes, compiled.requirements.page_sizes);
        assert_eq!(
            restored
                .expected_pci_devices
                .iter()
                .map(|device| (device.vm_id, device.bdf))
                .collect::<Vec<_>>(),
            compiled
                .requirements
                .expected_pci_devices
                .iter()
                .map(|device| (device.vm_id, device.bdf))
                .collect::<Vec<_>>()
        );
        assert_eq!(snapshot.config_digest, compiled.digest.bytes);
    }

    #[test]
    fn platform_requirements_from_snapshot_rejects_invalid_metadata() {
        let mut snapshot = RequirementsSnapshot {
            arch: REQUIREMENTS_ARCH_X86_64,
            vmx: FEATURE_REQUIRED,
            ept: FEATURE_REQUIRED,
            vtd: FEATURE_REQUIRED,
            min_physical_cores: 1,
            smt_policy: SMT_POLICY_DISABLED,
            min_ram_bytes: 1,
            interrupt_remapping: FEATURE_REQUIRED,
            x2apic: FEATURE_REQUIRED,
            invariant_tsc: FEATURE_REQUIRED,
            vpid: FEATURE_REQUIRED,
            vmx_preemption_timer: FEATURE_REQUIRED,
            nx: FEATURE_REQUIRED,
            page_size_count: 1,
            page_sizes: [4096, 0, 0, 0],
            expected_pci_count: 0,
            expected_pci: [ExpectedPciSnapshot {
                vm_id: 0,
                segment: 0,
                bus: 0,
                device: 0,
                function: 0,
                reserved: [0; 3],
            }; MAX_REQUIREMENTS_PCI_DEVICES],
            hypervisor_reserve_phys: 0,
            hypervisor_reserve_bytes: 4096,
            config_digest: [0; SHA256_DIGEST_BYTES],
        };
        snapshot.arch = 99;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());

        snapshot.arch = REQUIREMENTS_ARCH_X86_64;
        snapshot.page_size_count = MAX_REQUIREMENTS_PAGE_SIZES as u32 + 1;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());

        snapshot.page_size_count = 1;
        snapshot.expected_pci_count = MAX_REQUIREMENTS_PCI_DEVICES as u32 + 1;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());

        snapshot.expected_pci_count = 0;
        snapshot.vmx = 99;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());

        snapshot.vmx = FEATURE_REQUIRED;
        snapshot.smt_policy = 99;
        assert!(platform_requirements_from_snapshot(&snapshot).is_err());
    }

    #[test]
    fn requirements_snapshot_from_platform_rejects_oversized_inputs() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let (reserve_phys, reserve_bytes) = reference_reserve();
        let mut requirements = compiled.requirements.clone();
        requirements.page_sizes.sizes = vec![4096; MAX_REQUIREMENTS_PAGE_SIZES + 1];
        assert!(requirements_snapshot_from_platform(
            &requirements,
            compiled.digest.bytes,
            reserve_phys,
            reserve_bytes,
        )
        .is_err());

        requirements = compiled.requirements.clone();
        let device = compiled
            .requirements
            .expected_pci_devices
            .first()
            .expect("device")
            .clone();
        requirements.expected_pci_devices = (0..=MAX_REQUIREMENTS_PCI_DEVICES)
            .map(|_| device.clone())
            .collect();
        assert!(requirements_snapshot_from_platform(
            &requirements,
            compiled.digest.bytes,
            reserve_phys,
            reserve_bytes,
        )
        .is_err());
    }

    #[test]
    fn layout_snapshot_roundtrip_preserves_gate_c_planning_fields() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let requirements = requirements_snapshot_from_platform(
            &compiled.requirements,
            compiled.digest.bytes,
            layout.hypervisor_reserve.host_phys.raw(),
            layout.hypervisor_reserve.size.bytes(),
        )
        .expect("requirements snapshot");
        let layout_snapshot = layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
        let restored = static_platform_ir_from_layout_snapshot(&layout_snapshot, &requirements)
            .expect("restore");
        assert_eq!(restored.guest_memory.len(), layout.guest_memory.len());
        assert_eq!(restored.ipc_memory.len(), layout.ipc_memory.len());
        assert_eq!(restored.pci_devices.len(), layout.pci_devices.len());
        assert_eq!(
            restored
                .guest_memory
                .iter()
                .map(|region| region.vm_id)
                .collect::<Vec<_>>(),
            layout
                .guest_memory
                .iter()
                .map(|region| region.vm_id)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            restored
                .ipc_memory
                .iter()
                .map(|region| (
                    region.channel_id,
                    region.producer_vm_id,
                    region.consumer_vm_id
                ))
                .collect::<Vec<_>>(),
            layout
                .ipc_memory
                .iter()
                .map(|region| (
                    region.channel_id,
                    region.producer_vm_id,
                    region.consumer_vm_id
                ))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            restored
                .pci_devices
                .iter()
                .map(|device| (device.vm_id, device.kind.as_str()))
                .collect::<Vec<_>>(),
            layout
                .pci_devices
                .iter()
                .map(|device| (device.vm_id, device.kind.as_str()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn static_platform_ir_from_layout_snapshot_rejects_reserve_mismatch() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let requirements = requirements_snapshot_from_platform(
            &compiled.requirements,
            compiled.digest.bytes,
            layout.hypervisor_reserve.host_phys.raw(),
            layout.hypervisor_reserve.size.bytes(),
        )
        .expect("requirements snapshot");
        let mut layout_snapshot =
            layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
        layout_snapshot.hypervisor_reserve_bytes ^= 1;
        assert!(static_platform_ir_from_layout_snapshot(&layout_snapshot, &requirements).is_err());
    }
}
