//! Shared assertions for hypervisor EFI integration tests.

#![allow(dead_code)]

use hv_hypervisor_efi::DatapathLiveBootMarkers;

/// Asserts datapath-live markers for validate-only host tests.
pub fn assert_datapath_live_markers_validate_only(markers: &DatapathLiveBootMarkers) {
    assert!(!markers.foundation.vmx_launch.vmlaunch_executed);
    #[cfg(not(feature = "datapath-runtime"))]
    {
        assert!(markers.ipc_forward_executed);
        assert!(markers.e1000_mmio_handled);
    }
    #[cfg(feature = "datapath-runtime")]
    {
        assert!(!markers.ipc_forward_executed);
        assert!(!markers.e1000_mmio_handled);
    }
}
