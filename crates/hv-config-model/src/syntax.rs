//! Syntax and schema validation.

use crate::constants::SUPPORTED_ARCH;
use crate::error::{ConfigError, ConfigErrorKind};
use crate::raw::{RawConfig, SUPPORTED_SCHEMA_VERSION};

/// Validates syntax-level constraints on a raw configuration document.
pub fn validate_syntax(raw: &RawConfig) -> Result<(), ConfigError> {
    if raw.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(ConfigError::new(
            ConfigErrorKind::Syntax,
            format!(
                "unsupported schema_version {} (expected {SUPPORTED_SCHEMA_VERSION})",
                raw.schema_version
            ),
        )
        .with_path("schema_version"));
    }

    if raw.platform.name.is_empty() {
        return Err(
            ConfigError::new(ConfigErrorKind::Syntax, "platform.name must not be empty")
                .with_path("platform.name"),
        );
    }

    if raw.platform.requirements.arch != SUPPORTED_ARCH {
        return Err(ConfigError::new(
            ConfigErrorKind::Syntax,
            format!("unsupported arch '{}'", raw.platform.requirements.arch),
        )
        .with_path("platform.requirements.arch"));
    }

    if raw.partitions.is_empty() {
        return Err(ConfigError::new(
            ConfigErrorKind::Syntax,
            "at least one partition is required",
        )
        .with_path("partitions"));
    }

    for (index, partition) in raw.partitions.iter().enumerate() {
        if partition.id.is_empty() {
            return Err(ConfigError::new(
                ConfigErrorKind::Syntax,
                "partition id must not be empty",
            )
            .with_path(format!("partitions[{index}].id")));
        }
        if partition.vcpus == 0 {
            return Err(
                ConfigError::new(ConfigErrorKind::Syntax, "partition vcpus must be >= 1")
                    .with_path(format!("partitions[{index}].vcpus")),
            );
        }
        if partition.memory_gib == 0 {
            return Err(ConfigError::new(
                ConfigErrorKind::Syntax,
                "partition memory_gib must be >= 1",
            )
            .with_path(format!("partitions[{index}].memory_gib")));
        }
    }

    for (index, channel) in raw.ipc_channels.iter().enumerate() {
        if channel.id.is_empty() {
            return Err(ConfigError::new(
                ConfigErrorKind::Syntax,
                "ipc channel id must not be empty",
            )
            .with_path(format!("ipc_channels[{index}].id")));
        }
        if channel.queue_slots == 0 {
            return Err(
                ConfigError::new(ConfigErrorKind::Syntax, "ipc queue_slots must be >= 1")
                    .with_path(format!("ipc_channels[{index}].queue_slots")),
            );
        }
        if channel.slot_size_bytes == 0 {
            return Err(ConfigError::new(
                ConfigErrorKind::Syntax,
                "ipc slot_size_bytes must be >= 1",
            )
            .with_path(format!("ipc_channels[{index}].slot_size_bytes")));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::load_raw_from_str;

    #[test]
    fn rejects_unknown_schema_version() {
        let yaml = include_str!("../tests/fixtures/invalid/bad_schema_version.yaml");
        let raw = load_raw_from_str(yaml).expect("yaml parse");
        let err = validate_syntax(&raw).expect_err("must fail");
        assert_eq!(err.kind, ConfigErrorKind::Syntax);
    }

    #[test]
    fn rejects_empty_platform_name() {
        let yaml = include_str!("../tests/fixtures/invalid/empty_platform_name.yaml");
        let raw = load_raw_from_str(yaml).expect("yaml parse");
        let err = validate_syntax(&raw).expect_err("must fail");
        assert_eq!(err.kind, ConfigErrorKind::Syntax);
    }

    #[test]
    fn rejects_empty_partitions_list() {
        let yaml = include_str!("../tests/fixtures/invalid/empty_partitions.yaml");
        let raw = load_raw_from_str(yaml).expect("yaml parse");
        let err = validate_syntax(&raw).expect_err("must fail");
        assert_eq!(err.kind, ConfigErrorKind::Syntax);
    }

    #[test]
    fn rejects_zero_vcpus_and_memory() {
        let yaml = include_str!("../tests/fixtures/invalid/zero_vcpus.yaml");
        let raw = load_raw_from_str(yaml).expect("yaml parse");
        assert!(validate_syntax(&raw).is_err());

        let yaml = include_str!("../tests/fixtures/invalid/zero_memory_gib.yaml");
        let raw = load_raw_from_str(yaml).expect("yaml parse");
        assert!(validate_syntax(&raw).is_err());
    }

    #[test]
    fn rejects_invalid_ipc_fields() {
        let empty_id = include_str!("../tests/fixtures/invalid/empty_ipc_id.yaml");
        assert!(validate_syntax(&load_raw_from_str(empty_id).expect("parse")).is_err());
        let zero_slots = include_str!("../tests/fixtures/invalid/zero_queue_slots.yaml");
        assert!(validate_syntax(&load_raw_from_str(zero_slots).expect("parse")).is_err());
        let zero_size = include_str!("../tests/fixtures/invalid/zero_slot_size.yaml");
        assert!(validate_syntax(&load_raw_from_str(zero_size).expect("parse")).is_err());
    }
}
