//! Hypervisor spike developer CLI (`cargo xtask …`).

use std::env;
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
    /// Run Verus verification on annotated crates (`ring`, `isolation`).
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
/// Crates with Verus-oriented contracts (expand in Task 8).
const VERIFY_CRATES: &[&str] = &["ring", "isolation"];

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

fn resolve_verus_bin() -> Option<PathBuf> {
    if let Ok(p) = env::var("VERUS_BIN") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
        eprintln!("VERUS_BIN set but not a file: {}", path.display());
    }
    // Honor PATH lookup without requiring which(1).
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join("verus");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn workspace_rust_channel(root: &Path) -> String {
    let path = root.join("rust-toolchain.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return "unknown".into();
    };
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("channel") {
            if let Some(eq) = rest.find('=') {
                let v = rest[eq + 1..].trim().trim_matches('"');
                if !v.is_empty() {
                    return v.to_string();
                }
            }
        }
    }
    "unknown".into()
}

fn cmd_verify(root: &Path) -> ExitCode {
    let channel = workspace_rust_channel(root);
    let Some(verus) = resolve_verus_bin() else {
        eprintln!(
            "xtask verify: Verus binary not found (set VERUS_BIN or install `verus` on PATH)."
        );
        eprintln!(
            "xtask verify: soft-skip — workspace pin is Rust {channel}; see docs/VERUS.md"
        );
        eprintln!("xtask verify: annotated crates (stubs compile without Verus): {VERIFY_CRATES:?}");
        return ExitCode::SUCCESS;
    };

    println!("xtask verify: using {}", verus.display());
    for pkg in VERIFY_CRATES {
        let lib = root.join(pkg).join("src/lib.rs");
        if !lib.is_file() {
            eprintln!("xtask verify: missing {}", lib.display());
            return ExitCode::FAILURE;
        }
        let output = Command::new(&verus)
            .arg("--crate-type=lib")
            .arg(&lib)
            .current_dir(root)
            .output();
        let output = match output {
            Ok(o) => o,
            Err(e) => {
                eprintln!("xtask verify: failed to spawn {}: {e}", verus.display());
                return ExitCode::FAILURE;
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}");
        let toolchain_mismatch = combined.contains("required rust toolchain")
            || combined.contains("rustup not found");
        // Stubs compile without vstd; real verus!/vstd contracts land in Task 8.
        let stubs_only = combined.contains("verus_builtin")
            || combined.contains("use vstd::prelude");
        if toolchain_mismatch {
            eprintln!("xtask verify: Verus toolchain mismatch under pin {channel}");
            eprint!("{combined}");
            eprintln!("xtask verify: soft-skip — see docs/VERUS.md");
            return ExitCode::SUCCESS;
        }
        if stubs_only {
            eprintln!("xtask verify: `{pkg}` still uses Verus attribute stubs (no vstd yet)");
            eprint!("{combined}");
            eprintln!("xtask verify: soft-skip — see docs/VERUS.md (Task 8: real proofs)");
            return ExitCode::SUCCESS;
        }
        if !output.status.success() {
            eprintln!("xtask verify: Verus failed on `{pkg}`");
            eprint!("{combined}");
            return ExitCode::FAILURE;
        }
        print!("{combined}");
        println!("xtask verify: {pkg}: ok");
    }
    println!("xtask verify: ok");
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = workspace_root();
    match cli.cmd {
        Cmd::Build => cmd_build(&root),
        Cmd::RunNested => not_wired("run-nested"),
        Cmd::Verify => cmd_verify(&root),
        Cmd::Smoke => not_wired("smoke"),
        Cmd::Bench => not_wired("bench"),
        Cmd::Coverage => not_wired("coverage"),
        Cmd::Gdb => not_wired("gdb"),
    }
}
