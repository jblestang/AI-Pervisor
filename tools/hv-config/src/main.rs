//! Configuration compiler CLI.

mod generate;

use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};
use hv_config_model::compile_config_from_path;

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
        path: PathBuf,
    },
    /// Generate review artifacts from a configuration file.
    Generate {
        /// Path to the YAML configuration file.
        path: PathBuf,
        /// Output directory for generated artifacts.
        #[arg(short, long, default_value = "build/config")]
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let status = match cli.command {
        Command::Validate { path } => validate(&path),
        Command::Generate { path, output } => generate::generate(&path, &output),
    };
    process::exit(status);
}

fn validate(path: &Path) -> i32 {
    match compile_config_from_path(path) {
        Ok(compiled) => {
            eprintln!("configuration valid: {}", path.display());
            eprintln!("config digest: {}", compiled.digest.to_hex());
            for warning in compiled.warnings {
                eprintln!("warning: {warning}");
            }
            0
        }
        Err(err) => {
            eprintln!("error: {err}");
            1
        }
    }
}
