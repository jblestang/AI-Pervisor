//! hv-config CLI dispatch library.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

pub mod generate;

/// Parsed hv-config subcommands used by the CLI and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigCommand {
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
        output: PathBuf,
    },
}

/// Dispatches a parsed configuration command.
pub fn dispatch_config(command: ConfigCommand) -> i32 {
    match command {
        ConfigCommand::Validate { path } => validate_config(&path),
        ConfigCommand::Generate { path, output } => generate::generate(&path, &output),
    }
}

/// Validates a configuration file and prints status to stderr.
pub fn validate_config(path: &Path) -> i32 {
    match hv_config_model::compile_config_from_path(path) {
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
