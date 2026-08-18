//! Portable Gate B boot validation and VMX init orchestration.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod boot;
mod error;
mod gate_c;
mod snapshot;
mod transfer;
mod vmx;

pub use boot::boot_check;
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
pub use snapshot::{
    layout_snapshot_from_platform_ir, platform_requirements_from_snapshot,
    requirements_snapshot_from_platform, static_platform_ir_from_layout_snapshot,
};
pub use transfer::{boot_from_transfer, boot_from_transfer_snapshot};
pub use vmx::{boot_check_and_init_vmx, boot_from_transfer_and_init_vmx, BootAndVmxResult};
