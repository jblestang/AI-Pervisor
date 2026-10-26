//! Assemble Multiboot2 trampoline for freestanding `hv` images only.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("x86_64-unknown-none") {
        return;
    }
    // Only freestanding feature builds the binary that needs the trampoline.
    let freestanding = env::var("CARGO_FEATURE_FREESTANDING").is_ok();
    if !freestanding {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let src = manifest_dir.join("src/boot.s");
    let obj = out_dir.join("boot.o");

    println!("cargo:rerun-if-changed={}", src.display());
    println!("cargo:rerun-if-changed=linker.ld");

    let status = Command::new("as")
        .args(["--64", "-o"])
        .arg(&obj)
        .arg(&src)
        .status()
        .expect("failed to spawn `as` (binutils) for hv/src/boot.s");
    if !status.success() {
        panic!("assembling hv/src/boot.s failed: {status}");
    }

    println!("cargo:rustc-link-arg={}", obj.display());
    println!(
        "cargo:rustc-link-arg=-T{}",
        manifest_dir.join("linker.ld").display()
    );
    println!("cargo:rustc-link-arg=-nostdlib");
    // Static non-PIE image at the Multiboot load address.
    println!("cargo:rustc-link-arg=-no-pie");
}
