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
use constants::{DEFAULT_COVERAGE_MIN_LINES, DEFAULT_FUZZ_RUNS};
use hv_config::constants::DEFAULT_CONFIG_OUTPUT_DIR;

/// Runs `cargo test --workspace`.
pub fn run_tests() -> i32 {
    test_command(run)
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
    coverage_command(min_lines, run)
}

fn test_command(runner: fn(&str, &[&str]) -> i32) -> i32 {
    runner("cargo", &["test", "--workspace"])
}

fn coverage_command(min_lines: u8, runner: fn(&str, &[&str]) -> i32) -> i32 {
    let threshold = min_lines.to_string();
    runner(
        "cargo",
        &[
            "llvm-cov",
            "--workspace",
            "--summary-only",
            "--fail-under-lines",
            &threshold,
        ],
    )
}

/// Dispatches a parsed task to the appropriate handler.
pub fn dispatch_task(task: TaskCommand) -> i32 {
    dispatch_task_with(task, run)
}

fn dispatch_task_with(task: TaskCommand, runner: fn(&str, &[&str]) -> i32) -> i32 {
    match task {
        TaskCommand::Test => test_command(runner),
        TaskCommand::Build => runner("cargo", &["build", "--workspace"]),
        TaskCommand::Coverage { min_lines } => coverage_command(min_lines, runner),
        TaskCommand::Fuzz { runs } => fuzz_command(runs, run_with_cxx_gpp),
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
    use crate::constants::{DEFAULT_COVERAGE_MIN_LINES, DEFAULT_FUZZ_RUNS};
    use hv_config::constants::DEFAULT_CONFIG_OUTPUT_DIR;

    fn test_command_with(runner: fn(&str, &[&str]) -> i32) -> i32 {
        test_command(runner)
    }

    fn coverage_command_with(min_lines: u8, runner: fn(&str, &[&str]) -> i32) -> i32 {
        coverage_command(min_lines, runner)
    }

    fn mock_test_runner(program: &str, args: &[&str]) -> i32 {
        assert_eq!(program, "cargo");
        assert_eq!(args, &["test", "--workspace"]);
        0
    }

    fn mock_coverage_runner(program: &str, args: &[&str]) -> i32 {
        assert_eq!(program, "cargo");
        let threshold = DEFAULT_COVERAGE_MIN_LINES.to_string();
        assert_eq!(
            args,
            &[
                "llvm-cov",
                "--workspace",
                "--summary-only",
                "--fail-under-lines",
                threshold.as_str(),
            ]
        );
        0
    }

    fn mock_build_runner(program: &str, args: &[&str]) -> i32 {
        assert_eq!(program, "cargo");
        assert_eq!(args, &["build", "--workspace"]);
        0
    }

    #[test]
    fn run_tests_invokes_cargo_test_workspace() {
        assert_eq!(test_command_with(mock_test_runner), 0);
    }

    #[test]
    fn run_coverage_passes_threshold_to_llvm_cov() {
        assert_eq!(
            coverage_command_with(DEFAULT_COVERAGE_MIN_LINES, mock_coverage_runner),
            0
        );
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
        assert_eq!(
            dispatch_task_with(
                TaskCommand::Coverage {
                    min_lines: DEFAULT_COVERAGE_MIN_LINES,
                },
                mock_coverage_runner,
            ),
            0
        );
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
    }

    #[test]
    fn public_run_tests_wrapper_executes_once() {
        if std::env::var("XTASK_COVER_RUN_TESTS").is_ok() {
            return;
        }
        std::env::set_var("XTASK_COVER_RUN_TESTS", "1");
        assert_eq!(run_tests(), 0);
    }
}
