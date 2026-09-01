//! Artifact generation for human review and build integration.

use std::fs;
use std::path::{Path, PathBuf};

use crate::artifacts::{
    BOOT_LAYOUT, BOOT_MANIFEST, BUILD_MANIFEST, CONFIG_SHA256, CORE_OWNERSHIP, CPU_TOPOLOGY,
    GUEST_IMAGES, HYPERVISOR_EMBEDDED_CONFIG_RS, IPC_MAP, MEMORY_MAP, PCI_MAP, PLATFORM_LAYOUT,
    PLATFORM_REQUIREMENTS, QEMU_ARGS, STATIC_INTENT_JSON, STATIC_PLATFORM_LAYOUT_JSON,
    STATIC_PLATFORM_RS,
};
use hv_config_model::compile_config_from_path;
use hv_hypervisor_boot::{layout_snapshot_from_platform_ir, requirements_snapshot_from_platform};
use hv_platform_model::plan_static_platform_ir;

/// Generates configuration artifacts into `output`.
pub fn generate(path: &Path, output: &Path) -> i32 {
    match compile_config_from_path(path) {
        Ok(compiled) => match write_artifacts(path, output, &compiled) {
            Ok(()) => {
                eprintln!("generated artifacts in {}", output.display());
                0
            }
            Err(err) => {
                eprintln!("generation failed: {err}");
                1
            }
        },
        Err(err) => {
            eprintln!("error: {err}");
            1
        }
    }
}

fn write_artifacts(
    source: &Path,
    output: &Path,
    compiled: &hv_config_model::CompiledConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;

    write_file(
        output.join(PLATFORM_REQUIREMENTS),
        render_requirements(compiled),
    )?;
    write_file(output.join(IPC_MAP), render_ipc_map(compiled))?;
    write_file(output.join(PCI_MAP), render_pci_map(compiled))?;
    write_file(output.join(CPU_TOPOLOGY), render_cpu_topology(compiled))?;
    write_file(output.join(MEMORY_MAP), render_memory_map(compiled))?;
    write_file(output.join(CORE_OWNERSHIP), render_core_ownership(compiled))?;
    write_file(output.join(BOOT_LAYOUT), render_boot_layout(compiled))?;
    write_file(output.join(GUEST_IMAGES), render_guest_images(compiled))?;
    write_file(output.join(QEMU_ARGS), render_qemu_args(compiled))?;
    write_file(output.join(BOOT_MANIFEST), render_boot_manifest(compiled))?;
    write_file(
        output.join(BUILD_MANIFEST),
        render_build_manifest(source, compiled),
    )?;
    write_file(output.join(CONFIG_SHA256), compiled.digest.to_hex())?;
    write_file(
        output.join(STATIC_PLATFORM_RS),
        render_static_platform_rs(compiled)?,
    )?;

    let intent_json = serde_json::to_string_pretty(&compiled.intent)?;
    write_file(output.join(STATIC_INTENT_JSON), intent_json)?;

    let platform_ir = plan_static_platform_ir(&compiled.intent).map_err(|err| err.to_string())?;
    let platform_json = serde_json::to_string_pretty(&platform_ir)?;
    write_file(output.join(STATIC_PLATFORM_LAYOUT_JSON), platform_json)?;
    write_file(
        output.join(PLATFORM_LAYOUT),
        render_platform_layout(&platform_ir),
    )?;
    write_file(
        output.join(HYPERVISOR_EMBEDDED_CONFIG_RS),
        render_hypervisor_embedded_config(&compiled.digest.bytes, &platform_ir, compiled)?,
    )?;

    Ok(())
}

fn write_file(path: PathBuf, contents: String) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn render_requirements(compiled: &hv_config_model::CompiledConfig) -> String {
    let req = &compiled.requirements;
    format!(
        "arch={:?}\nvmx={:?}\nept={:?}\nvtd={:?}\nmin_physical_cores={}\nmin_ram_bytes={}\nsmt_policy={:?}\ninterrupt_remapping={:?}\nnx={:?}\npage_sizes={:?}\n",
        req.arch,
        req.vmx,
        req.ept,
        req.vtd,
        req.min_physical_cores,
        req.min_ram_bytes.bytes(),
        req.smt_policy,
        req.interrupt_remapping,
        req.nx,
        req.page_sizes.sizes,
    )
}

