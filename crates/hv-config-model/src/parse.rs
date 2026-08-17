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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn load_raw_from_str_parses_reference() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let raw = load_raw_from_str(yaml);
        assert!(raw.is_ok());
    }

    #[test]
    fn load_raw_from_path_reads_reference() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
        let raw = load_raw_from_path(&path);
        assert!(raw.is_ok());
    }

    #[test]
    fn load_raw_from_missing_path_fails() {
        let path = Path::new("/no/such/config/file.yaml");
        let err = load_raw_from_path(path).expect_err("missing file");
        assert_eq!(err.kind, ConfigErrorKind::Parse);
        assert_eq!(err.file, Some(path.to_path_buf()));
    }
}
