//! Workspace task runner library.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

use std::ffi::OsString;
use std::path::Path;
use std::process::Command as ProcessCommand;

mod build_guests;
mod constants;
mod datapath_benchmark;
mod host_net_taps;
mod live_qemu_smoke;
mod ovmf_smoke;
mod qemu_network;

use clap::{Parser, Subcommand};
use constants::{
    DEFAULT_BOOT_CHAIN_OUTPUT_DIR, DEFAULT_COVERAGE_MIN_LINES, DEFAULT_EFI_CONFIG_PATH,
    DEFAULT_EFI_OUTPUT_PATH, DEFAULT_FUZZ_RUNS, DEFAULT_HYPERVISOR_EFI_OUTPUT_PATH,
    DEFAULT_LIVE_BOOT_CHAIN_OUTPUT_DIR, DEFAULT_LIVE_QEMU_SMOKE_TIMEOUT_SECS,
    DEFAULT_OVMF_SMOKE_CONFIG_PATH, DEFAULT_OVMF_SMOKE_TIMEOUT_SECS,
    HYPERVISOR_EFI_DATAPATH_BENCHMARK_FEATURE, HYPERVISOR_EFI_DATAPATH_FOUNDATION_FEATURE,
    HYPERVISOR_EFI_DATAPATH_GUESTS_FEATURE, HYPERVISOR_EFI_DATAPATH_GUEST_EXECUTION_FEATURE,
    HYPERVISOR_EFI_DATAPATH_GUEST_LIVE_FEATURE, HYPERVISOR_EFI_DATAPATH_GUEST_RELAY_LIVE_FEATURE,
    HYPERVISOR_EFI_DATAPATH_GUEST_RELAY_MEASUREMENT_FEATURE,
    HYPERVISOR_EFI_DATAPATH_GUEST_SOURCES_FEATURE,
    HYPERVISOR_EFI_DATAPATH_GUEST_THROUGHPUT_FEATURE, HYPERVISOR_EFI_DATAPATH_LIVE_FEATURE,
    HYPERVISOR_EFI_DATAPATH_MALICIOUS_FEATURE, HYPERVISOR_EFI_DATAPATH_RUNTIME_FEATURE,
    HYPERVISOR_EFI_REAL_HW_FEATURE, HYPERVISOR_EFI_VMX_LAUNCH_FEATURE,
};
use hv_config::constants::DEFAULT_CONFIG_OUTPUT_DIR;

/// Runs `cargo test --workspace`.
pub fn run_tests() -> i32 {
    run_tests_with(run)
}

fn run_tests_with(runner: fn(&str, &[&str]) -> i32) -> i32 {
    test_command(runner)
}

/// Runs `cargo build --workspace`.
pub fn run_build() -> i32 {
    run("cargo", &["build", "--workspace"])
}

/// Validates a configuration file through the `hv-config` library.
pub fn run_config_validate(path: &str) -> i32 {
    hv_config::validate_config(Path::new(path))
}

/// Generates configuration artifacts through the `hv-config` library.
pub fn run_config_generate(path: &str, output: &str) -> i32 {
    hv_config::generate::generate(Path::new(path), Path::new(output))
}

/// Runs libFuzzer smoke tests for all parsing fuzz targets.
pub fn run_fuzz(runs: u32) -> i32 {
    fuzz_command(runs, run_with_cxx_gpp)
}

/// Builds the UEFI loader `.efi` image for the given configuration.
pub fn run_build_efi(config_path: &str, output_path: &str) -> i32 {
    run_build_efi_with(
        config_path,
        output_path,
        run_config_generate,
        run_with_cxx_gpp,
        build_efi_image,
    )
}

fn run_build_efi_with(
    config_path: &str,
    output_path: &str,
    generate: fn(&str, &str) -> i32,
    install_target: fn(&str, &[&str]) -> i32,
    build: fn(&std::path::Path, &str) -> i32,
) -> i32 {
    let workspace = workspace_root();
    let config = workspace.join(config_path);
    let output = workspace.join(output_path);
    let build_dir = workspace.join("build");

    if generate(
        config.to_str().unwrap_or(config_path),
        build_dir.to_str().unwrap_or("build"),
    ) != 0
    {
        return 1;
    }

    if install_target("rustup", &["target", "add", "x86_64-unknown-uefi"]) != 0 {
        return 1;
    }

    let digest_path = build_dir
        .join("config.sha256")
        .canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| {
            build_dir
                .join("config.sha256")
                .to_string_lossy()
                .into_owned()
        });

    if build(&workspace, &digest_path) != 0 {
        return 1;
    }

    copy_efi_artifact(output.to_str().unwrap_or(output_path))
}

/// Builds the UEFI hypervisor `.efi` image for the given configuration.
pub fn run_build_hypervisor_efi(config_path: &str, output_path: &str) -> i32 {
    run_build_hypervisor_efi_with(
        config_path,
        output_path,
        run_config_generate,
        run_with_cxx_gpp,
        build_hypervisor_efi_image,
    )
}

/// Runs an OVMF/QEMU smoke boot of the loader + hypervisor chain.
pub fn run_ovmf_smoke_boot(
    config_path: &str,
    boot_chain_dir: &str,
    timeout_secs: u64,
    build_first: bool,
) -> i32 {
    ovmf_smoke::run_ovmf_smoke_boot(config_path, boot_chain_dir, timeout_secs, build_first)
}

/// Runs a KVM/QEMU REAL_HW smoke boot of the loader + hypervisor chain.
pub fn run_live_qemu_smoke(
    config_path: &str,
    boot_chain_dir: &str,
    timeout_secs: u64,
    build_first: bool,
) -> i32 {
    live_qemu_smoke::run_live_qemu_smoke(config_path, boot_chain_dir, timeout_secs, build_first)
}

