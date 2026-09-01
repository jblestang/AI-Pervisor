//! QEMU e1000 + netdev argument planning from configuration.

use std::fs;

/// Planned QEMU CLI fragments for host-connected e1000 NICs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QemuNetworkPlan {
    /// Extra `qemu-system-x86_64` arguments (`-netdev`, `-device`, ...).
    pub args: Vec<String>,
}

/// Builds QEMU network/device arguments from a YAML configuration path.
pub fn plan_qemu_network_from_config(config_path: &str) -> Result<QemuNetworkPlan, String> {
    let workspace = crate::workspace_root();
    let path = workspace.join(config_path);
    let yaml = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read config {}: {err}", path.display()))?;
    let compiled =
        hv_config_model::compile_config_from_str(&yaml).map_err(|err| err.to_string())?;
    let network = &compiled.normalized.qemu.network;
    if !network.enabled {
        return Ok(QemuNetworkPlan::default());
    }
    if network.interfaces.is_empty() {
        return Err(String::from(
            "qemu.network.enabled requires at least one interface entry",
        ));
    }

    let mut args = Vec::new();
    for interface in &network.interfaces {
        if interface.netdev_id.is_empty() {
            return Err(String::from("qemu network interface missing netdev_id"));
        }
        let netdev = match network.backend.as_str() {
            "user" => format!("-netdev user,id={}", interface.netdev_id),
            "tap" => {
                let ifname = interface.tap_ifname.as_deref().ok_or_else(|| {
                    format!(
                        "qemu network tap backend requires tap_ifname for {}",
                        interface.netdev_id
                    )
                })?;
                format!(
                    "-netdev tap,id={},ifname={},script=no,downscript=no",
                    interface.netdev_id, ifname
                )
            }
            other => {
                return Err(format!(
                    "unsupported qemu network backend '{other}' (expected user or tap)"
                ));
            }
        };
        args.push(netdev);
        if interface.pci_addr.is_empty() {
            return Err(format!(
                "qemu network interface {} missing pci_addr",
                interface.bdf
            ));
        }
        args.push(format!(
            "-device e1000,netdev={},bus=pcie.0,addr={}",
            interface.netdev_id, interface.pci_addr
        ));
    }
    Ok(QemuNetworkPlan { args })
}

/// Returns whether host networking should be attached for a config path.
pub fn qemu_network_enabled_in_config(config_path: &str) -> bool {
    plan_qemu_network_from_config(config_path)
        .map(|plan| !plan.args.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn plan_qemu_network_disabled_by_default_for_ovmf_smoke_config() {
        let plan = plan_qemu_network_from_config("configs/ovmf-smoke.yaml").expect("plan");
        assert!(plan.args.is_empty());
    }

    #[test]
    fn plan_qemu_network_builds_independent_tap_devices_for_reference_config() {
        let plan = plan_qemu_network_from_config("configs/qemu.yaml").expect("plan");
        assert_eq!(plan.args.len(), 4);
        assert!(plan.args.iter().any(|arg| {
            arg.contains("-netdev tap,id=net_in,ifname=hvdp-in0,script=no,downscript=no")
        }));
        assert!(plan
            .args
            .iter()
            .any(|arg| arg.contains("-device e1000,netdev=net_in,bus=pcie.0,addr=0x3")));
        assert!(plan.args.iter().any(|arg| {
            arg.contains("-netdev tap,id=net_out,ifname=hvdp-out0,script=no,downscript=no")
        }));
        assert!(plan
            .args
            .iter()
            .any(|arg| arg.contains("-device e1000,netdev=net_out,bus=pcie.0,addr=0x4")));
    }

    #[test]
    fn plan_qemu_network_uses_distinct_tap_ifnames_for_in_and_out() {
        let yaml = include_str!("../../configs/qemu.yaml");
        let compiled = hv_config_model::compile_config_from_str(yaml).expect("compile");
        let interfaces = &compiled.normalized.qemu.network.interfaces;
        let in_if = interfaces
            .iter()
            .find(|iface| iface.partition == "in")
            .and_then(|iface| iface.tap_ifname.as_deref())
            .expect("in tap");
        let out_if = interfaces
            .iter()
            .find(|iface| iface.partition == "out")
            .and_then(|iface| iface.tap_ifname.as_deref())
            .expect("out tap");
        assert_ne!(in_if, out_if);
    }

    #[test]
    fn plan_qemu_network_rejects_missing_config() {
        assert!(plan_qemu_network_from_config("configs/missing.yaml").is_err());
    }
}
