//! Build real datapath guest ELFs from `guests/` source trees.

use std::process::Command;

const PARTITIONS: &[&str] = &["in", "mid", "out"];

/// Builds guest ELFs and copies them to `guests/guest-*/build/guest-*.elf`.
pub fn run_build_guests() -> i32 {
    let workspace = crate::workspace_root();
    let guests_dir = workspace.join("guests");

    if !guests_dir.is_dir() {
        eprintln!("guests workspace missing at {}", guests_dir.display());
        return 1;
    }

    if Command::new("rustup")
        .args(["target", "add", "x86_64-unknown-none"])
        .status()
        .is_err()
    {
        eprintln!("failed to invoke rustup to install x86_64-unknown-none target");
        return 1;
    }

    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&guests_dir)
        .status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("guest build failed with status {status}");
            return 1;
        }
        Err(err) => {
            eprintln!("failed to run guest build: {err}");
            return 1;
        }
    }

    let target_dir = guests_dir.join("target/x86_64-unknown-none/release");
    for partition in PARTITIONS {
        let artifact = target_dir.join(format!("guest-{partition}"));
        if !artifact.is_file() {
            eprintln!("missing guest artifact: {}", artifact.display());
            return 1;
        }
        let output_dir = guests_dir.join(format!("guest-{partition}")).join("build");
        if std::fs::create_dir_all(&output_dir).is_err() {
            eprintln!("failed to create {}", output_dir.display());
            return 1;
        }
        let output_elf = output_dir.join(format!("guest-{partition}.elf"));
        if std::fs::copy(&artifact, &output_elf).is_err() {
            eprintln!(
                "failed to copy {} to {}",
                artifact.display(),
                output_elf.display()
            );
            return 1;
        }
        eprintln!("built {}", output_elf.display());
    }

    0
}
