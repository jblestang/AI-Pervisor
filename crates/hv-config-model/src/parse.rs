//! YAML loading helpers.

use std::fs;
use std::path::Path;

use crate::error::{ConfigError, ConfigErrorKind};
use crate::raw::RawConfig;

/// Loads a raw configuration document from a filesystem path.
pub fn load_raw_from_path(path: &Path) -> Result<RawConfig, ConfigError> {
    let contents = fs::read_to_string(path).map_err(|err| {
        ConfigError::new(
            ConfigErrorKind::Parse,
            format!("failed to read config: {err}"),
        )
        .with_file(path.to_path_buf())
    })?;
    load_raw_from_str(&contents).map_err(|err| err.with_file(path.to_path_buf()))
}

/// Loads a raw configuration document from a string.
pub fn load_raw_from_str(contents: &str) -> Result<RawConfig, ConfigError> {
    serde_yaml::from_str(contents)
        .map_err(|err| ConfigError::new(ConfigErrorKind::Parse, format!("invalid YAML: {err}")))
}
