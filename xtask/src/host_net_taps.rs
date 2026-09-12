//! Host tap interface helpers for live QEMU smoke with independent IN/OUT netdevs.

use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

/// Returns tap interface names required by the config host network plan.
pub fn required_tap_ifnames_from_config(config_path: &str) -> Result<Vec<String>, String> {
    let workspace = crate::workspace_root();
    let path = workspace.join(config_path);
    let yaml = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read config {}: {err}", path.display()))?;
    let compiled =
        hv_config_model::compile_config_from_str(&yaml).map_err(|err| err.to_string())?;
    if !compiled.normalized.qemu.network.enabled {
        return Ok(Vec::new());
    }
    if compiled.normalized.qemu.network.backend != "tap" {
        return Ok(Vec::new());
    }
    Ok(compiled
        .normalized
        .qemu
        .network
        .interfaces
        .iter()
        .filter_map(|interface| interface.tap_ifname.clone())
        .collect())
}

/// Returns whether a Linux tap/net device exists by name.
pub fn tap_interface_exists(name: &str) -> bool {
    Path::new("/sys/class/net").join(name).exists()
}

/// Returns an error listing tap interfaces that are missing on the host.
pub fn ensure_host_net_taps_ready(config_path: &str) -> Result<(), String> {
    let taps = required_tap_ifnames_from_config(config_path)?;
    let missing = taps
        .iter()
        .filter(|tap| !tap_interface_exists(tap))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "host network tap interfaces missing: {} (run: cargo xtask setup-host-net-taps)",
        missing.join(", ")
    ))
}

/// Creates and brings up tap interfaces declared in the config host network plan.
pub fn setup_host_net_taps(config_path: &str) -> Result<(), String> {
    let user = std::env::var("USER").unwrap_or_else(|_| String::from("root"));
    for tap in required_tap_ifnames_from_config(config_path)? {
        if tap_interface_exists(&tap) {
            continue;
        }
        let status = ProcessCommand::new("ip")
            .args(["tuntap", "add", "dev", &tap, "mode", "tap", "user", &user])
            .status()
            .map_err(|err| format!("failed to run ip tuntap add for {tap}: {err}"))?;
        if !status.success() {
            return Err(format!(
                "ip tuntap add failed for {tap} (try running with sufficient privileges)"
            ));
        }
        let status = ProcessCommand::new("ip")
            .args(["link", "set", &tap, "up"])
            .status()
            .map_err(|err| format!("failed to run ip link set up for {tap}: {err}"))?;
        if !status.success() {
            return Err(format!("ip link set up failed for {tap}"));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn reference_config_requires_distinct_taps() {
        let taps = required_tap_ifnames_from_config("configs/qemu.yaml").expect("taps");
        assert_eq!(taps.len(), 2);
        assert!(taps.contains(&String::from("hvdp-in0")));
        assert!(taps.contains(&String::from("hvdp-out0")));
    }

    #[test]
    fn ovmf_smoke_config_requires_no_taps() {
        let taps = required_tap_ifnames_from_config("configs/ovmf-smoke.yaml").expect("taps");
        assert!(taps.is_empty());
    }
}