fn render_ipc_map(compiled: &hv_config_model::CompiledConfig) -> String {
    let mut out = String::new();
    for channel in &compiled.intent.ipc {
        out.push_str(&format!(
            "channel={} id={} producer={}({}) consumer={}({}) slots={} slot_bytes={} shared_bytes={}\n",
            channel.id,
            channel.channel_id.raw(),
            channel.producer,
            channel.producer_vm_id.raw(),
            channel.consumer,
            channel.consumer_vm_id.raw(),
            channel.queue_slots,
            channel.slot_size_bytes,
            channel.shared_bytes.bytes(),
        ));
    }
    out
}

fn render_pci_map(compiled: &hv_config_model::CompiledConfig) -> String {
    let mut out = String::new();
    for (bdf, vm_id, kind, role, mmio_guest_phys) in &compiled.intent.pci_intent.devices {
        out.push_str(&format!(
            "bdf={:04x}:{:02x}:{:02x}.{} vm_id={} kind={} role={} mmio_guest_phys={:#x}\n",
            bdf.segment.raw(),
            bdf.bus.raw(),
            bdf.device.raw(),
            bdf.function.raw(),
            vm_id.raw(),
            kind,
            role.as_deref().unwrap_or("-"),
            mmio_guest_phys,
        ));
    }
    out
}

fn render_cpu_topology(compiled: &hv_config_model::CompiledConfig) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "min_physical_cores={}\n",
        compiled.intent.cpu_intent.min_physical_cores
    ));
    for (vm_id, cores) in &compiled.intent.cpu_intent.core_assignments {
        out.push_str(&format!("vm_id={} cores={cores:?}\n", vm_id.raw()));
    }
    out
}

fn render_memory_map(compiled: &hv_config_model::CompiledConfig) -> String {
    format!(
        "total_guest_bytes={}\ntotal_ipc_bytes={}\nhypervisor_reserve_bytes={}\n",
        compiled.intent.memory_intent.total_guest_bytes.bytes(),
        compiled.intent.memory_intent.total_ipc_bytes.bytes(),
        compiled
            .intent
            .memory_intent
            .hypervisor_reserve_bytes
            .bytes(),
    )
}

fn render_core_ownership(compiled: &hv_config_model::CompiledConfig) -> String {
    render_cpu_topology(compiled)
}

fn render_boot_layout(compiled: &hv_config_model::CompiledConfig) -> String {
    render_guest_images(compiled)
}

fn render_guest_images(compiled: &hv_config_model::CompiledConfig) -> String {
    let mut out = String::new();
    for image in &compiled.intent.boot_intent.guest_images {
        out.push_str(&format!(
            "partition={} vm_id={} path={} sha256={}\n",
            image.partition,
            image.vm_id.raw(),
            image.path,
            image.sha256,
        ));
    }
    out
}

fn render_qemu_args(compiled: &hv_config_model::CompiledConfig) -> String {
    let plan = &compiled.intent.qemu_plan;
    format!(
        "-machine {}\n-accel {}\n-cpu {}\n-smp sockets={},cores={},threads={}\n-m {}\n",
        plan.machine,
        plan.accel,
        plan.cpu_model,
        plan.smp_sockets,
        plan.smp_cores,
        plan.smp_threads,
        plan.memory_bytes.bytes(),
    )
}

fn render_boot_manifest(compiled: &hv_config_model::CompiledConfig) -> String {
    format!(
        "platform={}\npartitions={}\nipc_channels={}\nconfig_sha256={}\n",
        compiled.intent.platform_name,
        compiled.intent.partitions.len(),
        compiled.intent.ipc.len(),
        compiled.digest.to_hex(),
    )
}

fn render_build_manifest(source: &Path, compiled: &hv_config_model::CompiledConfig) -> String {
    format!(
        "source={}\nrust_version={}\nconfig_sha256={}\nschema_version={}\n",
        source.display(),
        env!("CARGO_PKG_RUST_VERSION"),
        compiled.digest.to_hex(),
        compiled.normalized.schema_version,
    )
}

fn render_static_platform_rs(
    compiled: &hv_config_model::CompiledConfig,
) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string(&compiled.intent)?;
    let digest = compiled.digest.to_hex();
    Ok(format!(
        "// @generated by hv-config\npub const CONFIG_SHA256: &str = \"{digest}\";\npub static PLATFORM_IR_JSON: &str = r#\"{json}\"#;\n",
    ))
}