/// Creates and brings up host tap interfaces declared in the config host network plan.
pub fn run_setup_host_net_taps(config_path: &str) -> i32 {
    match host_net_taps::setup_host_net_taps(config_path) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

/// Runs live QEMU smoke with explicit strict/executed options.
pub fn run_live_qemu_smoke_with_options(
    config_path: &str,
    boot_chain_dir: &str,
    timeout_secs: u64,
    build_first: bool,
    options: &live_qemu_smoke::LiveQemuSmokeOptions,
) -> i32 {
    live_qemu_smoke::run_live_qemu_smoke_with_options(
        config_path,
        boot_chain_dir,
        timeout_secs,
        build_first,
        options,
    )
}

/// Builds loader and hypervisor `.efi` images into one output directory.
pub fn run_build_boot_chain(config_path: &str, output_dir: &str) -> i32 {
    run_build_boot_chain_with(
        config_path,
        output_dir,
        run_config_generate,
        run_with_cxx_gpp,
        build_efi_image,
        build_hypervisor_efi_image,
    )
}

fn run_build_boot_chain_with(
    config_path: &str,
    output_dir: &str,
    generate: fn(&str, &str) -> i32,
    install_target: fn(&str, &[&str]) -> i32,
    build_loader: fn(&std::path::Path, &str) -> i32,
    build_hypervisor: fn(&std::path::Path, &str, &str) -> i32,
) -> i32 {
    let workspace = workspace_root();
    let output = workspace.join(output_dir);
    if std::fs::create_dir_all(&output).is_err() {
        eprintln!(
            "failed to create boot-chain output directory: {}",
            output.display()
        );
        return 1;
    }

    let loader_output = output.join("hv-loader.efi");
    let hypervisor_output = output.join("hv-hypervisor.efi");
    let loader_path = loader_output.to_string_lossy().into_owned();
    let hypervisor_path = hypervisor_output.to_string_lossy().into_owned();

    if run_build_efi_with(
        config_path,
        &loader_path,
        generate,
        install_target,
        build_loader,
    ) != 0
    {
        return 1;
    }
    if run_build_hypervisor_efi_with(
        config_path,
        &hypervisor_path,
        generate,
        install_target,
        build_hypervisor,
    ) != 0
    {
        return 1;
    }

    eprintln!(
        "built boot chain: {} and {}",
        loader_output.display(),
        hypervisor_output.display()
    );
    0
}

/// Builds loader and REAL_HW hypervisor `.efi` images into one output directory.
pub fn run_build_boot_chain_live(config_path: &str, output_dir: &str) -> i32 {
    if build_guests::run_build_guests() != 0 {
        return 1;
    }
    run_build_boot_chain_live_with(
        config_path,
        output_dir,
        run_config_generate,
        run_with_cxx_gpp,
        build_efi_image,
        build_hypervisor_efi_image_live,
    )
}

fn run_build_boot_chain_live_with(
    config_path: &str,
    output_dir: &str,
    generate: fn(&str, &str) -> i32,
    install_target: fn(&str, &[&str]) -> i32,
    build_loader: fn(&std::path::Path, &str) -> i32,
    build_hypervisor: fn(&std::path::Path, &str, &str) -> i32,
) -> i32 {
    let workspace = workspace_root();
    let output = workspace.join(output_dir);
    if std::fs::create_dir_all(&output).is_err() {
        eprintln!(
            "failed to create boot-chain output directory: {}",
            output.display()
        );
        return 1;
    }

    let loader_output = output.join("hv-loader.efi");
    let hypervisor_output = output.join("hv-hypervisor.efi");
    let loader_path = loader_output.to_string_lossy().into_owned();
    let hypervisor_path = hypervisor_output.to_string_lossy().into_owned();

    if run_build_efi_with(
        config_path,
        &loader_path,
        generate,
        install_target,
        build_loader,
    ) != 0
    {
        return 1;
    }
    if run_build_hypervisor_efi_with(
        config_path,
        &hypervisor_path,
        generate,
        install_target,
        build_hypervisor,
    ) != 0
    {
        return 1;
    }

    eprintln!(
        "built REAL_HW boot chain: {} and {}",
        loader_output.display(),
        hypervisor_output.display()
    );
    0
}

fn run_build_hypervisor_efi_with(
    config_path: &str,
    output_path: &str,
    generate: fn(&str, &str) -> i32,
    install_target: fn(&str, &[&str]) -> i32,
    build: fn(&std::path::Path, &str, &str) -> i32,
) -> i32 {
    let workspace = workspace_root();
    let config = workspace.join(config_path);
    let output = workspace.join(output_path);
    let build_dir = workspace.join("build");

    if generate(
        config.to_str().unwrap_or(config_path),
        build_dir.to_str().unwrap_or("build"),
    ) != 0
    {
        return 1;
    }

    if install_target("rustup", &["target", "add", "x86_64-unknown-uefi"]) != 0 {
        return 1;
    }

    let digest_path = build_dir
        .join("config.sha256")
        .canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| {
            build_dir
                .join("config.sha256")
                .to_string_lossy()
                .into_owned()
        });
    let config_source = config
        .canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| config.to_string_lossy().into_owned());

    if build(&workspace, &digest_path, &config_source) != 0 {
        return 1;
    }

    copy_hypervisor_efi_artifact(output.to_str().unwrap_or(output_path))
}

fn build_hypervisor_efi_image(
    workspace: &std::path::Path,
    digest_path: &str,
    config_path: &str,
) -> i32 {
    build_hypervisor_efi_image_with(workspace, digest_path, config_path, &[], run_command)
}

fn build_hypervisor_efi_image_live(
    workspace: &std::path::Path,
    digest_path: &str,
    config_path: &str,
) -> i32 {
    build_hypervisor_efi_image_with(
        workspace,
        digest_path,
        config_path,
        &[
            HYPERVISOR_EFI_REAL_HW_FEATURE,
            HYPERVISOR_EFI_VMX_LAUNCH_FEATURE,
            HYPERVISOR_EFI_DATAPATH_FOUNDATION_FEATURE,
            HYPERVISOR_EFI_DATAPATH_LIVE_FEATURE,
            HYPERVISOR_EFI_DATAPATH_MALICIOUS_FEATURE,
            HYPERVISOR_EFI_DATAPATH_GUESTS_FEATURE,
            HYPERVISOR_EFI_DATAPATH_BENCHMARK_FEATURE,
            HYPERVISOR_EFI_DATAPATH_RUNTIME_FEATURE,
            HYPERVISOR_EFI_DATAPATH_GUEST_SOURCES_FEATURE,
            HYPERVISOR_EFI_DATAPATH_GUEST_LIVE_FEATURE,
            HYPERVISOR_EFI_DATAPATH_GUEST_EXECUTION_FEATURE,
            HYPERVISOR_EFI_DATAPATH_GUEST_THROUGHPUT_FEATURE,
            HYPERVISOR_EFI_DATAPATH_GUEST_RELAY_LIVE_FEATURE,
            HYPERVISOR_EFI_DATAPATH_GUEST_RELAY_MEASUREMENT_FEATURE,
        ],
        run_command,
    )
}

fn build_hypervisor_efi_image_with(
    workspace: &std::path::Path,
    digest_path: &str,
    config_path: &str,
    features: &[&str],
    runner: fn(ProcessCommand) -> i32,
) -> i32 {
    runner(hypervisor_efi_build_command(
        workspace,
        digest_path,
        config_path,
        features,
    ))
}

fn hypervisor_efi_build_command(
    workspace: &std::path::Path,
    digest_path: &str,
    config_path: &str,
    features: &[&str],
) -> ProcessCommand {
    let mut command = ProcessCommand::new("cargo");
    command
        .current_dir(workspace)
        .args([
            "build",
            "--release",
            "--manifest-path",
            "crates/hv-hypervisor-efi-bin/Cargo.toml",
            "--target",
            "x86_64-unknown-uefi",
        ])
        .env(
            "HV_HYPERVISOR_EMBEDDED_CONFIG_PATH",
            "build/hypervisor_embedded_config.rs",
        )
        .env("CXX", "g++");
    if !features.is_empty() {
        command.args(["--features", &features.join(",")]);
    }
    let _ = (digest_path, config_path);
    command
}

fn copy_hypervisor_efi_artifact(output_path: &str) -> i32 {
    let source = workspace_root()
        .join("crates/hv-hypervisor-efi-bin/target/x86_64-unknown-uefi/release/hv-hypervisor.efi");
    match std::fs::copy(&source, output_path) {
        Ok(_) => {
            eprintln!("built UEFI hypervisor: {output_path}");
            0
        }
        Err(err) => {
            eprintln!(
                "failed to copy {} to {output_path}: {err}",
                source.display()
            );
            1
        }
    }
}

fn build_efi_image(workspace: &std::path::Path, digest_path: &str) -> i32 {
    build_efi_image_with(workspace, digest_path, run_command)
}

fn build_efi_image_with(
    workspace: &std::path::Path,
    digest_path: &str,
    runner: fn(ProcessCommand) -> i32,
) -> i32 {
    runner(efi_build_command(workspace, digest_path))
}

fn efi_build_command(workspace: &std::path::Path, digest_path: &str) -> ProcessCommand {
    let mut command = ProcessCommand::new("cargo");
    command
        .current_dir(workspace)
        .args([
            "build",
            "--release",
            "--manifest-path",
            "crates/hv-loader-efi-bin/Cargo.toml",
            "--target",
            "x86_64-unknown-uefi",
        ])
        .env("HV_CONFIG_DIGEST_PATH", digest_path)
        .env("CXX", "g++");
    command
}

pub(crate) fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn run_command(mut command: ProcessCommand) -> i32 {
    match command.status() {
        Ok(status) => {
            if status.success() {
                0
            } else {
                match status.code() {
                    Some(code) => code,
                    None => 1,
                }
            }
        }
        Err(err) => {
            eprintln!("failed to run command: {err}");
            1
        }
    }
}

