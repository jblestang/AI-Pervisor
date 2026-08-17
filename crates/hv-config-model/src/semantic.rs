//! Semantic validation beyond syntax checks.

use std::collections::{HashMap, HashSet};

use crate::error::{ConfigError, ConfigErrorKind, ConfigWarning, WarningKind};
use crate::pci::parse_bdf;
use crate::raw::{RawConfig, RawDeviceRole, RawSmtPolicy};
use hv_types::PciBdf;

/// Validates semantic constraints and returns non-fatal warnings.
pub fn validate_semantics(raw: &RawConfig) -> Result<Vec<ConfigWarning>, ConfigError> {
    let mut warnings = Vec::new();
    let mut partition_ids = HashSet::new();
    let mut bdf_owner: HashMap<PciBdf, String> = HashMap::new();
    let mut core_owner: HashMap<u32, String> = HashMap::new();

    for partition in &raw.partitions {
        if !partition_ids.insert(partition.id.clone()) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!("duplicate partition id '{}'", partition.id),
            )
            .with_path(format!("partitions[id={}]", partition.id)));
        }

        for core in &partition.cpu_affinity.physical_cores {
            if let Some(existing) = core_owner.insert(*core, partition.id.clone()) {
                if raw.platform.requirements.smt_policy == RawSmtPolicy::ExclusiveCore {
                    return Err(ConfigError::new(
                        ConfigErrorKind::Semantic,
                        format!(
                            "physical core {core} assigned to '{existing}' and '{}'",
                            partition.id
                        ),
                    )
                    .with_path(format!("partitions[id={}].cpu_affinity", partition.id)));
                }
                warnings.push(
                    ConfigWarning::new(
                        WarningKind::Security,
                        format!(
                            "physical core {core} shared between '{existing}' and '{}'",
                            partition.id
                        ),
                    )
                    .with_path("platform.requirements.smt_policy"),
                );
            }
        }

        for device in &partition.devices {
            let bdf = parse_bdf(&device.bdf).map_err(|err| {
                err.with_path(format!("partitions[id={}].devices.bdf", partition.id))
            })?;
            if let Some(existing) = bdf_owner.insert(bdf, partition.id.clone()) {
                return Err(ConfigError::new(
                    ConfigErrorKind::Semantic,
                    format!(
                        "PCI BDF {bdf:?} assigned to '{existing}' and '{}'",
                        partition.id
                    ),
                )
                .with_path(format!("partitions[id={}].devices", partition.id)));
            }
        }
    }

    validate_ipc_graph(raw, &partition_ids)?;
    validate_guest_images(raw, &partition_ids)?;
    validate_datapath_policy(raw)?;

    if raw.platform.requirements.smt_policy == RawSmtPolicy::AllowCrossPartition {
        warnings.push(ConfigWarning::new(
            WarningKind::Security,
            "cross-partition SMT siblings are allowed by policy",
        ));
        warnings.push(ConfigWarning::new(
            WarningKind::Timing,
            "cross-partition SMT may increase jitter",
        ));
    }

    Ok(warnings)
}

fn validate_ipc_graph(raw: &RawConfig, partition_ids: &HashSet<String>) -> Result<(), ConfigError> {
    let mut channel_ids = HashSet::new();
    let mut edges: Vec<(String, String)> = Vec::new();

    for channel in &raw.ipc_channels {
        if !channel_ids.insert(channel.id.clone()) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!("duplicate ipc channel id '{}'", channel.id),
            )
            .with_path(format!("ipc_channels[id={}]", channel.id)));
        }
        if !partition_ids.contains(&channel.producer) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!("unknown producer partition '{}'", channel.producer),
            )
            .with_path(format!("ipc_channels[id={}].producer", channel.id)));
        }
        if !partition_ids.contains(&channel.consumer) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!("unknown consumer partition '{}'", channel.consumer),
            )
            .with_path(format!("ipc_channels[id={}].consumer", channel.id)));
        }
        if channel.producer == channel.consumer {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                "ipc producer and consumer must differ",
            )
            .with_path(format!("ipc_channels[id={}]", channel.id)));
        }
        edges.push((channel.producer.clone(), channel.consumer.clone()));
    }

    if has_cycle(&edges) {
        return Err(ConfigError::new(
            ConfigErrorKind::Semantic,
            "ipc channel graph contains a cycle",
        )
        .with_path("ipc_channels"));
    }

    Ok(())
}

