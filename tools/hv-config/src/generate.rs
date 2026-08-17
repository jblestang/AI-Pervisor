//! Artifact generation for human review and build integration.

use std::fs;
use std::path::{Path, PathBuf};

use hv_config_model::compile_config_from_path;

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
        output.join("platform-requirements.txt"),
        render_requirements(compiled),
    )?;
    write_file(output.join("ipc-map.txt"), render_ipc_map(compiled))?;
    write_file(output.join("pci-map.txt"), render_pci_map(compiled))?;
    write_file(
        output.join("cpu-topology.txt"),
        render_cpu_topology(compiled),
    )?;
    write_file(output.join("memory-map.txt"), render_memory_map(compiled))?;
    write_file(
        output.join("core-ownership.txt"),
        render_core_ownership(compiled),
    )?;
    write_file(output.join("boot-layout.txt"), render_boot_layout(compiled))?;
    write_file(
        output.join("guest-images.txt"),
        render_guest_images(compiled),
    )?;
    write_file(output.join("qemu-args.txt"), render_qemu_args(compiled))?;
    write_file(
        output.join("boot-manifest.txt"),
        render_boot_manifest(compiled),
    )?;
    write_file(
        output.join("build-manifest.txt"),
        render_build_manifest(source, compiled),
    )?;
    write_file(output.join("config.sha256"), compiled.digest.to_hex())?;
    write_file(
        output.join("static-platform.rs"),
        render_static_platform_rs(compiled)?,
    )?;

    let intent_json = serde_json::to_string_pretty(&compiled.intent)?;
    write_file(output.join("static-intent.json"), intent_json)?;

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
    for (bdf, vm_id, kind) in &compiled.intent.pci_intent.devices {
        out.push_str(&format!(
            "bdf={:04x}:{:02x}:{:02x}.{} vm_id={} kind={}\n",
            bdf.segment.raw(),
            bdf.bus.raw(),
            bdf.device.raw(),
            bdf.function.raw(),
            vm_id.raw(),
            kind,
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
}
