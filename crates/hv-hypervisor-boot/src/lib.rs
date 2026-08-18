//! Portable Gate B boot validation and VMX init orchestration.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod boot;
mod error;
mod gate_c;
#[cfg(feature = "datapath-foundation")]
mod gate_d;
mod snapshot;
mod transfer;
mod vmx;

pub use boot::boot_check;
pub use hv_boot_abi::{
    GATE_D_BOOT_INFO_BUILT_MARKER, GATE_D_DATAPATH_FOUNDATION_MARKER, GATE_D_DATAPATH_LIVE_MARKER,
    GATE_D_E1000_MMIO_MARKER, GATE_D_IPC_FORWARD_MARKER, REAL_HW_BOOT_SUCCESS_MARKER,
    REAL_HW_EPT_EXECUTED_MARKER, REAL_HW_VMLAUNCH_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER,
};
pub use error::{BootCheckError, BootCheckErrorKind};
pub use gate_c::{
    boot_check_and_init_gate_c, boot_check_and_init_gate_c_programming,
    boot_from_transfer_and_init_gate_c, boot_from_transfer_and_init_gate_c_from_snapshots,
    boot_from_transfer_and_init_gate_c_programming,
    boot_from_transfer_and_init_gate_c_programming_from_snapshots, GateCInitResult,
    GateCProgrammingResult,
};
#[cfg(feature = "cpu-seams")]
pub use gate_c::{
    boot_check_and_init_gate_c_cpu_seam, boot_from_transfer_and_init_gate_c_cpu_seam,
    boot_from_transfer_and_init_gate_c_cpu_seam_from_snapshots, GateCCpuSeamResult,
};
#[cfg(feature = "live-execution")]
pub use gate_c::{
    boot_check_and_init_gate_c_live_execution,
    boot_from_transfer_and_init_gate_c_live_execution,
    boot_from_transfer_and_init_gate_c_live_execution_from_snapshots, GateCLiveExecutionResult,
};
#[cfg(feature = "real-hw-execution")]
pub use gate_c::{
    boot_check_and_init_gate_c_real_hw, boot_from_transfer_and_init_gate_c_real_hw,
    boot_from_transfer_and_init_gate_c_real_hw_from_snapshots, GateCRealHwResult,
};
#[cfg(feature = "vmx-launch")]
pub use gate_c::{
    boot_check_and_init_gate_c_vmx_launch, boot_from_transfer_and_init_gate_c_vmx_launch,
    boot_from_transfer_and_init_gate_c_vmx_launch_from_snapshots, GateCVmxLaunchResult,
};
#[cfg(feature = "datapath-foundation")]
pub use gate_d::{
    boot_check_and_init_gate_d_datapath_foundation,
    boot_from_transfer_and_init_gate_d_datapath_foundation,
    boot_from_transfer_and_init_gate_d_datapath_foundation_from_snapshots,
    GateDDatapathFoundationResult,
};
#[cfg(feature = "datapath-live")]
pub use gate_d::{
    boot_check_and_init_gate_d_datapath_live,
    boot_from_transfer_and_init_gate_d_datapath_live,
    boot_from_transfer_and_init_gate_d_datapath_live_from_snapshots,
    GateDDatapathLiveResult,
};
pub use snapshot::{
    layout_snapshot_from_platform_ir, platform_requirements_from_snapshot,
    requirements_snapshot_from_platform, static_platform_ir_from_layout_snapshot,
};
pub use transfer::{boot_from_transfer, boot_from_transfer_snapshot};
pub use vmx::{boot_check_and_init_vmx, boot_from_transfer_and_init_vmx, BootAndVmxResult};
