//! Hypervisor spike developer CLI (`cargo xtask …`).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "xtask", about = "Hypervisor spike developer tasks")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Build workspace members (std libs + no_std freestanding images).
    Build,
    /// Run nested QEMU (wired in a later task).
    RunNested,
    /// Run Verus verification (wired in a later task).
    Verify,
    /// Nested smoke test (wired in a later task).
    Smoke,
    /// E2E bench @ 64 B (wired in a later task).
    Bench,
    /// Coverage gate (wired in a later task).
    Coverage,
    /// Attach GDB stub (wired in a later task).
    Gdb,
}

const FREESTANDING: &[&str] = &["hv", "guest-wan", "guest-lan"];
const HOST_LIBS: &[&str] = &["ring", "isolation", "pci-vf", "demo-config"];
const FREESTANDING_TARGET: &str = "x86_64-unknown-none";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace member")
        .to_path_buf()
}

fn not_wired(name: &str) -> ExitCode {
    eprintln!("{name}: not wired");
    ExitCode::from(2)
}

fn run_cargo(args: &[&str], root: &Path) -> Result<(), ExitCode> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|e| {
            eprintln!("failed to spawn cargo: {e}");
            ExitCode::FAILURE
        })?;
    if status.success() {
        Ok(())
    } else {
        eprintln!("cargo {:?} failed with {status}", args);
        Err(ExitCode::FAILURE)
    }
}

fn ensure_example_config(root: &Path) -> Result<(), ExitCode> {
    let path = root.join("config/demo.example.toml");
    demo_config::DemoConfig::from_path(&path).map_err(|e| {
        eprintln!("config/demo.example.toml invalid: {e}");
        ExitCode::FAILURE
    })?;
    Ok(())
}

fn cmd_build(root: &Path) -> ExitCode {
    if let Err(code) = ensure_example_config(root) {
        return code;
    }
    // Ensure config schema crate and host-testable libs build on the host triple.
    for pkg in HOST_LIBS {
        if let Err(code) = run_cargo(&["build", "-p", pkg], root) {
            return code;
        }
    }
    // Freestanding Type-1 / guest images.
    for pkg in FREESTANDING {
        if let Err(code) = run_cargo(
            &["build", "-p", pkg, "--target", FREESTANDING_TARGET],
            root,
        ) {
            return code;
        }
    }
    // xtask itself (std).
    if let Err(code) = run_cargo(&["build", "-p", "xtask"], root) {
        return code;
    }
    println!("xtask build: ok");
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = workspace_root();
    match cli.cmd {
        Cmd::Build => cmd_build(&root),
        Cmd::RunNested => not_wired("run-nested"),
        Cmd::Verify => not_wired("verify"),
        Cmd::Smoke => not_wired("smoke"),
        Cmd::Bench => not_wired("bench"),
        Cmd::Coverage => not_wired("coverage"),
        Cmd::Gdb => not_wired("gdb"),
    }
}
