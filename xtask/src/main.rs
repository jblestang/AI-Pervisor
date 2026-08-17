//! Workspace task runner.

use std::process::{self, Command as ProcessCommand};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Static hypervisor developer tasks")]
struct Cli {
    #[command(subcommand)]
    command: TaskCommand,
}

#[derive(Subcommand)]
enum TaskCommand {
    /// Run host unit tests.
    Test,
    /// Build all workspace crates.
    Build,
    /// Validate a configuration file.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
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
        #[arg(short, long, default_value = "build/config")]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let status = match cli.command {
        TaskCommand::Test => run("cargo", &["test", "--workspace"]),
        TaskCommand::Build => run("cargo", &["build", "--workspace"]),
        TaskCommand::Config { action } => match action {
            ConfigAction::Validate { path } => run(
                "cargo",
                &["run", "-q", "-p", "hv-config", "--", "validate", &path],
            ),
            ConfigAction::Generate { path, output } => run(
                "cargo",
                &[
                    "run",
                    "-q",
                    "-p",
                    "hv-config",
                    "--",
                    "generate",
                    &path,
                    "-o",
                    &output,
                ],
            ),
        },
    };
    process::exit(status);
}

fn run(program: &str, args: &[&str]) -> i32 {
    let status = ProcessCommand::new(program).args(args).status();
    match status {
        Ok(status) => {
            if status.success() {
                0
            } else {
                status.code().unwrap_or(1)
            }
        }
        Err(err) => {
            eprintln!("failed to run {program}: {err}");
            1
        }
    }
}
