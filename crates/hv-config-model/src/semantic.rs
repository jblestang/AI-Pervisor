//! Semantic validation beyond syntax checks.

use std::collections::{HashMap, HashSet};

use crate::error::{ConfigError, ConfigErrorKind, ConfigWarning, WarningKind};
use crate::pci::{parse_bdf, parse_guest_phys};
use crate::raw::{RawConfig, RawDeviceRole, RawSmtPolicy};
use hv_types::{PciBdf, SHA256_HEX_LEN};

/// Intel e1000 MMIO BAR size used for overlap validation.
const NIC_E1000_MMIO_BYTES: u64 = 0x20_000;

/// Validates semantic constraints and returns non-fatal warnings.
pub fn validate_semantics(raw: &RawConfig) -> Result<Vec<ConfigWarning>, ConfigError> {
    let mut warnings = Vec::new();
    let mut partition_ids = HashSet::new();
    let mut bdf_owner: HashMap<PciBdf, String> = HashMap::new();
    let mut core_owner: HashMap<u32, String> = HashMap::new();
    let mut mmio_regions: Vec<(u64, u64, String)> = Vec::new();

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
            let mmio_guest_phys = parse_guest_phys(&device.mmio_guest_phys).map_err(|err| {
                err.with_path(format!(
                    "partitions[id={}].devices.mmio_guest_phys",
                    partition.id
                ))
            })?;
            if mmio_guest_phys == 0 {
                return Err(ConfigError::new(
                    ConfigErrorKind::Semantic,
                    "device mmio_guest_phys must be non-zero",
                )
                .with_path(format!(
                    "partitions[id={}].devices.mmio_guest_phys",
                    partition.id
                )));
            }
            let mmio_end = mmio_guest_phys
                .checked_add(NIC_E1000_MMIO_BYTES)
                .ok_or_else(|| {
                    ConfigError::new(
                        ConfigErrorKind::Semantic,
                        "device mmio_guest_phys window overflow",
                    )
                    .with_path(format!(
                        "partitions[id={}].devices.mmio_guest_phys",
                        partition.id
                    ))
                })?;
            for (existing_base, existing_end, existing_owner) in &mmio_regions {
                if mmio_guest_phys < *existing_end && mmio_end > *existing_base {
                    return Err(ConfigError::new(
                        ConfigErrorKind::Semantic,
                        format!(
                            "device mmio_guest_phys overlaps '{existing_owner}' and '{}'",
                            partition.id
                        ),
                    )
                    .with_path(format!(
                        "partitions[id={}].devices.mmio_guest_phys",
                        partition.id
                    )));
                }
            }
            mmio_regions.push((mmio_guest_phys, mmio_end, partition.id.clone()));
            if mmio_guest_phys % 4096 != 0 {
                return Err(ConfigError::new(
                    ConfigErrorKind::Semantic,
                    "device mmio_guest_phys must be page aligned",
                )
                .with_path(format!(
                    "partitions[id={}].devices.mmio_guest_phys",
                    partition.id
                )));
            }
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
    validate_host_network(raw, &partition_ids)?;

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
        if image.sha256.len() != SHA256_HEX_LEN
            || !image.sha256.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!("guest image sha256 must be {SHA256_HEX_LEN} hex characters"),
            )
            .with_path(format!(
                "boot.guest_images[partition={}].sha256",
                image.partition
            )));
        }
    }
    Ok(())
}