fn validate_guest_images(
    raw: &RawConfig,
    partition_ids: &HashSet<String>,
) -> Result<(), ConfigError> {
    for image in &raw.boot.guest_images {
        if !partition_ids.contains(&image.partition) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!(
                    "guest image references unknown partition '{}'",
                    image.partition
                ),
            )
            .with_path(format!("boot.guest_images[partition={}]", image.partition)));
        }
        if image.sha256.len() != 64 || !image.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                "guest image sha256 must be 64 hex characters",
            )
            .with_path(format!(
                "boot.guest_images[partition={}].sha256",
                image.partition
            )));
        }
    }
    Ok(())
}

fn validate_datapath_policy(raw: &RawConfig) -> Result<(), ConfigError> {
    if !raw.security.require_mid_in_datapath {
        return Ok(());
    }

    let mut ingress: HashSet<String> = HashSet::new();
    let mut egress: HashSet<String> = HashSet::new();
    for partition in &raw.partitions {
        for device in &partition.devices {
            match device.role {
                Some(RawDeviceRole::DatapathIn) => {
                    ingress.insert(partition.id.clone());
                }
                Some(RawDeviceRole::DatapathOut) => {
                    egress.insert(partition.id.clone());
                }
                None => {}
            }
        }
    }

    for in_part in &ingress {
        for out_part in &egress {
            if in_part == out_part {
                continue;
            }
            if has_direct_ipc_path(in_part, out_part, raw) {
                return Err(ConfigError::new(
                    ConfigErrorKind::Semantic,
                    format!(
                        "direct ipc path from ingress partition '{in_part}' to egress partition '{out_part}' violates require_mid_in_datapath"
                    ),
                )
                .with_path("security.require_mid_in_datapath"));
            }
        }
    }

    Ok(())
}

fn has_direct_ipc_path(from: &str, to: &str, raw: &RawConfig) -> bool {
    raw.ipc_channels
        .iter()
        .any(|channel| channel.producer == from && channel.consumer == to)
}

fn has_cycle(edges: &[(String, String)]) -> bool {
    let mut nodes: HashSet<String> = HashSet::new();
    for (from, to) in edges {
        nodes.insert(from.clone());
        nodes.insert(to.clone());
    }

    for start in nodes {
        if dfs_has_cycle(&start, edges, &mut HashSet::new()) {
            return true;
        }
    }
    false
}

fn dfs_has_cycle(node: &str, edges: &[(String, String)], visiting: &mut HashSet<String>) -> bool {
    if !visiting.insert(node.to_string()) {
        return true;
    }
    for (from, to) in edges {
        if from == node && dfs_has_cycle(to, edges, visiting) {
            return true;
        }
    }
    visiting.remove(node);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::load_raw_from_str;
    use proptest::prop_assert;

    #[test]
    fn rejects_duplicate_partition_ids() {
        let yaml = include_str!("../tests/fixtures/invalid/duplicate_partition.yaml");
        let raw = load_raw_from_str(yaml).expect("parse");
        crate::syntax::validate_syntax(&raw).expect("syntax");
        let err = validate_semantics(&raw).expect_err("semantic");
        assert_eq!(err.kind, ConfigErrorKind::Semantic);
    }

    proptest::proptest! {
        #[test]
        fn ipc_graph_rejects_self_loop(producer in "abc") {
            let edges = vec![(producer.clone(), producer)];
            prop_assert!(has_cycle(&edges));
        }
    }
}