fn copy_efi_artifact(output_path: &str) -> i32 {
    let source = workspace_root()
        .join("crates/hv-loader-efi-bin/target/x86_64-unknown-uefi/release/hv-loader.efi");
    match std::fs::copy(&source, output_path) {
        Ok(_) => {
            eprintln!("built UEFI loader: {output_path}");
            0
        }
        Err(err) => {
            eprintln!(
                "failed to copy {} to {output_path}: {err}",
                source.display()
            );
            1
        }
    }
}

fn fuzz_command(runs: u32, runner: fn(&str, &[&str]) -> i32) -> i32 {
    if runner(
        "cargo",
        &["build", "--release", "--manifest-path", "fuzz/Cargo.toml"],
    ) != 0
    {
        return 1;
    }

    let runs_arg = format!("-runs={runs}");
    for target in constants::FUZZ_TARGETS {
        let binary = format!("fuzz/target/release/{target}");
        if runner(&binary, &[runs_arg.as_str(), "-max_total_time=30"]) != 0 {
            return 1;
        }
    }

    0
}

fn run_with_cxx_gpp(program: &str, args: &[&str]) -> i32 {
    let mut command = ProcessCommand::new(program);
    command.args(args).env("CXX", "g++");
    match command.status() {
        Ok(status) => {
            if status.success() {
                0
            } else {
                match status.code() {
                    Some(code) => code,
                    None => 1,
                }
            }
        }
        Err(err) => {
            eprintln!("failed to run {program}: {err}");
            1
        }
    }
}

/// Runs workspace coverage and fails below `min_lines` percent line coverage.
pub fn run_coverage(min_lines: u8) -> i32 {
    coverage_command_with(min_lines, spawn_llvm_cov_summary)
}

fn evaluate_coverage_output(min_lines: u8, stdout: &str, success: bool) -> i32 {
    if let Some(coverage) = parse_summary_line_coverage(stdout) {
        if coverage < f64::from(min_lines) {
            eprintln!("line coverage {coverage:.2}% is below minimum {min_lines}%");
            return 1;
        }
    } else {
        eprintln!("failed to parse line coverage from llvm-cov summary");
        return 1;
    }

    if success {
        0
    } else {
        1
    }
}

fn coverage_command(min_lines: u8) -> i32 {
    run_coverage(min_lines)
}

type CoverageSpawnFn = fn(u8) -> Result<(String, String, bool), i32>;

fn coverage_command_with(min_lines: u8, spawn: CoverageSpawnFn) -> i32 {
    let (stdout, stderr, success) = match spawn(min_lines) {
        Ok(result) => result,
        Err(code) => return code,
    };
    print!("{stdout}");
    eprint!("{stderr}");
    evaluate_coverage_output(min_lines, &stdout, success)
}

type CoveragePassRunner = fn(&[&str]) -> bool;

