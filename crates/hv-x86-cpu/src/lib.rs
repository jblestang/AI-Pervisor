//! x86 CPU instruction seams for Gate C hardware bring-up.
//!
//! Host-only crate: validates CPU capabilities and records instruction disposition.
//! Default builds do not execute privileged VMX/EPT/VT-d instructions (CI-safe).

#![cfg_attr(not(test), no_std)]
#![allow(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod backends;
mod constants;
mod cpuid;
mod error;
#[cfg(feature = "datapath-guest-relay-measurement")]
mod guest_relay_measurement;
mod instructions;
mod resident;
mod resident_backends;
mod seams;
#[cfg(feature = "datapath-guest-relay-measurement")]
mod vmexit_ipc_relay;
#[cfg(feature = "datapath-guest-relay-measurement")]
mod vmexit_mmio_relay;
#[cfg(feature = "datapath-guest-relay-measurement")]
mod vmexit_relay_counter;
#[cfg(feature = "datapath-guest-relay-measurement")]
mod vmexit_relay_dispatch;

pub use backends::{CpuSeamEptBackend, CpuSeamVmxBackend, CpuSeamVtdBackend};
pub use constants::{
    CR4_VMXE_BIT, HV_X86_LIVE_INSTRUCTIONS_DISABLED, HV_X86_LIVE_INSTRUCTIONS_ENABLED,
    HV_X86_LIVE_INSTRUCTIONS_ENV, HV_X86_LIVE_VMXON_UNAVAILABLE, HV_X86_LIVE_VTD_UNAVAILABLE,
    VMCS_EPT_POINTER_FIELD, VMXON_REVISION_PREFIX_BYTES, X86_CPL_MASK, X86_RING_0,
};
pub use error::{CpuSeamError, CpuSeamErrorKind};
#[cfg(feature = "datapath-guest-relay-measurement")]
pub use guest_relay_measurement::{
    measure_in_vm_relay_frames_from_boot_infos, measure_in_vm_relay_from_context,
    publish_relay_measurement_page_authoritative, read_ipc_delivered_frames_from_guest,
    read_relay_frames_completed_from_boot_info_blob, read_relay_measurement_extension_from_guest,
    read_relay_measurement_extension_from_installed_boot_info, GuestBootInfoMeasurementSite,
    GuestRelayMeasurementContext, InVmRelayMeasurement, GUEST_RELAY_MEASUREMENT_VM_ID,
};
pub use instructions::{
    current_privilege_level, execute_ept_pointer_load, execute_invept_single_context,
    execute_vmcs_field_programming, execute_vmcs_prepare, execute_vmlaunch, execute_vmxon,
    execute_vtd_enable, firmware_live_execution_enabled, hypervisor_elapsed_tsc,
    last_vtd_enable_intent, live_execution_environment_ready, live_execution_runtime_enabled,
    read_timestamp_counter, read_vmx_basic_msr, vmx_revision_from_basic_msr, VtdEnableIntent,
    IA32_VMX_BASIC,
};
#[cfg(feature = "datapath-guests")]
pub use resident::install_guest_elf;
#[cfg(feature = "datapath-guest-relay-measurement")]
pub use resident::{
    install_e1000_mmio_state_page, install_relay_measurement_page, E1000MmioStatePageInstall,
    RelayMeasurementPageInstall,
};
pub use resident::{
    install_ept_tables, install_guest_image, install_vmcs_region, install_vmxon_region,
    resolve_vmxon_revision, MockPageAllocator, PageAllocator, VMCS_REGION_BYTES,
};
#[cfg(feature = "datapath-guest-live")]
pub use resident::{install_guest_elf_with_boot_info, GuestElfWithBootInfoInstall};
pub use resident_backends::{
    ResidentCpuSeamEptBackend, ResidentCpuSeamVmxBackend, ResidentCpuSeamVtdBackend,
};
#[cfg(feature = "datapath-guest-execution")]
pub use seams::{run_datapath_guest_execution_cpu_seam, DatapathGuestExecutionCpuSeamOutcome};
#[cfg(feature = "datapath-guest-throughput")]
pub use seams::{run_datapath_guest_throughput_cpu_seam, DatapathGuestThroughputCpuSeamOutcome};
#[cfg(feature = "datapath-live")]
pub use seams::{run_datapath_live_cpu_seam, DatapathLiveCpuSeamOutcome};
#[cfg(feature = "datapath-runtime")]
pub use seams::{run_datapath_runtime_cpu_seam, DatapathRuntimeCpuSeamOutcome};
pub use seams::{
    run_ept_pointer_cpu_seam, run_ept_pointer_reload_cpu_seam_batch, run_vmx_launch_cpu_seam,
    run_vmxon_cpu_seam, run_vtd_enable_cpu_seam, CpuInstructionDisposition, EptCpuSeamOutcome,
    EptPointerReloadCpuSeamOutcome, VmxCpuSeamOutcome, VmxLaunchCpuSeamOutcome, VtdCpuSeamOutcome,
};
#[cfg(feature = "datapath-guests")]
pub use seams::{run_multi_vmx_launch_cpu_seam, MultiVmxLaunchCpuSeamOutcome};
#[cfg(feature = "datapath-guest-relay-measurement")]
pub use vmexit_ipc_relay::{
    handle_ipc_vmexit, initialize_ipc_queue_backing, ipc_write_size_from_instruction_len,
    is_ipc_region_write_violation, VmexitIpcRelayConfig, VMCS_GUEST_RAX,
};
#[cfg(feature = "datapath-guest-relay-measurement")]
pub use vmexit_mmio_relay::{
    handle_e1000_mmio_vmexit, initialize_e1000_mmio_guest_view, is_e1000_mmio_write_violation,
    read_e1000_mmio_register, VmexitE1000MmioConfig,
};
#[cfg(feature = "datapath-guest-relay-measurement")]
pub use vmexit_relay_counter::{
    handle_relay_frame_vmexit, increment_relay_measurement_page_frames,
    is_measurement_page_write_violation, read_relay_measurement_page_frames,
    reset_relay_measurement_page_frames, validate_vmexit_relay_frame_count,
    VmexitRelayCounterConfig, VM_EXIT_REASON_EPT_VIOLATION, VM_EXIT_REASON_HLT,
};
#[cfg(feature = "datapath-guest-relay-measurement")]
pub use vmexit_relay_dispatch::{
    finalize_measurement_relay_frames, handle_relay_dispatch_vmexit,
    validate_vmexit_ipc_relay_events, validate_vmexit_mmio_relay_events, VmexitRelayDispatchConfig,
    VmexitRelayDispatchOutcome, VmexitRelayDispatchPlan,
};
