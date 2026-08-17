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

mod constants;

use clap::{Parser, Subcommand};
use constants::{DEFAULT_COVERAGE_MIN_LINES, DEFAULT_EFI_CONFIG_PATH, DEFAULT_EFI_OUTPUT_PATH, DEFAULT_FUZZ_RUNS};
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
        .unwrap_or_else(|_| build_dir.join("config.sha256").to_string_lossy().into_owned());

    if build(&workspace, &digest_path) != 0 {
        return 1;
    }

    copy_efi_artifact(output.to_str().unwrap_or(output_path))
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

fn workspace_root() -> std::path::PathBuf {
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
    let source = workspace_root().join(
        "crates/hv-loader-efi-bin/target/x86_64-unknown-uefi/release/hv-loader.efi",
    );
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
            eprintln!(
                "line coverage {coverage:.2}% is below minimum {min_lines}%"
            );
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

fn spawn_llvm_cov_summary(min_lines: u8) -> Result<(String, String, bool), i32> {
    let mut command = ProcessCommand::new("cargo");
    spawn_llvm_cov_summary_with(min_lines, &mut command)
}

fn spawn_llvm_cov_summary_with(
    min_lines: u8,
    command: &mut ProcessCommand,
) -> Result<(String, String, bool), i32> {
    let threshold = min_lines.to_string();
    command.args([
        "llvm-cov",
        "--workspace",
        "--summary-only",
        "--fail-under-lines",
        threshold.as_str(),
    ]);
    let status = command.output().map_err(|err| {
        eprintln!("failed to run coverage: {err}");
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
        TaskCommand::BuildEfi {
            config,
            output,
        } => run_build_efi(&config, &output),
        TaskCommand::ConfigValidate { path } => run_config_validate(&path),
        TaskCommand::ConfigGenerate { path, output } => run_config_generate(&path, &output),
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
        TaskCommandCli::Config { action } => match action {
            ConfigActionCli::Validate { path } => TaskCommand::ConfigValidate { path },
            ConfigActionCli::Generate { path, output } => TaskCommand::ConfigGenerate { path, output },
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
    use crate::constants::{DEFAULT_COVERAGE_MIN_LINES, DEFAULT_EFI_CONFIG_PATH, DEFAULT_EFI_OUTPUT_PATH, DEFAULT_FUZZ_RUNS};
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
        let mut command = ProcessCommand::new("sh");
        command.arg("-c").arg(
            "echo 'TOTAL  2018  290  85.63%  457  55  87.96%  4088  204  95.01%  0  0  -'",
        );
        let (stdout, stderr, success) =
            spawn_llvm_cov_summary_with(95, &mut command).expect("spawn");
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
    fn dispatch_task_routes_test_build_and_coverage() {
        assert_eq!(
            dispatch_task_with(TaskCommand::Test, mock_test_runner),
            0
        );
        assert_eq!(
            dispatch_task_with(TaskCommand::Build, mock_build_runner),
            0
        );
    }

    #[test]
    fn dispatch_task_routes_fuzz() {
        assert_ne!(dispatch_task(TaskCommand::Fuzz { runs: 1 }), 0);
    }

    #[test]
    fn dispatch_task_routes_config_commands() {
        let path = format!("{}/../configs/qemu.yaml", env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            dispatch_task(TaskCommand::ConfigValidate {
                path: path.clone()
            }),
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
            parse_task_command(["xtask", "config", "validate", "cfg.yaml"]).expect("parse validate"),
            TaskCommand::ConfigValidate {
                path: String::from("cfg.yaml")
            }
        );
        assert_eq!(
            parse_task_command(["xtask", "config", "generate", "cfg.yaml"]).expect("parse generate"),
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
        let env_keys: Vec<_> = command.get_envs().map(|(key, _)| key.to_string_lossy()).collect();
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
            run_build_efi_with("configs/qemu.yaml", "build/out.efi", |_, _| 1, |_, _| 0, |_, _| 0),
            0
        );
        assert_ne!(
            run_build_efi_with("configs/qemu.yaml", "build/out.efi", |_, _| 0, |_, _| 1, |_, _| 0),
            0
        );
        assert_ne!(
            run_build_efi_with("configs/qemu.yaml", "build/out.efi", |_, _| 0, |_, _| 0, |_, _| 1),
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