fn run_test_pass(args: &[&str]) -> bool {
    ProcessCommand::new("cargo")
        .arg("test")
        .args(args)
        .env("RUSTFLAGS", "--cfg=coverage")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_llvm_cov_pass(args: &[&str]) -> bool {
    if args == ["--build-guests"] {
        return build_guests::run_build_guests() == 0;
    }
    ProcessCommand::new("cargo")
        .arg("llvm-cov")
        .args(args)
        .arg("--no-report")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn spawn_llvm_cov_summary(min_lines: u8) -> Result<(String, String, bool), i32> {
    let mut command = ProcessCommand::new("cargo");
    spawn_llvm_cov_summary_with(min_lines, &mut command, run_llvm_cov_pass, run_test_pass)
}

fn spawn_llvm_cov_summary_with(
    min_lines: u8,
    command: &mut ProcessCommand,
    pass_runner: CoveragePassRunner,
    test_pass_runner: CoveragePassRunner,
) -> Result<(String, String, bool), i32> {
    let threshold = min_lines.to_string();
    if !pass_runner(&["--workspace"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        HYPERVISOR_EFI_REAL_HW_FEATURE,
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["-p", "hv-hypervisor-boot", "--features", "vmx-launch"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-x86-cpu",
        "--features",
        "execute-instructions,std,firmware-live-execution,vmx-launch",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-foundation",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["-p", "hv-hypervisor-boot", "--features", "datapath-live"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-malicious",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["-p", "hv-hypervisor-efi", "--features", "datapath-live"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-malicious",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["-p", "hv-hypervisor-boot", "--features", "datapath-guests"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["-p", "hv-hypervisor-efi", "--features", "datapath-guests"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-benchmark",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-benchmark",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["-p", "hv-hypervisor-boot", "--features", "datapath-runtime"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["-p", "hv-hypervisor-efi", "--features", "datapath-runtime"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["--build-guests"]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-guest-sources",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-guest-sources",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-guest-live",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-guest-live",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-guest-execution",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-guest-execution",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-guest-throughput",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-guest-throughput",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-guest-relay-live",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !test_pass_runner(&[
        "-p",
        "hv-hypervisor-boot",
        "--features",
        "datapath-guest-relay-measurement",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-guest-relay-live",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !test_pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-guest-relay-measurement",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&[
        "-p",
        "hv-hypervisor-efi",
        "--features",
        "datapath-foundation",
    ]) {
        return Ok((String::new(), String::new(), false));
    }
    if !pass_runner(&["-p", "hv-hypervisor-efi", "--features", "vmx-launch"]) {
        return Ok((String::new(), String::new(), false));
    }

    command.args([
        "llvm-cov",
        "report",
        "--summary-only",
        "--fail-under-lines",
        threshold.as_str(),
    ]);
    let status = command.output().map_err(|err| {
        eprintln!("failed to run coverage report: {err}");
        1
    })?;

    Ok((
        String::from_utf8_lossy(&status.stdout).into_owned(),
        String::from_utf8_lossy(&status.stderr).into_owned(),
        status.status.success(),
    ))
}

fn test_command(runner: fn(&str, &[&str]) -> i32) -> i32 {
    runner("cargo", &["test", "--workspace"])
}

fn parse_summary_line_coverage(summary: &str) -> Option<f64> {
    for line in summary.lines() {
        if line.starts_with("TOTAL") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                return parts
                    .get(9)
                    .and_then(|value| value.trim_end_matches('%').parse().ok());
            }
        }
    }
    None
}

/// Dispatches a parsed task to the appropriate handler.
pub fn dispatch_task(task: TaskCommand) -> i32 {
    dispatch_task_with(task, run)
}

fn dispatch_task_with(task: TaskCommand, runner: fn(&str, &[&str]) -> i32) -> i32 {
    match task {
        TaskCommand::Test => test_command(runner),
        TaskCommand::Build => runner("cargo", &["build", "--workspace"]),
        TaskCommand::Coverage { min_lines } => coverage_command(min_lines),
        TaskCommand::Fuzz { runs } => fuzz_command(runs, run_with_cxx_gpp),
        TaskCommand::BuildEfi { config, output } => run_build_efi(&config, &output),
        TaskCommand::BuildHypervisorEfi { config, output } => {
            run_build_hypervisor_efi(&config, &output)
        }
        TaskCommand::BuildBootChain { config, output_dir } => {
            run_build_boot_chain(&config, &output_dir)
        }
        TaskCommand::BuildBootChainLive { config, output_dir } => {
            run_build_boot_chain_live(&config, &output_dir)
        }
        TaskCommand::OvmfSmokeBoot {
            config,
            boot_chain_dir,
            timeout_secs,
            build,
        } => run_ovmf_smoke_boot(&config, &boot_chain_dir, timeout_secs, build),
        TaskCommand::LiveQemuSmoke {
            config,
            boot_chain_dir,
            timeout_secs,
            build,
            require_executed,
            no_skip,
            no_host_net,
        } => run_live_qemu_smoke_with_options(
            &config,
            &boot_chain_dir,
            timeout_secs,
            build,
            &live_qemu_smoke::LiveQemuSmokeOptions {
                require_executed,
                no_skip,
                no_host_net,
            },
        ),
        TaskCommand::DatapathBenchmark { config } => {
            datapath_benchmark::run_datapath_benchmark(&config)
        }
        TaskCommand::BuildGuests => build_guests::run_build_guests(),
        TaskCommand::DatapathLiveBenchmark { config } => {
            if build_guests::run_build_guests() != 0 {
                return 1;
            }
            datapath_benchmark::run_datapath_benchmark(&config)
        }
        TaskCommand::ConfigValidate { path } => run_config_validate(&path),
        TaskCommand::ConfigGenerate { path, output } => run_config_generate(&path, &output),
        TaskCommand::SetupHostNetTaps { config } => run_setup_host_net_taps(&config),
    }
}

#[derive(Parser, Debug)]
#[command(name = "xtask", about = "Static hypervisor developer tasks")]
struct Cli {
    #[command(subcommand)]
    command: TaskCommandCli,
}

#[derive(Subcommand, Debug)]
enum TaskCommandCli {
    /// Run host unit tests.
    Test,
    /// Build all workspace crates.
    Build,
    /// Run tests and enforce minimum line coverage.
    Coverage {
        /// Minimum required line coverage percentage.
        #[arg(long, default_value_t = DEFAULT_COVERAGE_MIN_LINES)]
        min_lines: u8,
    },
    /// Validate a configuration file.
    Config {
        #[command(subcommand)]
        action: ConfigActionCli,
    },
    /// Run libFuzzer smoke tests for parsing code.
    Fuzz {
        /// Number of libFuzzer iterations per target.
        #[arg(long, default_value_t = DEFAULT_FUZZ_RUNS)]
        runs: u32,
    },
    /// Build the UEFI loader `.efi` image.
    BuildEfi {
        /// Path to YAML configuration used to embed the digest.
        #[arg(long, default_value = DEFAULT_EFI_CONFIG_PATH)]
        config: String,
        /// Output path for the built `.efi` file.
        #[arg(long, default_value = DEFAULT_EFI_OUTPUT_PATH)]
        output: String,
    },
    /// Build the UEFI hypervisor `.efi` image.
    BuildHypervisorEfi {
        /// Path to YAML configuration used to embed requirements metadata.
        #[arg(long, default_value = DEFAULT_EFI_CONFIG_PATH)]
        config: String,
        /// Output path for the built `.efi` file.
        #[arg(long, default_value = DEFAULT_HYPERVISOR_EFI_OUTPUT_PATH)]
        output: String,
    },
    /// Build loader and hypervisor `.efi` images for OVMF boot-chain testing.
    BuildBootChain {
        /// Path to YAML configuration used to embed artifacts.
        #[arg(long, default_value = DEFAULT_EFI_CONFIG_PATH)]
        config: String,
        /// Output directory for boot-chain images.
        #[arg(long, default_value = DEFAULT_BOOT_CHAIN_OUTPUT_DIR)]
        output_dir: String,
    },
    /// Build loader and REAL_HW hypervisor `.efi` images for KVM live smoke testing.
    BuildBootChainLive {
        /// Path to YAML configuration used to embed artifacts.
        #[arg(long, default_value = DEFAULT_EFI_CONFIG_PATH)]
        config: String,
        /// Output directory for boot-chain images.
        #[arg(long, default_value = DEFAULT_LIVE_BOOT_CHAIN_OUTPUT_DIR)]
        output_dir: String,
    },
    /// Boot the loader + hypervisor chain under OVMF/QEMU and verify serial output.
    OvmfSmokeBoot {
        /// Path to YAML configuration used when `--build` is set.
        #[arg(long, default_value = DEFAULT_OVMF_SMOKE_CONFIG_PATH)]
        config: String,
        /// Directory containing `hv-loader.efi` and `hv-hypervisor.efi`.
        #[arg(long, default_value = DEFAULT_BOOT_CHAIN_OUTPUT_DIR)]
        boot_chain_dir: String,
        /// Maximum seconds to wait for OVMF/QEMU before evaluating the serial log.
        #[arg(long, default_value_t = DEFAULT_OVMF_SMOKE_TIMEOUT_SECS)]
        timeout_secs: u64,
        /// Skip rebuilding the boot chain (requires existing `.efi` images).
        #[arg(long, default_value_t = false)]
        no_build: bool,
    },
    /// Boot the loader + REAL_HW hypervisor chain under KVM/QEMU and verify serial output.
    LiveQemuSmoke {
        /// Path to YAML configuration used when `--build` is set.
        #[arg(long, default_value = DEFAULT_EFI_CONFIG_PATH)]
        config: String,
        /// Directory containing `hv-loader.efi` and `hv-hypervisor.efi`.
        #[arg(long, default_value = DEFAULT_LIVE_BOOT_CHAIN_OUTPUT_DIR)]
        boot_chain_dir: String,
        /// Maximum seconds to wait for KVM/QEMU before evaluating the serial log.
        #[arg(long, default_value_t = DEFAULT_LIVE_QEMU_SMOKE_TIMEOUT_SECS)]
        timeout_secs: u64,
        /// Skip rebuilding the REAL_HW boot chain (requires existing `.efi` images).
        #[arg(long, default_value_t = false)]
        no_build: bool,
        /// Require in-VM executed throughput and guest relay markers (reject validate-only proof).
        #[arg(long, default_value_t = false)]
        require_executed: bool,
        /// Fail instead of skipping when KVM/VMX or the OVMF/KVM serial probe is unavailable.
        #[arg(long, default_value_t = false)]
        no_skip: bool,
        /// Disable host-connected outer QEMU e1000/netdev wiring even when enabled in config.
        #[arg(long, default_value_t = false)]
        no_host_net: bool,
    },
    /// Create and bring up host tap interfaces from the config host network plan.
    SetupHostNetTaps {
        /// Path to YAML configuration describing host network taps.
        #[arg(long, default_value = DEFAULT_EFI_CONFIG_PATH)]
        config: String,
    },
    /// Run the host datapath throughput benchmark per docs/benchmark.md.
    DatapathBenchmark {
        /// Path to YAML configuration used to plan the reference datapath.
        #[arg(long, default_value = DEFAULT_EFI_CONFIG_PATH)]
        config: String,
    },
    /// Build real datapath guest ELFs from `guests/` source trees.
    BuildGuests,
    /// Build guests then run the host wall-clock datapath benchmark.
    DatapathLiveBenchmark {
        /// Path to YAML configuration used to plan the reference datapath.
        #[arg(long, default_value = DEFAULT_EFI_CONFIG_PATH)]
        config: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigActionCli {
    /// Validate configuration semantics and syntax.
    Validate {
        /// Path to YAML configuration.
        path: String,
    },
    /// Generate configuration artifacts.
    Generate {
        /// Path to YAML configuration.
        path: String,
        /// Output directory.
        #[arg(short, long, default_value = DEFAULT_CONFIG_OUTPUT_DIR)]
        output: String,
    },
}

/// Maps CLI subcommands to library dispatch values.
pub(crate) fn map_cli_command(command: TaskCommandCli) -> TaskCommand {
    match command {
        TaskCommandCli::Test => TaskCommand::Test,
        TaskCommandCli::Build => TaskCommand::Build,
        TaskCommandCli::Coverage { min_lines } => TaskCommand::Coverage { min_lines },
        TaskCommandCli::Fuzz { runs } => TaskCommand::Fuzz { runs },
        TaskCommandCli::BuildEfi { config, output } => TaskCommand::BuildEfi { config, output },
        TaskCommandCli::BuildHypervisorEfi { config, output } => {
            TaskCommand::BuildHypervisorEfi { config, output }
        }
        TaskCommandCli::BuildBootChain { config, output_dir } => {
            TaskCommand::BuildBootChain { config, output_dir }
        }
        TaskCommandCli::BuildBootChainLive { config, output_dir } => {
            TaskCommand::BuildBootChainLive { config, output_dir }
        }
        TaskCommandCli::OvmfSmokeBoot {
            config,
            boot_chain_dir,
            timeout_secs,
            no_build,
        } => TaskCommand::OvmfSmokeBoot {
            config,
            boot_chain_dir,
            timeout_secs,
            build: !no_build,
        },
        TaskCommandCli::LiveQemuSmoke {
            config,
            boot_chain_dir,
            timeout_secs,
            no_build,
            require_executed,
            no_skip,
            no_host_net,
        } => TaskCommand::LiveQemuSmoke {
            config,
            boot_chain_dir,
            timeout_secs,
            build: !no_build,
            require_executed,
            no_skip,
            no_host_net,
        },
        TaskCommandCli::SetupHostNetTaps { config } => TaskCommand::SetupHostNetTaps { config },
        TaskCommandCli::DatapathBenchmark { config } => TaskCommand::DatapathBenchmark { config },
        TaskCommandCli::BuildGuests => TaskCommand::BuildGuests,
        TaskCommandCli::DatapathLiveBenchmark { config } => {
            TaskCommand::DatapathLiveBenchmark { config }
        }
        TaskCommandCli::Config { action } => match action {
            ConfigActionCli::Validate { path } => TaskCommand::ConfigValidate { path },
            ConfigActionCli::Generate { path, output } => {
                TaskCommand::ConfigGenerate { path, output }
            }
        },
    }
}

/// Parses CLI arguments into a [`TaskCommand`].
pub fn parse_task_command<I, T>(args: I) -> Result<TaskCommand, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    Ok(map_cli_command(cli.command))
}

/// Parsed xtask subcommands used by the CLI and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskCommand {
    /// Run host unit tests.
    Test,
    /// Build all workspace crates.
    Build,
    /// Run tests and enforce minimum line coverage.
    Coverage {
        /// Minimum required line coverage percentage.
        min_lines: u8,
    },
    /// Run libFuzzer smoke tests for parsing code.
    Fuzz {
        /// Number of libFuzzer iterations per target.
        runs: u32,
    },
    /// Build the UEFI loader `.efi` image.
    BuildEfi {
        /// Path to YAML configuration used to embed the digest.
        config: String,
        /// Output path for the built `.efi` file.
        output: String,
    },
    /// Build the UEFI hypervisor `.efi` image.
    BuildHypervisorEfi {
        /// Path to YAML configuration used to embed requirements metadata.
        config: String,
        /// Output path for the built `.efi` file.
        output: String,
    },
    /// Build loader and hypervisor `.efi` images for OVMF boot-chain testing.
    BuildBootChain {
        /// Path to YAML configuration used to embed artifacts.
        config: String,
        /// Output directory for boot-chain images.
        output_dir: String,
    },
    /// Build loader and REAL_HW hypervisor `.efi` images for KVM live smoke testing.
    BuildBootChainLive {
        /// Path to YAML configuration used to embed artifacts.
        config: String,
        /// Output directory for boot-chain images.
        output_dir: String,
    },
    /// Boot the loader + hypervisor chain under OVMF/QEMU and verify serial output.
    OvmfSmokeBoot {
        /// Path to YAML configuration used when building the boot chain.
        config: String,
        /// Directory containing `hv-loader.efi` and `hv-hypervisor.efi`.
        boot_chain_dir: String,
        /// Maximum seconds to wait for OVMF/QEMU before evaluating the serial log.
        timeout_secs: u64,
        /// Build the boot chain before launching QEMU.
        build: bool,
    },
    /// Boot the loader + REAL_HW hypervisor chain under KVM/QEMU and verify serial output.
    LiveQemuSmoke {
        /// Path to YAML configuration used when building the boot chain.
        config: String,
        /// Directory containing `hv-loader.efi` and `hv-hypervisor.efi`.
        boot_chain_dir: String,
        /// Maximum seconds to wait for KVM/QEMU before evaluating the serial log.
        timeout_secs: u64,
        /// Build the REAL_HW boot chain before launching QEMU.
        build: bool,
        /// Require in-VM executed throughput and guest relay markers (reject validate-only proof).
        require_executed: bool,
        /// Fail instead of skipping when KVM/VMX or the OVMF/KVM serial probe is unavailable.
        no_skip: bool,
        /// Disable host-connected outer QEMU e1000/netdev wiring even when enabled in config.
        no_host_net: bool,
    },
    /// Create and bring up host tap interfaces from the config host network plan.
    SetupHostNetTaps {
        /// Path to YAML configuration describing host network taps.
        config: String,
    },
    /// Run the host datapath throughput benchmark per docs/benchmark.md.
    DatapathBenchmark {
        /// Path to YAML configuration used to plan the reference datapath.
        config: String,
    },
    /// Build real datapath guest ELFs from `guests/` source trees.
    BuildGuests,
    /// Build guests then run the host wall-clock datapath benchmark.
    DatapathLiveBenchmark {
        /// Path to YAML configuration used to plan the reference datapath.
        config: String,
    },
    /// Validate a configuration file.
    ConfigValidate {
        /// Path to YAML configuration.
        path: String,
    },
    /// Generate configuration artifacts.
    ConfigGenerate {
        /// Path to YAML configuration.
        path: String,
        /// Output directory.
        output: String,
    },
}

/// Executes a subprocess and maps its exit status to an integer code.
pub fn run(program: &str, args: &[&str]) -> i32 {
    let status = ProcessCommand::new(program).args(args).status();
    match status {
        Ok(status) => {
            if status.success() {
                0
            } else {
                match status.code() {
                    Some(code) => code,
                    None => 1,
                }
            }
        }
        Err(err) => {
            eprintln!("failed to run {program}: {err}");
            1
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use crate::constants::{
        DEFAULT_BOOT_CHAIN_OUTPUT_DIR, DEFAULT_COVERAGE_MIN_LINES, DEFAULT_EFI_CONFIG_PATH,
        DEFAULT_EFI_OUTPUT_PATH, DEFAULT_FUZZ_RUNS, DEFAULT_HYPERVISOR_EFI_OUTPUT_PATH,
        DEFAULT_OVMF_SMOKE_TIMEOUT_SECS,
    };
    use hv_config::constants::DEFAULT_CONFIG_OUTPUT_DIR;

    fn mock_test_runner(program: &str, args: &[&str]) -> i32 {
        assert_eq!(program, "cargo");
        assert_eq!(args, &["test", "--workspace"]);
        0
    }

    fn enforce_line_coverage(min_lines: u8, summary: &str) -> i32 {
        evaluate_coverage_output(min_lines, summary, true)
    }

    #[test]
    fn evaluate_coverage_output_accepts_passing_summary() {
        let summary = "TOTAL  2018  290  85.63%  457  55  87.96%  4088  204  95.01%  0  0  -";
        assert_eq!(evaluate_coverage_output(95, summary, true), 0);
    }

    #[test]
    fn evaluate_coverage_output_rejects_low_coverage_or_bad_summary() {
        let summary = "TOTAL  2018  290  85.63%  457  55  87.96%  4088  204  95.01%  0  0  -";
        assert_eq!(evaluate_coverage_output(96, summary, true), 1);
        assert_eq!(evaluate_coverage_output(95, "missing total row", true), 1);
        assert_eq!(evaluate_coverage_output(95, summary, false), 1);
    }

    #[test]
    fn run_fuzz_delegates_to_fuzz_command() {
        assert_ne!(run_fuzz(1), 0);
    }

    #[test]
    fn build_efi_image_with_mock_runner_succeeds() {
        let workspace = workspace_root();
        assert_eq!(
            build_efi_image_with(&workspace, "/tmp/config.sha256", |_| {
                run_command(ProcessCommand::new("true"))
            }),
            0
        );
    }

    #[test]
    fn parse_summary_line_coverage_reads_total_row() {
        let summary = "TOTAL  2018  290  85.63%  457  55  87.96%  4088  204  95.01%  0  0  -";
        assert_eq!(parse_summary_line_coverage(summary), Some(95.01));
    }

    #[test]
    fn parse_summary_line_coverage_rejects_missing_total() {
        assert_eq!(parse_summary_line_coverage("no summary here"), None);
    }

    #[test]
    fn enforce_line_coverage_rejects_and_accepts_threshold() {
        let summary = "TOTAL  2018  290  85.63%  457  55  87.96%  4088  204  95.01%  0  0  -";
        assert_eq!(enforce_line_coverage(95, summary), 0);
        assert_eq!(enforce_line_coverage(96, summary), 1);
        assert_eq!(enforce_line_coverage(95, "no summary"), 1);
    }

    use std::sync::atomic::{AtomicU8, Ordering};

    static FUZZ_BUILD_CALLS: AtomicU8 = AtomicU8::new(0);
    static FUZZ_TARGET_CALLS: AtomicU8 = AtomicU8::new(0);
    static FUZZ_FAIL_TARGETS: AtomicU8 = AtomicU8::new(0);

    fn mock_fuzz_runner(program: &str, args: &[&str]) -> i32 {
        if program == "cargo" {
            FUZZ_BUILD_CALLS.fetch_add(1, Ordering::SeqCst);
            return 0;
        }
        FUZZ_TARGET_CALLS.fetch_add(1, Ordering::SeqCst);
        assert!(args.iter().any(|arg| *arg == "-runs=7"));
        0
    }

    fn mock_fuzz_runner_fail_targets(program: &str, _args: &[&str]) -> i32 {
        if program == "cargo" {
            0
        } else {
            FUZZ_FAIL_TARGETS.fetch_add(1, Ordering::SeqCst);
            1
        }
    }

    #[test]
    fn fuzz_command_runs_all_targets_with_mock_runner() {
        FUZZ_BUILD_CALLS.store(0, Ordering::SeqCst);
        FUZZ_TARGET_CALLS.store(0, Ordering::SeqCst);
        assert_eq!(fuzz_command(7, mock_fuzz_runner), 0);
        assert_eq!(FUZZ_BUILD_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(
            usize::from(FUZZ_TARGET_CALLS.load(Ordering::SeqCst)),
            constants::FUZZ_TARGETS.len()
        );
    }

    #[test]
    fn fuzz_command_stops_when_build_fails() {
        assert_ne!(fuzz_command(1, |_, _| 1), 0);
    }

    #[test]
    fn fuzz_command_stops_when_target_fails() {
        FUZZ_FAIL_TARGETS.store(0, Ordering::SeqCst);
        assert_ne!(fuzz_command(1, mock_fuzz_runner_fail_targets), 0);
        assert_eq!(FUZZ_FAIL_TARGETS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn spawn_llvm_cov_summary_with_mock_command_reads_output() {
        fn mock_pass(args: &[&str]) -> bool {
            let _ = args;
            true
        }
        let mut command = ProcessCommand::new("sh");
        command
            .arg("-c")
            .arg("echo 'TOTAL  2018  290  85.63%  457  55  87.96%  4088  204  95.01%  0  0  -'");
        let (stdout, stderr, success) =
            spawn_llvm_cov_summary_with(95, &mut command, mock_pass, mock_pass).expect("spawn");
        assert!(stdout.contains("TOTAL"));
        assert!(stderr.is_empty());
        assert!(success);
    }

    #[test]
    fn run_with_cxx_gpp_reports_command_failures() {
        assert_eq!(run_with_cxx_gpp("true", &[]), 0);
        assert_ne!(run_with_cxx_gpp("false", &[]), 0);
        assert_ne!(run_with_cxx_gpp("/no/such/binary", &[]), 0);
    }

    #[test]
    fn run_tests_invokes_cargo_test_workspace() {
        assert_eq!(run_tests_with(mock_test_runner), 0);
    }

    #[test]
    fn run_tests_with_delegates_to_test_command() {
        assert_eq!(run_tests_with(|_, _| 0), 0);
    }

    #[test]
    fn run_build_boot_chain_live_with_mock_pipeline_succeeds() {
        let workspace = workspace_root();
        let output_dir = workspace.join("build/mock-live-boot-chain");
        let _ = std::fs::remove_dir_all(&output_dir);
        let write_loader = |workspace: &std::path::Path, _digest: &str| {
            let source = workspace
                .join("crates/hv-loader-efi-bin/target/x86_64-unknown-uefi/release/hv-loader.efi");
            std::fs::create_dir_all(source.parent().expect("parent")).expect("dir");
            std::fs::write(&source, b"mock-loader").expect("write");
            0
        };
        let write_hypervisor = |workspace: &std::path::Path, _digest: &str, _config: &str| {
            let source = workspace.join(
                "crates/hv-hypervisor-efi-bin/target/x86_64-unknown-uefi/release/hv-hypervisor.efi",
            );
            std::fs::create_dir_all(source.parent().expect("parent")).expect("dir");
            std::fs::write(&source, b"mock-hypervisor").expect("write");
            0
        };
        assert_eq!(
            run_build_boot_chain_live_with(
                "configs/qemu.yaml",
                "build/mock-live-boot-chain",
                |_, _| 0,
                |_, _| 0,
                write_loader,
                write_hypervisor,
            ),
            0
        );
        assert!(output_dir.join("hv-loader.efi").is_file());
        assert!(output_dir.join("hv-hypervisor.efi").is_file());
    }

    #[test]
    fn hypervisor_efi_build_command_passes_real_hw_features() {
        let workspace = workspace_root();
        let command = hypervisor_efi_build_command(
            &workspace,
            "/tmp/config.sha256",
            "/tmp/config.yaml",
            &[HYPERVISOR_EFI_REAL_HW_FEATURE],
        );
        let args: Vec<_> = command.get_args().collect();
        assert!(args.iter().any(|arg| *arg == "--features"));
        assert!(args.iter().any(|arg| {
            arg.to_string_lossy()
                .contains(HYPERVISOR_EFI_REAL_HW_FEATURE)
        }));
    }

    #[test]
    fn run_build_hypervisor_efi_delegates_to_pipeline() {
        assert_ne!(
            run_build_hypervisor_efi("/no/such/config.yaml", "build/out.efi"),
            0
        );
    }

    #[test]
    fn run_build_efi_delegates_to_pipeline() {
        assert_ne!(run_build_efi("/no/such/config.yaml", "build/out.efi"), 0);
    }

    #[test]
    fn run_tests_with_mock_runner() {
        assert_eq!(run_tests_with(|_, _| 0), 0);
    }

    #[test]
    fn run_build_boot_chain_delegates_to_pipeline() {
        assert_ne!(run_build_boot_chain("/no/such/config.yaml", "build/out"), 0);
    }

    #[test]
    fn run_build_boot_chain_live_delegates_to_pipeline() {
        assert_ne!(
            run_build_boot_chain_live("/no/such/config.yaml", "build/out"),
            0
        );
    }

    #[test]
    fn run_build_hypervisor_efi_with_mock_pipeline_succeeds() {
        let workspace = workspace_root();
        let output = workspace.join("build/mock-hypervisor.efi");
        let _ = std::fs::remove_file(&output);
        let write_hypervisor = |workspace: &std::path::Path, _digest: &str, _config: &str| {
            let source = workspace.join(
                "crates/hv-hypervisor-efi-bin/target/x86_64-unknown-uefi/release/hv-hypervisor.efi",
            );
            std::fs::create_dir_all(source.parent().expect("parent")).expect("dir");
            std::fs::write(&source, b"mock-hypervisor").expect("write");
            0
        };
        assert_eq!(
            run_build_hypervisor_efi_with(
                "configs/qemu.yaml",
                "build/mock-hypervisor.efi",
                |_, _| 0,
                |_, _| 0,
                write_hypervisor,
            ),
            0
        );
        assert!(output.is_file());
    }

    #[test]
    fn run_live_qemu_smoke_and_ovmf_wrappers_are_callable() {
        let _ = run_ovmf_smoke_boot("configs/qemu.yaml", "build/missing-ovmf-chain", 1, false);
        let _ = run_live_qemu_smoke("configs/qemu.yaml", "build/missing-live-chain", 1, false);
    }

    #[test]
    fn run_coverage_delegates_to_coverage_command() {
        fn mock_spawn(_min_lines: u8) -> Result<(String, String, bool), i32> {
            Ok((
                String::from(
                    "TOTAL  2018  290  85.63%  457  55  87.96%  4088  204  95.01%  0  0  -",
                ),
                String::new(),
                true,
            ))
        }
        assert_eq!(coverage_command_with(95, mock_spawn), 0);
    }

    #[test]
    fn run_command_reports_spawn_failure() {
        assert_eq!(run_command(ProcessCommand::new("/no/such/binary")), 1);
    }

    fn mock_build_runner(program: &str, args: &[&str]) -> i32 {
        assert_eq!(program, "cargo");
        assert_eq!(args, &["build", "--workspace"]);
        0
    }

    #[test]
    fn coverage_command_with_mock_spawn_composes_evaluation() {
        fn mock_spawn(_min_lines: u8) -> Result<(String, String, bool), i32> {
            Ok((
                String::from(
                    "TOTAL  2018  290  85.63%  457  55  87.96%  4088  204  95.01%  0  0  -",
                ),
                String::new(),
                true,
            ))
        }
        assert_eq!(coverage_command_with(95, mock_spawn), 0);
        fn mock_spawn_fail(_min_lines: u8) -> Result<(String, String, bool), i32> {
            Err(9)
        }
        assert_eq!(coverage_command_with(95, mock_spawn_fail), 9);
    }

    #[test]
    fn dispatch_task_routes_live_boot_chain_and_smoke_commands() {
        assert_ne!(
            dispatch_task(TaskCommand::BuildBootChainLive {
                config: "/no/such/config.yaml".into(),
                output_dir: "build/out".into(),
            }),
            0
        );
        let smoke_status = dispatch_task(TaskCommand::LiveQemuSmoke {
            config: String::from(DEFAULT_EFI_CONFIG_PATH),
            boot_chain_dir: String::from("build/missing-live-chain-dispatch"),
            timeout_secs: 1,
            build: false,
            require_executed: false,
            no_skip: false,
            no_host_net: false,
        });
        assert_eq!(
            smoke_status,
            if super::live_qemu_smoke::live_qemu_hardware_ready() {
                1
            } else {
                0
            }
        );
    }

    #[test]
    fn dispatch_task_routes_test_build_and_coverage() {
        assert_eq!(dispatch_task_with(TaskCommand::Test, mock_test_runner), 0);
        assert_eq!(dispatch_task_with(TaskCommand::Build, mock_build_runner), 0);
    }

    #[test]
    fn dispatch_task_routes_fuzz() {
        assert_ne!(dispatch_task(TaskCommand::Fuzz { runs: 1 }), 0);
    }

    #[test]
    fn dispatch_task_routes_config_commands() {
        let path = format!("{}/../configs/qemu.yaml", env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            dispatch_task(TaskCommand::ConfigValidate { path: path.clone() }),
            0
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let output = dir.path().to_string_lossy().to_string();
        assert_eq!(
            dispatch_task(TaskCommand::ConfigGenerate { path, output }),
            0
        );
    }

    #[test]
    fn parse_task_command_covers_all_subcommands() {
        assert_eq!(
            parse_task_command(["xtask", "test"]).expect("parse test"),
            TaskCommand::Test
        );
        assert_eq!(
            parse_task_command(["xtask", "build"]).expect("parse build"),
            TaskCommand::Build
        );
        assert_eq!(
            parse_task_command([
                "xtask",
                "coverage",
                "--min-lines",
                &DEFAULT_COVERAGE_MIN_LINES.to_string(),
            ])
            .expect("parse coverage"),
            TaskCommand::Coverage {
                min_lines: DEFAULT_COVERAGE_MIN_LINES,
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "config", "validate", "cfg.yaml"])
                .expect("parse validate"),
            TaskCommand::ConfigValidate {
                path: String::from("cfg.yaml")
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "config", "generate", "cfg.yaml"])
                .expect("parse generate"),
            TaskCommand::ConfigGenerate {
                path: String::from("cfg.yaml"),
                output: String::from(DEFAULT_CONFIG_OUTPUT_DIR),
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "fuzz"]).expect("parse fuzz"),
            TaskCommand::Fuzz {
                runs: DEFAULT_FUZZ_RUNS,
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "fuzz", "--runs", "1024"]).expect("parse fuzz runs"),
            TaskCommand::Fuzz { runs: 1024 }
        );
        assert_eq!(
            parse_task_command(["xtask", "build-efi"]).expect("parse build efi"),
            TaskCommand::BuildEfi {
                config: String::from(DEFAULT_EFI_CONFIG_PATH),
                output: String::from(DEFAULT_EFI_OUTPUT_PATH),
            }
        );
        assert_eq!(
            parse_task_command([
                "xtask",
                "build-efi",
                "--config",
                "cfg.yaml",
                "--output",
                "out.efi",
            ])
            .expect("parse build efi args"),
            TaskCommand::BuildEfi {
                config: String::from("cfg.yaml"),
                output: String::from("out.efi"),
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "build-hypervisor-efi"]).expect("parse build hypervisor"),
            TaskCommand::BuildHypervisorEfi {
                config: String::from(DEFAULT_EFI_CONFIG_PATH),
                output: String::from(DEFAULT_HYPERVISOR_EFI_OUTPUT_PATH),
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "build-boot-chain"]).expect("parse boot chain"),
            TaskCommand::BuildBootChain {
                config: String::from(DEFAULT_EFI_CONFIG_PATH),
                output_dir: String::from(DEFAULT_BOOT_CHAIN_OUTPUT_DIR),
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "ovmf-smoke-boot"]).expect("parse ovmf smoke boot"),
            TaskCommand::OvmfSmokeBoot {
                config: String::from(DEFAULT_OVMF_SMOKE_CONFIG_PATH),
                boot_chain_dir: String::from(DEFAULT_BOOT_CHAIN_OUTPUT_DIR),
                timeout_secs: DEFAULT_OVMF_SMOKE_TIMEOUT_SECS,
                build: true,
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "ovmf-smoke-boot", "--no-build"]).expect("parse no build"),
            TaskCommand::OvmfSmokeBoot {
                config: String::from(DEFAULT_OVMF_SMOKE_CONFIG_PATH),
                boot_chain_dir: String::from(DEFAULT_BOOT_CHAIN_OUTPUT_DIR),
                timeout_secs: DEFAULT_OVMF_SMOKE_TIMEOUT_SECS,
                build: false,
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "build-boot-chain-live"]).expect("parse live boot chain"),
            TaskCommand::BuildBootChainLive {
                config: String::from(DEFAULT_EFI_CONFIG_PATH),
                output_dir: String::from(DEFAULT_LIVE_BOOT_CHAIN_OUTPUT_DIR),
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "live-qemu-smoke"]).expect("parse live qemu smoke"),
            TaskCommand::LiveQemuSmoke {
                config: String::from(DEFAULT_EFI_CONFIG_PATH),
                boot_chain_dir: String::from(DEFAULT_LIVE_BOOT_CHAIN_OUTPUT_DIR),
                timeout_secs: DEFAULT_LIVE_QEMU_SMOKE_TIMEOUT_SECS,
                build: true,
                require_executed: false,
                no_skip: false,
                no_host_net: false,
            }
        );
        assert_eq!(
            parse_task_command([
                "xtask",
                "live-qemu-smoke",
                "--require-executed",
                "--no-skip",
            ])
            .expect("parse live qemu smoke strict"),
            TaskCommand::LiveQemuSmoke {
                config: String::from(DEFAULT_EFI_CONFIG_PATH),
                boot_chain_dir: String::from(DEFAULT_LIVE_BOOT_CHAIN_OUTPUT_DIR),
                timeout_secs: DEFAULT_LIVE_QEMU_SMOKE_TIMEOUT_SECS,
                build: true,
                require_executed: true,
                no_skip: true,
                no_host_net: false,
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "setup-host-net-taps"]).expect("parse setup taps"),
            TaskCommand::SetupHostNetTaps {
                config: String::from(DEFAULT_EFI_CONFIG_PATH),
            }
        );
    }

    #[test]
    fn hypervisor_efi_build_command_sets_release_uefi_manifest_and_env() {
        let workspace = workspace_root();
        let command =
            hypervisor_efi_build_command(&workspace, "/tmp/config.sha256", "/tmp/config.yaml", &[]);
        assert_eq!(command.get_program(), "cargo");
        let args: Vec<_> = command.get_args().collect();
        assert!(args.iter().any(|arg| *arg == "--release"));
        assert!(args
            .iter()
            .any(|arg| *arg == "crates/hv-hypervisor-efi-bin/Cargo.toml"));
        let env_keys: Vec<_> = command
            .get_envs()
            .map(|(key, _)| key.to_string_lossy())
            .collect();
        assert!(env_keys
            .iter()
            .any(|key| key == "HV_HYPERVISOR_EMBEDDED_CONFIG_PATH"));
    }

    #[test]
    fn run_build_boot_chain_with_propagates_loader_build_failure() {
        assert_eq!(
            run_build_boot_chain_with(
                "configs/qemu.yaml",
                "build/mock-boot-chain-fail",
                |_, _| 0,
                |_, _| 0,
                |_, _| 1,
                |_, _, _| 0,
            ),
            1
        );
    }

    #[test]
    fn run_build_boot_chain_with_propagates_hypervisor_build_failure() {
        assert_eq!(
            run_build_boot_chain_with(
                "configs/qemu.yaml",
                "build/mock-boot-chain-fail-hv",
                |_, _| 0,
                |_, _| 0,
                |_, _| 0,
                |_, _, _| 1,
            ),
            1
        );
    }

    #[test]
    fn run_build_boot_chain_live_with_propagates_loader_build_failure() {
        assert_eq!(
            run_build_boot_chain_live_with(
                "configs/qemu.yaml",
                "build/mock-live-boot-chain-fail",
                |_, _| 0,
                |_, _| 0,
                |_, _| 1,
                |_, _, _| 0,
            ),
            1
        );
    }

    #[test]
    fn copy_hypervisor_efi_artifact_succeeds_when_source_exists() {
        let workspace = workspace_root();
        let source = workspace.join(
            "crates/hv-hypervisor-efi-bin/target/x86_64-unknown-uefi/release/hv-hypervisor.efi",
        );
        std::fs::create_dir_all(source.parent().expect("parent")).expect("dir");
        std::fs::write(&source, b"mock-hypervisor").expect("write");
        let output = workspace.join("build/mock-copy-hypervisor.efi");
        let _ = std::fs::remove_file(&output);
        std::fs::create_dir_all(output.parent().expect("parent")).expect("dir");
        assert_eq!(
            copy_hypervisor_efi_artifact(output.to_str().expect("path")),
            0
        );
        assert!(output.is_file());
    }

    #[test]
    fn run_build_boot_chain_with_mock_pipeline_succeeds() {
        let workspace = workspace_root();
        let output_dir = workspace.join("build/mock-boot-chain");
        let _ = std::fs::remove_dir_all(&output_dir);
        let write_loader = |workspace: &std::path::Path, _digest: &str| {
            let source = workspace
                .join("crates/hv-loader-efi-bin/target/x86_64-unknown-uefi/release/hv-loader.efi");
            std::fs::create_dir_all(source.parent().expect("parent")).expect("dir");
            std::fs::write(&source, b"mock-loader").expect("write");
            0
        };
        let write_hypervisor = |workspace: &std::path::Path, _digest: &str, _config: &str| {
            let source = workspace.join(
                "crates/hv-hypervisor-efi-bin/target/x86_64-unknown-uefi/release/hv-hypervisor.efi",
            );
            std::fs::create_dir_all(source.parent().expect("parent")).expect("dir");
            std::fs::write(&source, b"mock-hypervisor").expect("write");
            0
        };
        assert_eq!(
            run_build_boot_chain_with(
                "configs/qemu.yaml",
                "build/mock-boot-chain",
                |_, _| 0,
                |_, _| 0,
                write_loader,
                write_hypervisor,
            ),
            0
        );
        assert!(output_dir.join("hv-loader.efi").is_file());
        assert!(output_dir.join("hv-hypervisor.efi").is_file());
    }

    #[test]
    fn efi_build_command_sets_release_uefi_manifest_and_digest_env() {
        let workspace = workspace_root();
        let command = efi_build_command(&workspace, "/tmp/config.sha256");
        assert_eq!(command.get_program(), "cargo");
        let args: Vec<_> = command.get_args().collect();
        assert!(args.iter().any(|arg| *arg == "--release"));
        assert!(args
            .iter()
            .any(|arg| *arg == "crates/hv-loader-efi-bin/Cargo.toml"));
        assert!(args.iter().any(|arg| *arg == "x86_64-unknown-uefi"));
        let env_keys: Vec<_> = command
            .get_envs()
            .map(|(key, _)| key.to_string_lossy())
            .collect();
        assert!(env_keys.iter().any(|key| key == "HV_CONFIG_DIGEST_PATH"));
        assert!(env_keys.iter().any(|key| key == "CXX"));
    }

    #[test]
    fn run_command_reports_success_and_failure() {
        assert_eq!(run_command(ProcessCommand::new("true")), 0);
        assert_ne!(run_command(ProcessCommand::new("false")), 0);
    }

    #[test]
    fn run_build_efi_with_mock_pipeline_covers_branches() {
        let workspace = workspace_root();
        let output = workspace.join("build/mock-pipeline.efi");
        let _ = std::fs::remove_file(&output);

        assert_eq!(
            run_build_efi_with(
                "configs/qemu.yaml",
                "build/mock-pipeline.efi",
                |_, _| 0,
                |_, _| 0,
                |_, _| 0,
            ),
            0
        );
        assert!(output.is_file());

        assert_ne!(
            run_build_efi_with(
                "configs/qemu.yaml",
                "build/out.efi",
                |_, _| 1,
                |_, _| 0,
                |_, _| 0
            ),
            0
        );
        assert_ne!(
            run_build_efi_with(
                "configs/qemu.yaml",
                "build/out.efi",
                |_, _| 0,
                |_, _| 1,
                |_, _| 0
            ),
            0
        );
        assert_ne!(
            run_build_efi_with(
                "configs/qemu.yaml",
                "build/out.efi",
                |_, _| 0,
                |_, _| 0,
                |_, _| 1
            ),
            0
        );
    }

    #[test]
    fn dispatch_task_routes_build_efi() {
        assert_ne!(
            dispatch_task(TaskCommand::BuildEfi {
                config: "/no/such/config.yaml".into(),
                output: "build/out.efi".into(),
            }),
            0
        );
    }

    #[test]
    fn build_efi_image_runs_release_uefi_build() {
        assert_ne!(
            build_efi_image(&workspace_root(), "/no/such/config.sha256"),
            0,
            "missing digest should fail the UEFI build"
        );
    }

    #[test]
    fn run_build_efi_fails_for_missing_config() {
        assert_ne!(
            run_build_efi("/no/such/config.yaml", "build/missing.efi"),
            0
        );
    }

    #[test]
    fn copy_efi_artifact_reports_missing_destination() {
        assert_ne!(copy_efi_artifact("/no/such/dir/out.efi"), 0);
    }
}
