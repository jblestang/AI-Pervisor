//! Guest datapath planning for IPC and device MMIO descriptors.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod benchmark;
mod compromised;
mod constants;
mod e1000;
mod error;
mod forward;
mod guest_relay_live;
mod guest_runtime;
mod guest_throughput;
mod ipc;
mod plan;
mod runtime;
mod topology;

pub use guest_relay_live::{
    apply_live_guest_throughput_benchmark, guest_throughput_result_with_live_relay,
    live_measurement_completed_for_execution, run_sustained_guest_relay_benchmark,
    validate_sustained_host_relay_benchmark, GUEST_RELAY_BENCHMARK_FRAMES,
};
pub use guest_throughput::{
    apply_guest_throughput_disposition, guest_throughput_disposition_for_seam,
    run_mock_guest_throughput_benchmark, GuestThroughputBenchmarkResult,
    GuestThroughputDisposition,
};
pub use guest_runtime::{
    apply_runtime_disposition, runtime_disposition_for_guest_execution_seam,
    run_guest_datapath_runtime, DatapathRuntimeDisposition, DatapathRuntimeOutcome,
    GuestDatapathRuntime, GUEST_DATAPATH_IPC_HOPS,
};
pub use benchmark::{
    compute_benchmark_run_stats, elapsed_nanos_from_tsc, mock_throughput_mbit,
    run_mock_datapath_benchmark, throughput_mbit_from_frames, DatapathBenchmarkConfig,
    DatapathBenchmarkResult, DatapathBenchmarkRunStats, BENCHMARK_MEASUREMENT_SECS,
    BENCHMARK_MIN_RUNS, BENCHMARK_WARMUP_SECS, TARGET_THROUGHPUT_MBIT_PER_SEC,
};
pub use compromised::{
    apply_compromised_guest_write, enforce_forward_integrity, scan_ipc_queue_integrity,
    CompromisedGuestAction, E1000Partition, IpcChannelSelector, REFERENCE_COMPROMISED_SCENARIOS,
};
pub use constants::{
    E1000_MMIO_GUEST_PHYS_BASE, E1000_MMIO_GUEST_PHYS_STRIDE, E1000_MMIO_SIZE_BYTES,
    REFERENCE_IPC_CHAN_A_GUEST_PHYS, REFERENCE_IPC_CHAN_B_GUEST_PHYS, REFERENCE_IPC_SHARED_BYTES,
    RELAY_MEASUREMENT_PAGE_BYTES, RELAY_MEASUREMENT_PAGE_GUEST_PHYS,
};
pub use e1000::{
    handle_e1000_mmio_read, handle_e1000_mmio_write, E1000MmioState, E1000_REG_RDH, E1000_REG_RDT,
    E1000_REG_TDH, E1000_REG_TDT,
};
pub use error::{DatapathError, DatapathErrorKind};
pub use topology::{DatapathForwardPlan, IpcChannelRuntime};
pub use forward::{
    forward_frame_in_mid_out, forward_synthetic_frame, is_compromised_action_blocked,
    plan_datapath_forward, run_reference_compromised_scenarios, SYNTHETIC_FRAME_PAYLOAD,
};
pub use ipc::{
    queue_storage_bytes, IpcQueueHeader, IpcQueueView, IpcSlotHeader, REFERENCE_IPC_QUEUE_SLOTS,
    REFERENCE_IPC_SLOT_SIZE_BYTES,
};
pub use plan::{
    plan_datapath_for_partition, plan_datapath_for_vm_id, plan_e1000_mmio_guest_phys,
    plan_out_ipc_consumer_guest_phys, plan_relay_measurement_page_gpa, DatapathPartitionPlan,
};
pub use runtime::{
    run_datapath_live_forward, DatapathLiveDisposition, DatapathLiveOutcome,
};
