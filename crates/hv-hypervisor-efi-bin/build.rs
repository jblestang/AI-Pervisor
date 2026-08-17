//! Embeds compiled configuration digest and requirements snapshot into the hypervisor image.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    let workspace = Path::new(&manifest_dir).join("../..");
    let default_source = workspace.join("build/hypervisor_embedded_config.rs");
    let source_path = env::var("HV_HYPERVISOR_EMBEDDED_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or(default_source);
    let source_path = if source_path.is_absolute() {
        source_path
    } else {
        workspace.join(source_path)
    };

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let generated = Path::new(&out_dir).join("embedded_config.rs");
    fs::copy(&source_path, &generated).unwrap_or_else(|err| {
        panic!(
            "failed to copy embedded hypervisor config from {}: {err}. Run `cargo xtask config generate` first.",
            source_path.display()
        )
    });

    println!("cargo:rerun-if-env-changed=HV_HYPERVISOR_EMBEDDED_CONFIG_PATH");
    println!("cargo:rerun-if-changed={}", source_path.display());
}
