//! Configuration digest computation.

use sha2::{Digest, Sha256};

use crate::error::{ConfigError, ConfigErrorKind};
use crate::normalize::NormalizedConfig;

/// SHA-256 digest of the canonical normalized configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDigest {
    /// Raw digest bytes.
    pub bytes: [u8; 32],
}

impl ConfigDigest {
    /// Returns the digest as lowercase hex.
    pub fn to_hex(&self) -> String {
        let mut hex = String::with_capacity(64);
        for byte in self.bytes {
            use core::fmt::Write;
            let _ = write!(hex, "{byte:02x}");
        }
        hex
    }
}

/// Computes the SHA-256 digest of a normalized configuration.
pub fn config_digest(config: &NormalizedConfig) -> Result<ConfigDigest, ConfigError> {
    let json = serde_json::to_string(config).map_err(|err| {
        ConfigError::new(
            ConfigErrorKind::Internal,
            format!("failed to serialize normalized config: {err}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Ok(ConfigDigest { bytes })
}

#[cfg(test)]
mod tests {
    use crate::pipeline::compile_config_from_str;

    #[test]
    fn digest_is_stable() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled_a = compile_config_from_str(yaml).expect("compile");
        let compiled_b = compile_config_from_str(yaml).expect("compile");
        assert_eq!(
            compiled_a.digest.to_hex(),
            compiled_b.digest.to_hex(),
            "config hash must be deterministic"
        );
    }
}
