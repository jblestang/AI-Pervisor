//! Embeds compiled configuration digest and requirements snapshot into the hypervisor image.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use hv_config_model::compile_config_from_str;
use hv_hypervisor::requirements_snapshot_from_platform;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    let workspace = Path::new(&manifest_dir).join("../..");
    let default_config = workspace.join("configs/qemu.yaml");
    let config_path = env::var("HV_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or(default_config);
    let config_path = if config_path.is_absolute() {
        config_path
    } else {
        workspace.join(config_path)
    };

    let default_digest = workspace.join("build/config.sha256");
    let digest_path = env::var("HV_CONFIG_DIGEST_PATH")
        .map(PathBuf::from)
        .unwrap_or(default_digest);
    let digest_path = if digest_path.is_absolute() {
        digest_path
    } else {
        workspace.join(digest_path)
    };

    let yaml = fs::read_to_string(&config_path).unwrap_or_else(|err| {
        panic!(
            "failed to read config at {}: {err}. Run `cargo xtask config generate` first.",
            config_path.display()
        )
    });
    let compiled = compile_config_from_str(&yaml).unwrap_or_else(|err| {
        panic!("failed to compile config at {}: {err}", config_path.display())
    });

    let hex = fs::read_to_string(&digest_path).unwrap_or_else(|err| {
        panic!(
            "failed to read config digest at {}: {err}. Run `cargo xtask config generate` first.",
            digest_path.display()
        )
    });
    let bytes = hex::decode(hex.trim()).unwrap_or_else(|err| {
        panic!(
            "invalid config digest hex in {}: {err}",
            digest_path.display()
        )
    });
    if bytes.len() != 32 {
        panic!(
            "expected 32-byte config digest in {}, found {} bytes",
            digest_path.display(),
            bytes.len()
        );
    }

    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes);

    let snapshot = requirements_snapshot_from_platform(&compiled.requirements, digest)
        .unwrap_or_else(|err| panic!("failed to build requirements snapshot: {err}"));

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let generated = Path::new(&out_dir).join("embedded_config.rs");
    fs::write(&generated, render_embedded_config(&digest, &snapshot)).expect("write embedded rs");

    println!("cargo:rerun-if-env-changed=HV_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=HV_CONFIG_DIGEST_PATH");
    println!("cargo:rerun-if-changed={}", config_path.display());
    println!("cargo:rerun-if-changed={}", digest_path.display());
}

fn render_embedded_config(digest: &[u8; 32], snapshot: &hv_boot_abi::RequirementsSnapshot) -> String {
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
    rendered.push_str(&format!(
        "    min_ram_bytes: {},\n",
        snapshot.min_ram_bytes
    ));
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
    rendered.push_str("    config_digest: [");
    for (index, byte) in snapshot.config_digest.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&format!("0x{byte:02X}"));
    }
    rendered.push_str("],\n};\n");
    rendered
}
