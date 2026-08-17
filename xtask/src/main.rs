//! Workspace task runner entry point.

use std::process;

use clap::{Parser, Subcommand};
use xtask::{dispatch_task, TaskCommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Static hypervisor developer tasks")]
struct Cli {
    #[command(subcommand)]
    command: TaskCommandCli,
}

#[derive(Subcommand)]
enum TaskCommandCli {
    /// Run host unit tests.
    Test,
    /// Build all workspace crates.
    Build,
    /// Run tests and enforce minimum line coverage.
    Coverage {
        /// Minimum required line coverage percentage.
        #[arg(long, default_value_t = 95)]
        min_lines: u8,
    },
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

fn map_command(command: TaskCommandCli) -> TaskCommand {
    match command {
        TaskCommandCli::Test => TaskCommand::Test,
        TaskCommandCli::Build => TaskCommand::Build,
        TaskCommandCli::Coverage { min_lines } => TaskCommand::Coverage { min_lines },
        TaskCommandCli::Config { action } => match action {
            ConfigAction::Validate { path } => TaskCommand::ConfigValidate { path },
            ConfigAction::Generate { path, output } => TaskCommand::ConfigGenerate { path, output },
        },
    }
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let _ = err.print();
            process::exit(2);
        }
    };
    process::exit(dispatch_task(map_command(cli.command)));
}
