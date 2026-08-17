//! Workspace task runner library.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

use std::path::Path;
use std::process::Command as ProcessCommand;

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
        TaskCommand::ConfigValidate { path } => run_config_validate(&path),
        TaskCommand::ConfigGenerate { path, output } => run_config_generate(&path, &output),
    }
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
        assert_eq!(
            args,
            &[
                "llvm-cov",
                "--workspace",
                "--summary-only",
                "--fail-under-lines",
                "95",
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
        assert_eq!(coverage_command_with(95, mock_coverage_runner), 0);
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
            dispatch_task_with(TaskCommand::Coverage { min_lines: 95 }, mock_coverage_runner),
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
}