fn validate_host_network(
    raw: &RawConfig,
    partition_ids: &HashSet<String>,
) -> Result<(), ConfigError> {
    if !raw.qemu.network.enabled {
        return Ok(());
    }

    let mut network_bdfs: HashMap<PciBdf, String> = HashMap::new();
    let mut network_partitions: HashSet<String> = HashSet::new();
    let mut tap_ifnames: HashSet<String> = HashSet::new();

    for (index, interface) in raw.qemu.network.interfaces.iter().enumerate() {
        let path = format!("qemu.network.interfaces[{index}]");
        if !partition_ids.contains(&interface.partition) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!(
                    "host network interface references unknown partition '{}'",
                    interface.partition
                ),
            )
            .with_path(format!("{path}.partition")));
        }
        if !network_partitions.insert(interface.partition.clone()) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!(
                    "duplicate host network interface for partition '{}'",
                    interface.partition
                ),
            )
            .with_path(format!("{path}.partition")));
        }
        let bdf = parse_bdf(&interface.bdf).map_err(|err| err.with_path(format!("{path}.bdf")))?;
        if let Some(existing) = network_bdfs.insert(bdf, interface.partition.clone()) {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!(
                    "host network PCI BDF {bdf:?} assigned to '{existing}' and '{}'",
                    interface.partition
                ),
            )
            .with_path(path));
        }

        let partition = raw
            .partitions
            .iter()
            .find(|partition| partition.id == interface.partition)
            .ok_or_else(|| {
                ConfigError::new(
                    ConfigErrorKind::Semantic,
                    format!(
                        "host network interface references unknown partition '{}'",
                        interface.partition
                    ),
                )
                .with_path(format!("{path}.partition"))
            })?;
        let matching_device = partition
            .devices
            .iter()
            .find(|device| parse_bdf(&device.bdf).ok() == Some(bdf));
        let Some(device) = matching_device else {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                format!(
                    "host network BDF {bdf:?} missing from partition '{}' devices",
                    interface.partition
                ),
            )
            .with_path(path));
        };
        if device.kind != crate::raw::RawDeviceKind::NicE1000 {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                "host network interface BDF must reference a nic_e1000 device",
            )
            .with_path(path));
        }

        if raw.qemu.network.backend == "tap"
            && interface.tap_ifname.as_deref().is_none_or(str::is_empty)
        {
            return Err(ConfigError::new(
                ConfigErrorKind::Semantic,
                "host network tap backend requires tap_ifname on each interface",
            )
            .with_path(format!("{path}.tap_ifname")));
        }

        if let Some(tap_ifname) = interface.tap_ifname.as_deref() {
            if !tap_ifnames.insert(tap_ifname.to_string()) {
                return Err(ConfigError::new(
                    ConfigErrorKind::Semantic,
                    format!("duplicate host network tap interface '{tap_ifname}'"),
                )
                .with_path(format!("{path}.tap_ifname")));
            }
        }
    }

    for partition in &raw.partitions {
        for device in &partition.devices {
            let Some(role) = device.role else {
                continue;
            };
            let bdf = parse_bdf(&device.bdf).map_err(|err| {
                err.with_path(format!("partitions[id={}].devices.bdf", partition.id))
            })?;
            if !raw.qemu.network.interfaces.iter().any(|interface| {
                interface.partition == partition.id && parse_bdf(&interface.bdf).ok() == Some(bdf)
            }) {
                return Err(ConfigError::new(
                    ConfigErrorKind::Semantic,
                    format!(
                        "datapath role '{role:?}' device on partition '{}' missing from host network plan",
                        partition.id
                    ),
                )
                .with_path(format!("partitions[id={}].devices.role", partition.id)));
            }
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
#[allow(clippy::expect_used)]
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

    #[test]
    fn datapath_policy_ignores_devices_without_role() {
        let yaml = include_str!("../tests/fixtures/valid/datapath_device_without_role.yaml");
        let raw = load_raw_from_str(yaml).expect("parse");
        crate::syntax::validate_syntax(&raw).expect("syntax");
        validate_semantics(&raw).expect("semantic");
    }

    #[test]
    fn datapath_policy_skips_same_partition_ingress_and_egress() {
        let yaml = include_str!("../tests/fixtures/valid/datapath_same_partition_gateway.yaml");
        let raw = load_raw_from_str(yaml).expect("parse");
        crate::syntax::validate_syntax(&raw).expect("syntax");
        validate_semantics(&raw).expect("semantic");
    }

    #[test]
    fn host_network_rejects_unknown_partition() {
        let yaml = include_str!("../tests/fixtures/invalid/host_network_unknown_partition.yaml");
        let raw = load_raw_from_str(yaml).expect("parse");
        crate::syntax::validate_syntax(&raw).expect("syntax");
        let err = validate_semantics(&raw).expect_err("semantic");
        assert_eq!(err.kind, ConfigErrorKind::Semantic);
    }

    #[test]
    fn host_network_rejects_bdf_missing_from_partition() {
        let yaml = include_str!("../tests/fixtures/invalid/host_network_bdf_mismatch.yaml");
        let raw = load_raw_from_str(yaml).expect("parse");
        crate::syntax::validate_syntax(&raw).expect("syntax");
        let err = validate_semantics(&raw).expect_err("semantic");
        assert_eq!(err.kind, ConfigErrorKind::Semantic);
    }

    #[test]
    fn host_network_rejects_tap_backend_without_ifname() {
        let yaml = include_str!("../tests/fixtures/invalid/host_network_tap_missing_ifname.yaml");
        let raw = load_raw_from_str(yaml).expect("parse");
        crate::syntax::validate_syntax(&raw).expect("syntax");
        let err = validate_semantics(&raw).expect_err("semantic");
        assert_eq!(err.kind, ConfigErrorKind::Semantic);
    }
}