fn render_platform_layout(platform_ir: &hv_platform_model::StaticPlatformIR) -> String {
    let mut out = String::new();
    out.push_str(&format!("platform={}\n", platform_ir.platform_name));
    for region in &platform_ir.guest_memory {
        out.push_str(&format!(
            "guest partition={} vm_id={} host_phys={:#x} size={}\n",
            region.partition_id,
            region.vm_id.raw(),
            region.host_phys.raw(),
            region.size.bytes(),
        ));
    }
    for region in &platform_ir.ipc_memory {
        out.push_str(&format!(
            "ipc channel={} id={} host_phys={:#x} size={}\n",
            region.channel_name,
            region.channel_id.raw(),
            region.host_phys.raw(),
            region.size.bytes(),
        ));
    }
    out.push_str(&format!(
        "hypervisor_reserve host_phys={:#x} size={}\n",
        platform_ir.hypervisor_reserve.host_phys.raw(),
        platform_ir.hypervisor_reserve.size.bytes(),
    ));
    for device in &platform_ir.pci_devices {
        out.push_str(&format!(
            "pci bdf={:04x}:{:02x}:{:02x}.{} vm_id={} kind={}\n",
            device.bdf.segment.raw(),
            device.bdf.bus.raw(),
            device.bdf.device.raw(),
            device.bdf.function.raw(),
            device.vm_id.raw(),
            device.kind,
        ));
    }
    out
}

fn render_hypervisor_embedded_config(
    digest: &[u8; 32],
    platform_ir: &hv_platform_model::StaticPlatformIR,
    compiled: &hv_config_model::CompiledConfig,
) -> Result<String, String> {
    let snapshot = requirements_snapshot_from_platform(
        &compiled.requirements,
        *digest,
        platform_ir.hypervisor_reserve.host_phys.raw(),
        platform_ir.hypervisor_reserve.size.bytes(),
    )
    .map_err(|err| err.message)?;
    let layout_snapshot =
        layout_snapshot_from_platform_ir(platform_ir).map_err(|err| err.message)?;
    Ok(render_embedded_config_rs(
        digest,
        &snapshot,
        &layout_snapshot,
    ))
}

fn render_byte_array(bytes: &[u8]) -> String {
    let mut rendered = String::from("[");
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&format!("0x{byte:02X}"));
    }
    rendered.push(']');
    rendered
}

