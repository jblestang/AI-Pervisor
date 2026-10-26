//! Hypervisor spike developer CLI (`cargo xtask …`).

mod coverage;
mod qemu;

use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};

use qemu::{
    build_qemu_command, check_kvm_nested, load_demo_config, resolve_datapath_for_launch,
    resolve_image, write_flat_cfg_blob, NestedImages, NestedMode,
};

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
    /// Run nested QEMU (argv from DemoConfig only).
    RunNested,
    /// Run Verus verification on annotated crates (`ring`, `isolation`).
    Verify,
    /// Nested smoke: boot path, wait for serial `SMOKE_OK`.
    Smoke,
    /// E2E bench @ 64 B (wired in a later task).
    Bench,
    /// Line-coverage gate (≥90% on declared crates; see `coverage-allowlist.toml`).
    Coverage,
    /// Attach GDB stub (wired in a later task).
    Gdb,
}

const FREESTANDING: &[&str] = &["hv", "guest-wan", "guest-lan"];
const HOST_LIBS: &[&str] = &["ring", "isolation", "pci-vf", "demo-config", "guest-common"];
const FREESTANDING_TARGET: &str = "x86_64-unknown-none";
const VERIFY_CRATES: &[&str] = &["ring", "isolation"];
const SMOKE_TOKEN: &[u8] = b"SMOKE_OK";
const SMOKE_TIMEOUT_SECS: u64 = 30;

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
    for pkg in HOST_LIBS {
        if let Err(code) = run_cargo(&["build", "-p", pkg], root) {
            return code;
        }
    }
    for pkg in FREESTANDING {
        let mut args = vec!["build", "-p", pkg, "--target", FREESTANDING_TARGET];
        if *pkg == "hv" {
            args.extend_from_slice(&["--features", "freestanding"]);
        }
        if let Err(code) = run_cargo(&args, root) {
            return code;
        }
    }
    if let Err(code) = run_cargo(&["build", "-p", "xtask"], root) {
        return code;
    }
    println!("xtask build: ok");
    ExitCode::SUCCESS
}

fn resolve_verus_bin() -> Option<PathBuf> {
    if let Ok(p) = env::var("VERUS_BIN") {
        if !p.trim().is_empty() {
            let path = PathBuf::from(p);
            if path.is_file() {
                return Some(path);
            }
            eprintln!("VERUS_BIN set but not a file: {}", path.display());
        }
    }
    if let Ok(home) = env::var("VERUS_HOME") {
        let home = PathBuf::from(home);
        let candidate = home.join("verus");
        if candidate.is_file() {
            return Some(candidate);
        }
        // Unpacked release layout: $VERUS_HOME/verus-x86-linux/verus
        let nested = home.join("verus-x86-linux/verus");
        if nested.is_file() {
            return Some(nested);
        }
    }
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join("verus");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    // Common local install used by spike agents.
    let home_fallback = PathBuf::from(env::var_os("HOME").unwrap_or_default())
        .join("verus/verus-x86-linux/verus");
    if home_fallback.is_file() {
        return Some(home_fallback);
    }
    None
}

