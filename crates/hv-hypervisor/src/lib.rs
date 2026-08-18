//! Hypervisor boot-path validation orchestration for Gate B (host re-exports).

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

pub use hv_hypervisor_boot::{
    boot_check, boot_check_and_init_gate_c, boot_check_and_init_vmx, boot_from_transfer,
    boot_from_transfer_and_init_gate_c, boot_from_transfer_and_init_vmx, boot_from_transfer_snapshot,
    platform_requirements_from_snapshot, requirements_snapshot_from_platform, BootAndVmxResult,
    BootCheckError, BootCheckErrorKind, GateCInitResult,
};
