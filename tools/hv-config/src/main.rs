//! Configuration compiler CLI entry point.

use std::process;

use clap::{Parser, Subcommand};
use hv_config::{dispatch_config, ConfigCommand};

#[derive(Parser)]
#[command(name = "hv-config", about = "Static hypervisor configuration compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a configuration file.
    Validate {
        /// Path to the YAML configuration file.
        path: std::path::PathBuf,
    },
    /// Generate review artifacts from a configuration file.
    Generate {
        /// Path to the YAML configuration file.
        path: std::path::PathBuf,
        /// Output directory for generated artifacts.
        #[arg(short, long, default_value = "build/config")]
        output: std::path::PathBuf,
    },
}

fn map_command(command: Command) -> ConfigCommand {
    match command {
        Command::Validate { path } => ConfigCommand::Validate { path },
        Command::Generate { path, output } => ConfigCommand::Generate { path, output },
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
    process::exit(dispatch_config(map_command(cli.command)));
}