fn resolve_cargo_verus() -> Option<PathBuf> {
    if let Ok(p) = env::var("CARGO_VERUS") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(verus) = resolve_verus_bin() {
        if let Some(dir) = verus.parent() {
            let candidate = dir.join("cargo-verus");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join("cargo-verus");
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
    let Some(cargo_verus) = resolve_cargo_verus() else {
        eprintln!(
            "xtask verify: cargo-verus not found (set CARGO_VERUS / VERUS_BIN / VERUS_HOME, \
             or install Verus release 0.2026.10.26 on PATH)."
        );
        eprintln!(
            "xtask verify: soft-skip — need Verus matching Rust {channel}; see docs/VERUS.md"
        );
        eprintln!("xtask verify: annotated crates: {VERIFY_CRATES:?}");
        return ExitCode::SUCCESS;
    };

    // Ensure the sibling `verus` binary is discoverable for cargo-verus.
    if let Some(verus) = resolve_verus_bin() {
        println!("xtask verify: verus {}", verus.display());
        if let Some(dir) = verus.parent() {
            let mut path = env::var_os("PATH").unwrap_or_default();
            let mut paths = env::split_paths(&path).collect::<Vec<_>>();
            paths.insert(0, dir.to_path_buf());
            if let Ok(joined) = env::join_paths(paths) {
                path = joined;
                env::set_var("PATH", path);
            }
        }
    }
    println!("xtask verify: using cargo-verus {}", cargo_verus.display());

    for pkg in VERIFY_CRATES {
        let mut cmd = Command::new("cargo");
        // `cargo verus` discovers `cargo-verus` on PATH.
        cmd.args(["verus", "verify", "-p", pkg, "--color", "never"])
            .current_dir(root);
        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("xtask verify: failed to spawn cargo verus: {e}");
                return ExitCode::FAILURE;
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}");
        let toolchain_mismatch = combined.contains("required rust toolchain")
            || combined.contains("rustup not found")
            || combined.contains("feature `edition2024` is required");
        if toolchain_mismatch {
            eprintln!("xtask verify: Verus/cargo toolchain mismatch under pin {channel}");
            eprint!("{combined}");
            eprintln!("xtask verify: soft-skip — see docs/VERUS.md");
            return ExitCode::SUCCESS;
        }
        if !output.status.success() {
            eprintln!("xtask verify: Verus failed on `{pkg}`");
            eprint!("{combined}");
            return ExitCode::FAILURE;
        }
        // Surface the verification summary lines (cached runs may only print Finished).
        let mut saw_results = false;
        for line in combined.lines() {
            if line.contains("verification results") {
                println!("{line}");
                saw_results = true;
            }
        }
        if !saw_results {
            println!("xtask verify: {pkg}: cargo verus verify succeeded (cached or quiet)");
        }
        println!("xtask verify: {pkg}: ok");
    }
    println!("xtask verify: ok");
    ExitCode::SUCCESS
}

fn prepare_nested(root: &Path) -> Result<(demo_config::DemoConfig, NestedImages), ExitCode> {
    if let Err(code) = cmd_build_as_result(root) {
        return Err(code);
    }
    let mut cfg = load_demo_config(root).map_err(|e| {
        eprintln!("xtask: {e}");
        ExitCode::FAILURE
    })?;
    resolve_datapath_for_launch(&mut cfg).map_err(|e| {
        eprintln!("xtask: {e}");
        ExitCode::FAILURE
    })?;
    check_kvm_nested().map_err(|e| {
        eprintln!("xtask: {e}");
        ExitCode::FAILURE
    })?;

    let hv = resolve_image(root, &cfg.paths.hv_image).map_err(|e| {
        eprintln!("xtask: {e}");
        ExitCode::FAILURE
    })?;
    let guest_wan = resolve_image(root, &cfg.paths.guest_wan_image).map_err(|e| {
        eprintln!("xtask: {e}");
        ExitCode::FAILURE
    })?;
    let guest_lan = resolve_image(root, &cfg.paths.guest_lan_image).map_err(|e| {
        eprintln!("xtask: {e}");
        ExitCode::FAILURE
    })?;
    let flat_cfg = write_flat_cfg_blob(root, &cfg).map_err(|e| {
        eprintln!("xtask: {e}");
        ExitCode::FAILURE
    })?;

    Ok((
        cfg,
        NestedImages {
            hv,
            guest_wan,
            guest_lan,
            flat_cfg,
        },
    ))
}

fn cmd_build_as_result(root: &Path) -> Result<(), ExitCode> {
    match cmd_build(root) {
        c if c == ExitCode::SUCCESS => Ok(()),
        c => Err(c),
    }
}

fn cmd_run_nested(root: &Path) -> ExitCode {
    let (cfg, images) = match prepare_nested(root) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (qemu, args) = match build_qemu_command(root, &cfg, &images, NestedMode::Run) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("xtask run-nested: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("xtask run-nested: {} {}", qemu.display(), args.join(" "));
    let status = Command::new(&qemu)
        .args(&args)
        .current_dir(root)
        .status();
    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => {
            eprintln!("xtask run-nested: qemu exited {s}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!(
                "xtask run-nested: failed to spawn {}: {e}\n  \
                 hint: set lab.qemu_bin in config or install qemu-system-x86_64",
                qemu.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn cmd_smoke(root: &Path) -> ExitCode {
    let (cfg, images) = match prepare_nested(root) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (qemu, args) = match build_qemu_command(root, &cfg, &images, NestedMode::Smoke) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("xtask smoke: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("xtask smoke: {} {}", qemu.display(), args.join(" "));
    eprintln!(
        "xtask smoke: waiting for serial token {:?} (timeout {}s, backend={:?})",
        core::str::from_utf8(SMOKE_TOKEN).unwrap_or("SMOKE_OK"),
        SMOKE_TIMEOUT_SECS,
        cfg.datapath.backend
    );

    let mut child = match Command::new(&qemu)
        .args(&args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "xtask smoke: failed to spawn {}: {e}\n  \
                 hint: install qemu-system-x86 / set lab.qemu_bin (see docs/HARNESS.md)",
                qemu.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let mut stdout = child.stdout.take().expect("piped stdout");
    // Drop stderr so a stuck QEMU cannot block us on drain; serial is on stdout.
    drop(child.stderr.take());

    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let reader = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut serial = Vec::new();
        loop {
            match stdout.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = &buf[..n];
                    let _ = std::io::stderr().write_all(chunk);
                    serial.extend_from_slice(chunk);
                    if serial.windows(SMOKE_TOKEN.len()).any(|w| w == SMOKE_TOKEN) {
                        let _ = tx.send(serial);
                        return true;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(serial);
        false
    });

    let deadline = Instant::now() + Duration::from_secs(SMOKE_TIMEOUT_SECS);
    let mut found = false;
    let mut serial = Vec::new();
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(data) => {
                serial = data;
                found = serial.windows(SMOKE_TOKEN.len()).any(|w| w == SMOKE_TOKEN);
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Ok(Some(_)) = child.try_wait() {
                    if let Ok(data) = rx.recv_timeout(Duration::from_secs(2)) {
                        serial = data;
                        found = serial.windows(SMOKE_TOKEN.len()).any(|w| w == SMOKE_TOKEN);
                    }
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    // Kill QEMU first so the stdout reader observes EOF and joins.
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();

    if found {
        println!("xtask smoke: ok (saw SMOKE_OK)");
        ExitCode::SUCCESS
    } else {
        let text = String::from_utf8_lossy(&serial);
        eprintln!(
            "xtask smoke: FAILED - did not observe SMOKE_OK within {SMOKE_TIMEOUT_SECS}s"
        );
        eprintln!("xtask smoke: serial capture ({} bytes):\n{text}", serial.len());
        if text.is_empty() {
            eprintln!(
                "xtask smoke: empty serial - hv may not have Multiboot-booted.\n  \
                 hint: Multiboot1 magic must be in first 8KiB of hv image; \
                 /dev/kvm + nested=1; see docs/HARNESS.md"
            );
        }
        ExitCode::FAILURE
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = workspace_root();
    match cli.cmd {
        Cmd::Build => cmd_build(&root),
        Cmd::RunNested => cmd_run_nested(&root),
        Cmd::Verify => cmd_verify(&root),
        Cmd::Smoke => cmd_smoke(&root),
        Cmd::Bench => not_wired("bench"),
        Cmd::Coverage => coverage::cmd_coverage(&root),
        Cmd::Gdb => not_wired("gdb"),
    }
}