fn render_embedded_config_rs(
    digest: &[u8; 32],
    snapshot: &hv_boot_abi::RequirementsSnapshot,
    layout_snapshot: &hv_boot_abi::LayoutSnapshot,
) -> String {
    let mut rendered = String::from("pub const CONFIG_DIGEST: [u8; 32] = [");
    for (index, byte) in digest.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&format!("0x{byte:02X}"));
    }
    rendered.push_str("];\n\npub const REQUIREMENTS_SNAPSHOT: hv_boot_abi::RequirementsSnapshot = hv_boot_abi::RequirementsSnapshot {\n");
    rendered.push_str(&format!("    arch: {},\n", snapshot.arch));
    rendered.push_str(&format!("    vmx: {},\n", snapshot.vmx));
    rendered.push_str(&format!("    ept: {},\n", snapshot.ept));
    rendered.push_str(&format!("    vtd: {},\n", snapshot.vtd));
    rendered.push_str(&format!(
        "    min_physical_cores: {},\n",
        snapshot.min_physical_cores
    ));
    rendered.push_str(&format!("    smt_policy: {},\n", snapshot.smt_policy));
    rendered.push_str(&format!("    min_ram_bytes: {},\n", snapshot.min_ram_bytes));
    rendered.push_str(&format!(
        "    interrupt_remapping: {},\n",
        snapshot.interrupt_remapping
    ));
    rendered.push_str(&format!("    x2apic: {},\n", snapshot.x2apic));
    rendered.push_str(&format!("    invariant_tsc: {},\n", snapshot.invariant_tsc));
    rendered.push_str(&format!("    vpid: {},\n", snapshot.vpid));
    rendered.push_str(&format!(
        "    vmx_preemption_timer: {},\n",
        snapshot.vmx_preemption_timer
    ));
    rendered.push_str(&format!("    nx: {},\n", snapshot.nx));
    rendered.push_str(&format!(
        "    page_size_count: {},\n",
        snapshot.page_size_count
    ));
    rendered.push_str("    page_sizes: [");
    for (index, size) in snapshot.page_sizes.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&size.to_string());
    }
    rendered.push_str("],\n");
    rendered.push_str(&format!(
        "    expected_pci_count: {},\n",
        snapshot.expected_pci_count
    ));
    rendered.push_str("    expected_pci: [\n");
    for entry in &snapshot.expected_pci {
        rendered.push_str("        hv_boot_abi::ExpectedPciSnapshot {\n");
        rendered.push_str(&format!("            vm_id: {},\n", entry.vm_id));
        rendered.push_str(&format!("            segment: {},\n", entry.segment));
        rendered.push_str(&format!("            bus: {},\n", entry.bus));
        rendered.push_str(&format!("            device: {},\n", entry.device));
        rendered.push_str(&format!("            function: {},\n", entry.function));
        rendered.push_str(&format!(
            "            reserved: [{}, {}, {}],\n",
            entry.reserved[0], entry.reserved[1], entry.reserved[2]
        ));
        rendered.push_str("        },\n");
    }
    rendered.push_str("    ],\n");
    rendered.push_str(&format!(
        "    hypervisor_reserve_phys: {},\n",
        snapshot.hypervisor_reserve_phys
    ));
    rendered.push_str(&format!(
        "    hypervisor_reserve_bytes: {},\n",
        snapshot.hypervisor_reserve_bytes
    ));
    rendered.push_str("    config_digest: [");
    for (index, byte) in snapshot.config_digest.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&format!("0x{byte:02X}"));
    }
    rendered.push_str("],\n};\n");
    rendered.push_str("\npub const LAYOUT_SNAPSHOT: hv_boot_abi::LayoutSnapshot = hv_boot_abi::LayoutSnapshot {\n");
    rendered.push_str(&format!(
        "    guest_region_count: {},\n",
        layout_snapshot.guest_region_count
    ));
    rendered.push_str("    guest_regions: [\n");
    for region in layout_snapshot.guest_regions.iter() {
        rendered.push_str("        hv_boot_abi::LayoutGuestRegionSnapshot {\n");
        rendered.push_str(&format!("            vm_id: {},\n", region.vm_id));
        rendered.push_str(&format!("            host_phys: {},\n", region.host_phys));
        rendered.push_str(&format!("            size_bytes: {},\n", region.size_bytes));
        rendered.push_str(&format!(
            "            partition_id_len: {},\n",
            region.partition_id_len
        ));
        rendered.push_str(&format!(
            "            partition_id: {},\n",
            render_byte_array(&region.partition_id)
        ));
        rendered.push_str("        },\n");
    }
    rendered.push_str("    ],\n");
    rendered.push_str(&format!(
        "    ipc_region_count: {},\n",
        layout_snapshot.ipc_region_count
    ));
    rendered.push_str("    ipc_regions: [\n");
    for region in layout_snapshot.ipc_regions.iter() {
        rendered.push_str("        hv_boot_abi::LayoutIpcRegionSnapshot {\n");
        rendered.push_str(&format!("            channel_id: {},\n", region.channel_id));
        rendered.push_str(&format!(
            "            producer_vm_id: {},\n",
            region.producer_vm_id
        ));
        rendered.push_str(&format!(
            "            consumer_vm_id: {},\n",
            region.consumer_vm_id
        ));
        rendered.push_str(&format!("            host_phys: {},\n", region.host_phys));
        rendered.push_str(&format!("            size_bytes: {},\n", region.size_bytes));
        rendered.push_str("        },\n");
    }
    rendered.push_str("    ],\n");
    rendered.push_str(&format!(
        "    pci_device_count: {},\n",
        layout_snapshot.pci_device_count
    ));
    rendered.push_str("    pci_devices: [\n");
    for device in layout_snapshot.pci_devices.iter() {
        rendered.push_str("        hv_boot_abi::LayoutPciSnapshot {\n");
        rendered.push_str(&format!("            vm_id: {},\n", device.vm_id));
        rendered.push_str(&format!("            segment: {},\n", device.segment));
        rendered.push_str(&format!("            bus: {},\n", device.bus));
        rendered.push_str(&format!("            device: {},\n", device.device));
        rendered.push_str(&format!("            function: {},\n", device.function));
        rendered.push_str(&format!(
            "            device_role: {},\n",
            device.device_role
        ));
        rendered.push_str(&format!("            reserved: {},\n", device.reserved));
        rendered.push_str(&format!(
            "            device_kind: {},\n",
            device.device_kind
        ));
        rendered.push_str(&format!(
            "            mmio_guest_phys: {},\n",
            device.mmio_guest_phys
        ));
        rendered.push_str(&format!(
            "            mmio_size_bytes: {},\n",
            device.mmio_size_bytes
        ));
        rendered.push_str("        },\n");
    }
    rendered.push_str("    ],\n");
    rendered.push_str(&format!(
        "    hypervisor_reserve_phys: {},\n",
        layout_snapshot.hypervisor_reserve_phys
    ));
    rendered.push_str(&format!(
        "    hypervisor_reserve_bytes: {},\n",
        layout_snapshot.hypervisor_reserve_bytes
    ));
    rendered.push_str(&format!(
        "    host_network_enabled: {},\n",
        layout_snapshot.host_network_enabled
    ));
    rendered.push_str(&format!(
        "    host_network_backend: {},\n",
        layout_snapshot.host_network_backend
    ));
    rendered.push_str(&format!(
        "    host_network_reserved: {},\n",
        render_byte_array(&layout_snapshot.host_network_reserved)
    ));
    rendered.push_str(&format!(
        "    host_network_interface_count: {},\n",
        layout_snapshot.host_network_interface_count
    ));
    rendered.push_str("    host_network_interfaces: [\n");
    for interface in layout_snapshot.host_network_interfaces.iter() {
        rendered.push_str("        hv_boot_abi::LayoutHostNetworkSnapshot {\n");
        rendered.push_str(&format!("            vm_id: {},\n", interface.vm_id));
        rendered.push_str(&format!("            segment: {},\n", interface.segment));
        rendered.push_str(&format!("            bus: {},\n", interface.bus));
        rendered.push_str(&format!("            device: {},\n", interface.device));
        rendered.push_str(&format!("            function: {},\n", interface.function));
        rendered.push_str(&format!("            backend: {},\n", interface.backend));
        rendered.push_str(&format!(
            "            tap_ifname_len: {},\n",
            interface.tap_ifname_len
        ));
        rendered.push_str(&format!(
            "            tap_ifname: {},\n",
            render_byte_array(&interface.tap_ifname)
        ));
        rendered.push_str(&format!(
            "            netdev_id_len: {},\n",
            interface.netdev_id_len
        ));
        rendered.push_str(&format!("            reserved: {},\n", interface.reserved));
        rendered.push_str(&format!(
            "            netdev_id: {},\n",
            render_byte_array(&interface.netdev_id)
        ));
        rendered.push_str("        },\n");
    }
    rendered.push_str("    ],\n");
    rendered.push_str("};\n");
    rendered
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn write_file_creates_nested_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_file(
            dir.path().join("nested/deep/file.txt"),
            String::from("payload"),
        )
        .expect("write nested file");
        assert!(dir.path().join("nested/deep/file.txt").is_file());
    }

    #[test]
    fn render_hypervisor_embedded_config_writes_snapshot_fields() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = hv_config_model::compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let rendered =
            render_hypervisor_embedded_config(&compiled.digest.bytes, &layout, &compiled)
                .expect("render");
        assert!(rendered.contains("hypervisor_reserve_bytes"));
        assert!(rendered.contains("REQUIREMENTS_SNAPSHOT"));
        assert!(rendered.contains("LAYOUT_SNAPSHOT"));
        assert!(rendered.contains("guest_region_count"));
        assert!(rendered.contains("partition_id_len"));
        assert!(rendered.contains("device_role"));
        assert!(rendered.contains("host_network_enabled"));
        assert!(rendered.contains("mmio_guest_phys"));
        assert!(rendered.contains("mmio_size_bytes"));
    }

    #[test]
    fn render_supporting_artifacts_include_expected_fields() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = hv_config_model::compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let source = std::path::Path::new("configs/qemu.yaml");
        assert!(render_build_manifest(source, &compiled).contains("schema_version"));
        assert!(render_qemu_args(&compiled).contains("-machine"));
        assert!(render_platform_layout(&layout).contains("hypervisor_reserve"));
        assert!(render_static_platform_rs(&compiled)
            .expect("static rs")
            .contains("CONFIG_SHA256"));
    }
}
