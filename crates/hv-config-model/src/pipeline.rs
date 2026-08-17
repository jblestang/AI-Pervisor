//! End-to-end configuration compilation pipeline.

use std::path::Path;

use crate::digest::{config_digest, ConfigDigest};
use crate::error::{ConfigError, ConfigWarning};
use crate::intent::{static_intent_ir, StaticIntentIR};
use crate::normalize::{normalize, NormalizedConfig};
use crate::parse::{load_raw_from_path, load_raw_from_str};
use crate::raw::RawConfig;
use crate::requirements::{platform_requirements, PlatformRequirements};
use crate::semantic::validate_semantics;
use crate::syntax::validate_syntax;

/// Fully compiled configuration artifacts.
#[derive(Debug, Clone)]
pub struct CompiledConfig {
    /// Canonical normalized configuration.
    pub normalized: NormalizedConfig,
    /// Derived platform requirements.
    pub requirements: PlatformRequirements,
    /// Static intent IR.
    pub intent: StaticIntentIR,
    /// Configuration digest.
    pub digest: ConfigDigest,
    /// Non-fatal warnings.
    pub warnings: Vec<ConfigWarning>,
}

/// Compiles a configuration file from disk.
pub fn compile_config_from_path(path: &Path) -> Result<CompiledConfig, ConfigError> {
    let raw = load_raw_from_path(path)?;
    compile_config(raw)
}

/// Compiles a configuration document from YAML text.
pub fn compile_config_from_str(contents: &str) -> Result<CompiledConfig, ConfigError> {
    let raw = load_raw_from_str(contents)?;
    compile_config(raw)
}

/// Compiles a raw configuration through the full validation pipeline.
pub fn compile_config(raw: RawConfig) -> Result<CompiledConfig, ConfigError> {
    validate_syntax(&raw)?;
    let warnings = validate_semantics(&raw)?;
    let normalized = normalize(raw)?;
    let requirements = platform_requirements(&normalized);
    let intent = static_intent_ir(&normalized, &requirements)?;
    let digest = config_digest(&normalized)?;
    Ok(CompiledConfig {
        normalized,
        requirements,
        intent,
        digest,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn compile_config_from_path_succeeds() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/qemu.yaml");
        let compiled = compile_config_from_path(&path).expect("compile");
        assert_eq!(compiled.normalized.partitions.len(), 3);
    }

    #[test]
    fn compile_config_roundtrip() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let raw = crate::parse::load_raw_from_str(yaml).expect("parse");
        let compiled = compile_config(raw).expect("compile");
        assert!(!compiled.intent.ipc.is_empty());
    }
}
