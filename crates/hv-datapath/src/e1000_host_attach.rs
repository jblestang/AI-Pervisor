//! Validates independent outer host e1000 bindings (IN and OUT not linked).
//!
//! Outer QEMU attaches `net_in` and `net_out` to separate host tap interfaces
//! described in platform configuration. Nested guests own in→mid→out relay over IPC.

use hv_platform_model::{
    bdf_for_datapath_role, StaticPlatformIR, DATAPATH_ROLE_IN, DATAPATH_ROLE_OUT,
};
use hv_types::PciBdf;

use crate::error::{DatapathError, DatapathErrorKind};

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
    if layout.host_network.enabled {
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
    }
}
