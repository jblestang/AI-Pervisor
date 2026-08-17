//! Configuration compiler library used by the CLI and tests.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

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

#[derive(Parser, Debug)]
#[command(name = "hv-config", about = "Static hypervisor configuration compiler")]
struct Cli {
    #[command(subcommand)]
    command: CommandCli,
}

#[derive(Subcommand, Debug)]
enum CommandCli {
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

/// Maps CLI subcommands to library dispatch values.
pub(crate) fn map_cli_command(command: CommandCli) -> ConfigCommand {
    match command {
        CommandCli::Validate { path } => ConfigCommand::Validate { path },
        CommandCli::Generate { path, output } => ConfigCommand::Generate { path, output },
    }
}

/// Parses CLI arguments into a [`ConfigCommand`].
pub fn parse_config_command<I, T>(args: I) -> Result<ConfigCommand, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    Ok(map_cli_command(cli.command))
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_validate_command() {
        let command = parse_config_command(["hv-config", "validate", "configs/qemu.yaml"])
            .expect("parse validate");
        assert_eq!(
            command,
            ConfigCommand::Validate {
                path: PathBuf::from("configs/qemu.yaml")
            }
        );
    }

    #[test]
    fn parse_generate_command_with_default_output() {
        let command = parse_config_command(["hv-config", "generate", "configs/qemu.yaml"])
            .expect("parse generate");
        assert_eq!(
            command,
            ConfigCommand::Generate {
                path: PathBuf::from("configs/qemu.yaml"),
                output: PathBuf::from("build/config"),
            }
        );
    }

    #[test]
    fn parse_generate_command_with_custom_output() {
        let command =
            parse_config_command(["hv-config", "generate", "configs/qemu.yaml", "-o", "out"])
                .expect("parse generate custom output");
        assert_eq!(
            command,
            ConfigCommand::Generate {
                path: PathBuf::from("configs/qemu.yaml"),
                output: PathBuf::from("out"),
            }
        );
    }
}
