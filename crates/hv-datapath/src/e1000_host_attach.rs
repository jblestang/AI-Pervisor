//! Validates independent outer host e1000 bindings (IN and OUT not linked).
//!
//! Outer QEMU attaches `net_in` and `net_out` to separate host tap interfaces
//! described in platform configuration. Nested guests own in→mid→out relay over IPC.

use hv_platform_model::{
    bdf_for_datapath_role, mmio_guest_phys_for_datapath_role, validate_layout_host_network_coherence,
    StaticPlatformIR, DATAPATH_ROLE_IN, DATAPATH_ROLE_OUT,
};
use hv_types::PciBdf;

use crate::constants::E1000_MMIO_SIZE_BYTES;
use crate::error::{DatapathError, DatapathErrorKind};

/// BAR0 bases discovered from outer PCI config space at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveredOuterHostBars {
    /// BAR0 MMIO base for the datapath ingress outer e1000.
    pub host_in_bar0: u64,
    /// BAR0 MMIO base for the datapath egress outer e1000.
    pub host_out_bar0: u64,
}

/// Planned independent host NIC bindings from platform layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E1000HostAttachPlan {
    /// PCI BDF for the datapath ingress outer e1000.
    pub host_in_bdf: PciBdf,
    /// PCI BDF for the datapath egress outer e1000.
    pub host_out_bdf: PciBdf,
}

/// Builds host attach bindings from static platform PCI intent and host network plan.
pub fn plan_e1000_host_attach(
    layout: &StaticPlatformIR,
) -> Result<E1000HostAttachPlan, DatapathError> {
    let host_in_bdf =
        bdf_for_datapath_role(layout, DATAPATH_ROLE_IN).map_err(map_platform_error)?;
    let host_out_bdf =
        bdf_for_datapath_role(layout, DATAPATH_ROLE_OUT).map_err(map_platform_error)?;
    if host_in_bdf == host_out_bdf {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "IN and OUT host NICs must use independent BDF bindings",
        ));
    }
    let in_mmio =
        mmio_guest_phys_for_datapath_role(layout, DATAPATH_ROLE_IN).map_err(map_platform_error)?;
    let out_mmio =
        mmio_guest_phys_for_datapath_role(layout, DATAPATH_ROLE_OUT).map_err(map_platform_error)?;
    if in_mmio == out_mmio {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "IN and OUT e1000 MMIO BAR bases must be independent",
        ));
    }
    if layout.host_network.enabled {
        validate_layout_host_network_coherence(layout).map_err(map_platform_error)?;
        validate_host_network_matches_pci(layout, host_in_bdf, host_out_bdf)?;
    }
    Ok(E1000HostAttachPlan {
        host_in_bdf,
        host_out_bdf,
    })
}

fn validate_host_network_matches_pci(
    layout: &StaticPlatformIR,
    host_in_bdf: PciBdf,
    host_out_bdf: PciBdf,
) -> Result<(), DatapathError> {
    let host_in = layout
        .host_network
        .interfaces
        .iter()
        .find(|interface| interface.bdf == host_in_bdf)
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "host network plan missing datapath IN interface",
            )
        })?;
    let host_out = layout
        .host_network
        .interfaces
        .iter()
        .find(|interface| interface.bdf == host_out_bdf)
        .ok_or_else(|| {
            DatapathError::new(
                DatapathErrorKind::InvalidInput,
                "host network plan missing datapath OUT interface",
            )
        })?;
    if host_in.tap_ifname.is_some()
        && host_out.tap_ifname.is_some()
        && host_in.tap_ifname == host_out.tap_ifname
    {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "host network plan must use distinct tap interfaces for IN and OUT",
        ));
    }
    Ok(())
}

/// Validates runtime-discovered outer e1000 BAR0 bases against platform contract BDFs.
pub fn validate_discovered_outer_host_bars(
    plan: &E1000HostAttachPlan,
    discovered: &DiscoveredOuterHostBars,
) -> Result<(), DatapathError> {
    validate_outer_bar0("datapath IN", discovered.host_in_bar0)?;
    validate_outer_bar0("datapath OUT", discovered.host_out_bar0)?;
    if discovered.host_in_bar0 == discovered.host_out_bar0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "discovered IN and OUT outer e1000 BAR0 bases must be independent",
        ));
    }
    if outer_bar_windows_overlap(
        discovered.host_in_bar0,
        discovered.host_out_bar0,
        E1000_MMIO_SIZE_BYTES,
    ) {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "discovered IN and OUT outer e1000 BAR0 windows overlap",
        ));
    }
    let _ = plan;
    Ok(())
}

fn outer_bar_windows_overlap(base_a: u64, base_b: u64, size: u64) -> bool {
    let end_a = base_a.saturating_add(size);
    let end_b = base_b.saturating_add(size);
    base_a < end_b && base_b < end_a
}

fn validate_outer_bar0(label: &str, bar0: u64) -> Result<(), DatapathError> {
    if bar0 == 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "discovered outer e1000 BAR0 must be non-zero",
        ));
    }
    if bar0 % 4096 != 0 {
        return Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "discovered outer e1000 BAR0 must be page aligned",
        ));
    }
    let _ = label;
    Ok(())
}

fn map_platform_error(err: hv_platform_model::PlatformError) -> DatapathError {
    DatapathError::new(DatapathErrorKind::InvalidInput, err.message)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn plan_finds_independent_host_in_and_host_out_nics_from_platform_description() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_e1000_host_attach(&layout).expect("plan");
        assert_ne!(plan.host_in_bdf, plan.host_out_bdf);
        assert_eq!(layout.host_network.interfaces.len(), 2);
        let in_mmio =
            mmio_guest_phys_for_datapath_role(&layout, DATAPATH_ROLE_IN).expect("in mmio");
        let out_mmio =
            mmio_guest_phys_for_datapath_role(&layout, DATAPATH_ROLE_OUT).expect("out mmio");
        assert_eq!(in_mmio, 0xFEB0_0000);
        assert_eq!(out_mmio, 0xFEB2_0000);
        assert_ne!(in_mmio, out_mmio);
        let host_in = layout
            .host_network
            .interfaces
            .iter()
            .find(|interface| interface.bdf == plan.host_in_bdf)
            .expect("host in");
        let host_out = layout
            .host_network
            .interfaces
            .iter()
            .find(|interface| interface.bdf == plan.host_out_bdf)
            .expect("host out");
        assert_eq!(host_in.mmio_guest_phys, in_mmio);
        assert_eq!(host_out.mmio_guest_phys, out_mmio);
    }

    #[test]
    fn validate_discovered_outer_host_bars_accepts_reference_layout_addresses() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_e1000_host_attach(&layout).expect("plan");
        let discovered = DiscoveredOuterHostBars {
            host_in_bar0: 0xFEB0_0000,
            host_out_bar0: 0xFEB2_0000,
        };
        validate_discovered_outer_host_bars(&plan, &discovered).expect("valid");
    }

    #[test]
    fn validate_discovered_outer_host_bars_rejects_overlap() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let plan = plan_e1000_host_attach(&layout).expect("plan");
        let discovered = DiscoveredOuterHostBars {
            host_in_bar0: 0xFEB0_0000,
            host_out_bar0: 0xFEB0_1000,
        };
        assert!(validate_discovered_outer_host_bars(&plan, &discovered).is_err());
    }
}
