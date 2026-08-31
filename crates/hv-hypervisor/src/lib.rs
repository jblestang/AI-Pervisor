//! Hypervisor boot-path validation orchestration for Gate B (host re-exports).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

pub use hv_hypervisor_boot::{
    boot_check, boot_check_and_init_gate_c, boot_check_and_init_gate_c_programming,
    boot_check_and_init_vmx, boot_from_transfer, boot_from_transfer_and_init_gate_c,
    boot_from_transfer_and_init_gate_c_from_snapshots,
    boot_from_transfer_and_init_gate_c_programming,
    boot_from_transfer_and_init_gate_c_programming_from_snapshots, boot_from_transfer_and_init_vmx,
    boot_from_transfer_snapshot, layout_snapshot_from_platform_ir,
    platform_requirements_from_snapshot, requirements_snapshot_from_platform,
    static_platform_ir_from_layout_snapshot, BootAndVmxResult, BootCheckError, BootCheckErrorKind,
    GateCInitResult, GateCProgrammingResult,
};
#[cfg(feature = "cpu-seams")]
pub use hv_hypervisor_boot::{
    boot_check_and_init_gate_c_cpu_seam, boot_from_transfer_and_init_gate_c_cpu_seam,
    boot_from_transfer_and_init_gate_c_cpu_seam_from_snapshots, GateCCpuSeamResult,
};
#[cfg(feature = "live-execution")]
pub use hv_hypervisor_boot::{
    boot_check_and_init_gate_c_live_execution, boot_from_transfer_and_init_gate_c_live_execution,
    boot_from_transfer_and_init_gate_c_live_execution_from_snapshots, GateCLiveExecutionResult,
};
#[cfg(feature = "real-hw-execution")]
pub use hv_hypervisor_boot::{
    boot_check_and_init_gate_c_real_hw, boot_from_transfer_and_init_gate_c_real_hw,
    boot_from_transfer_and_init_gate_c_real_hw_from_snapshots, GateCRealHwResult,
};
