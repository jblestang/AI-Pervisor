//! Portable hypervisor transfer verification for the UEFI entry path.

#![cfg_attr(not(test), no_std)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

extern crate alloc;

#[cfg(feature = "real-hw-execution")]
mod allocator;

mod error;

use hv_boot_abi::{LayoutSnapshot, RequirementsSnapshot};
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_c_from_snapshots;
use hv_types::SHA256_DIGEST_BYTES;

#[cfg(feature = "real-hw-execution")]
use hv_x86_cpu::{CpuInstructionDisposition, PageAllocator};
#[cfg(feature = "real-hw-execution")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_c_real_hw_from_snapshots;
#[cfg(feature = "vmx-launch")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_c_vmx_launch_from_snapshots;
#[cfg(feature = "datapath-foundation")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_d_datapath_foundation_from_snapshots;
#[cfg(feature = "datapath-live")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_d_datapath_live_from_snapshots;
#[cfg(feature = "datapath-malicious")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_d_datapath_malicious_from_snapshots;
#[cfg(feature = "datapath-guests")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_d_datapath_guests_from_snapshots;
#[cfg(feature = "datapath-benchmark")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_d_datapath_benchmark_from_snapshots;
#[cfg(feature = "datapath-runtime")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_d_datapath_runtime_from_snapshots;
#[cfg(feature = "datapath-guest-sources")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_d_datapath_guest_sources_from_snapshots;
#[cfg(feature = "datapath-guest-live")]
use hv_hypervisor_boot::boot_from_transfer_and_init_gate_d_datapath_guest_live_from_snapshots;

pub use error::{HypervisorEfiError, HypervisorEfiErrorKind};
pub use hv_hypervisor_boot::{
    GATE_D_BOOT_INFO_BUILT_MARKER, GATE_D_DATAPATH_FOUNDATION_MARKER, GATE_D_DATAPATH_LIVE_MARKER,
    GATE_D_DATAPATH_MALICIOUS_MARKER, GATE_D_DATAPATH_GUESTS_MARKER, GATE_D_DATAPATH_BENCHMARK_MARKER,
    GATE_D_BENCHMARK_TARGET_MET_MARKER, GATE_D_DATAPATH_RUNTIME_MARKER, GATE_D_GUEST_DATAPATH_FRAME_MARKER,
    GATE_D_E1000_MMIO_MARKER,
    GATE_D_GUEST_ELF_INSTALLED_MARKER, GATE_D_GUEST_SOURCE_ELF_MARKER,
    GATE_D_GUEST_BOOT_INFO_INSTALLED_MARKER, GATE_D_IPC_FORWARD_MARKER,
    GATE_D_IPC_INTEGRITY_MARKER,
    GATE_D_MULTI_VMLAUNCH_MARKER, REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER,
    REAL_HW_VMLAUNCH_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER,
};
pub use hv_guest_boot::GUEST_SMOKE_RUNNING_MARKER;

#[cfg(feature = "real-hw-execution")]
pub use allocator::UefiPageAllocator;

/// REAL_HW boot outcome markers for serial-log verification.
#[cfg(feature = "real-hw-execution")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealHwBootMarkers {
    /// Whether VMXON was executed live.
    pub vmxon_executed: bool,
    /// Whether the EPT pointer was loaded live.
    pub ept_executed: bool,
}

/// VMX launch boot outcome markers for serial-log verification.
#[cfg(feature = "vmx-launch")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmxLaunchBootMarkers {
    /// REAL_HW boot markers from Gate C init.
    pub real_hw: RealHwBootMarkers,
    /// Whether VMLAUNCH was executed live.
    pub vmlaunch_executed: bool,
}

/// Gate D datapath foundation boot outcome markers for serial-log verification.
#[cfg(feature = "datapath-foundation")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathFoundationBootMarkers {
    /// VMX launch boot markers from Gate C init.
    pub vmx_launch: VmxLaunchBootMarkers,
    /// Whether guest boot info was built for all partitions.
    pub datapath_boot_infos_built: bool,
}

/// Gate D datapath live boot outcome markers for serial-log verification.
#[cfg(feature = "datapath-live")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathLiveBootMarkers {
    /// Datapath foundation boot markers.
    pub foundation: DatapathFoundationBootMarkers,
    /// Whether a synthetic IPC frame was forwarded in→mid→out.
    pub ipc_forward_executed: bool,
    /// Whether e1000 MMIO was handled on the live datapath path.
    pub e1000_mmio_handled: bool,
}

/// Gate D datapath malicious boot outcome markers for serial-log verification.
#[cfg(feature = "datapath-malicious")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathMaliciousBootMarkers {
    /// Datapath live boot markers.
    pub live: DatapathLiveBootMarkers,
    /// Whether clean IPC queues passed integrity scans.
    pub integrity_checks_passed: bool,
    /// Number of reference compromised-guest scenarios blocked.
    pub compromised_scenarios_blocked: u32,
}

/// Gate D datapath guests boot outcome markers for serial-log verification.
#[cfg(feature = "datapath-guests")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathGuestsBootMarkers {
    /// Datapath malicious boot markers.
    pub malicious: DatapathMaliciousBootMarkers,
    /// Number of reference guest ELF images installed.
    pub elf_images_installed: u32,
    /// Whether multi-partition VMLAUNCH seams completed for all reference partitions.
    pub multi_partition_vmlaunch: bool,
}

/// Gate D datapath benchmark boot outcome markers for serial-log verification.
#[cfg(feature = "datapath-benchmark")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathBenchmarkBootMarkers {
    /// Datapath guests boot markers.
    pub guests: DatapathGuestsBootMarkers,
    /// Whether the official 200 Mbit/s benchmark target was met.
    pub benchmark_target_met: bool,
    /// Minimum observed throughput across benchmark runs (Mbit/s).
    pub benchmark_min_mbit_per_sec: u64,
}

/// Gate D datapath runtime boot outcome markers for serial-log verification.
#[cfg(feature = "datapath-runtime")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathRuntimeBootMarkers {
    /// Datapath benchmark boot markers.
    pub benchmark: DatapathBenchmarkBootMarkers,
    /// Whether a guest-driven frame was forwarded in→mid→out.
    pub guest_datapath_frame_forwarded: bool,
    /// Number of datapath-capable guest ELF images installed.
    pub datapath_elf_images_installed: u32,
    /// Whether VM-exit dispatch was validated for all partitions.
    pub vmexit_dispatch_validated: bool,
}

/// Gate D datapath guest-sources boot outcome markers for serial-log verification.
#[cfg(feature = "datapath-guest-sources")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathGuestSourcesBootMarkers {
    /// Datapath runtime boot markers using built `guests/` source-tree ELFs.
    pub runtime: DatapathRuntimeBootMarkers,
    /// Number of source-tree guest ELF images installed.
    pub guest_source_elfs_installed: u32,
}

/// Gate D datapath guest-live boot outcome markers for serial-log verification.
#[cfg(feature = "datapath-guest-live")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatapathGuestLiveBootMarkers {
    /// Datapath guest-sources boot markers.
    pub sources: DatapathGuestSourcesBootMarkers,
    /// Number of guest boot-info blobs installed and wired to VMCS RDI.
    pub guest_boot_infos_installed: u32,
}

/// Runs full Gate B validation and mock-backed Gate C init from a transfer blob.
pub fn boot_hypervisor_from_transfer(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
) -> Result<(), HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    boot_from_transfer_and_init_gate_c_from_snapshots(transfer, requirements, layout)
        .map(|_| ())
        .map_err(HypervisorEfiError::from)
}

/// Runs Gate B validation and REAL_HW Gate C init with resident page installation.
#[cfg(feature = "real-hw-execution")]
pub fn boot_hypervisor_from_transfer_real_hw<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<RealHwBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_c_real_hw_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    let vmxon_executed = result
        .live
        .cpu_seam
        .vmx_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    let ept_executed = result
        .live
        .cpu_seam
        .ept_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    Ok(RealHwBootMarkers {
        vmxon_executed,
        ept_executed,
    })
}

/// Runs Gate B validation and VMX launch Gate C init with resident page installation.
#[cfg(feature = "vmx-launch")]
pub fn boot_hypervisor_from_transfer_vmx_launch<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<VmxLaunchBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_c_vmx_launch_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    let vmxon_executed = result
        .real_hw
        .live
        .cpu_seam
        .vmx_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    let ept_executed = result
        .real_hw
        .live
        .cpu_seam
        .ept_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    let vmlaunch_executed = result
        .launch_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    Ok(VmxLaunchBootMarkers {
        real_hw: RealHwBootMarkers {
            vmxon_executed,
            ept_executed,
        },
        vmlaunch_executed,
    })
}

/// Runs Gate B validation and Gate D datapath foundation init with resident page installation.
#[cfg(feature = "datapath-foundation")]
pub fn boot_hypervisor_from_transfer_datapath_foundation<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<DatapathFoundationBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_d_datapath_foundation_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    let vmxon_executed = result
        .vmx_launch
        .real_hw
        .live
        .cpu_seam
        .vmx_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    let ept_executed = result
        .vmx_launch
        .real_hw
        .live
        .cpu_seam
        .ept_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    let vmlaunch_executed = result
        .vmx_launch
        .launch_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    Ok(DatapathFoundationBootMarkers {
        vmx_launch: VmxLaunchBootMarkers {
            real_hw: RealHwBootMarkers {
                vmxon_executed,
                ept_executed,
            },
            vmlaunch_executed,
        },
        datapath_boot_infos_built: result.partition_boot_infos.len() == 3,
    })
}

/// Runs Gate B validation and Gate D datapath live init with resident page installation.
#[cfg(feature = "datapath-live")]
pub fn boot_hypervisor_from_transfer_datapath_live<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<DatapathLiveBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_d_datapath_live_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    Ok(boot_hypervisor_from_transfer_datapath_live_markers(&result))
}

#[cfg(feature = "datapath-live")]
fn boot_hypervisor_from_transfer_datapath_live_markers(
    result: &hv_hypervisor_boot::GateDDatapathLiveResult,
) -> DatapathLiveBootMarkers {
    let vmxon_executed = result
        .foundation
        .vmx_launch
        .real_hw
        .live
        .cpu_seam
        .vmx_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    let ept_executed = result
        .foundation
        .vmx_launch
        .real_hw
        .live
        .cpu_seam
        .ept_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    let vmlaunch_executed = result
        .foundation
        .vmx_launch
        .launch_seam
        .as_ref()
        .is_some_and(|seam| seam.disposition == CpuInstructionDisposition::Executed);
    DatapathLiveBootMarkers {
        foundation: DatapathFoundationBootMarkers {
            vmx_launch: VmxLaunchBootMarkers {
                real_hw: RealHwBootMarkers {
                    vmxon_executed,
                    ept_executed,
                },
                vmlaunch_executed,
            },
            datapath_boot_infos_built: result.foundation.partition_boot_infos.len() == 3,
        },
        ipc_forward_executed: result
            .live_outcome
            .as_ref()
            .is_some_and(|outcome| outcome.synthetic_frame_forwarded),
        e1000_mmio_handled: result
            .live_outcome
            .as_ref()
            .is_some_and(|outcome| outcome.e1000_tx_observed),
    }
}

/// Runs Gate B validation and Gate D datapath malicious init with resident page installation.
#[cfg(feature = "datapath-malicious")]
pub fn boot_hypervisor_from_transfer_datapath_malicious<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<DatapathMaliciousBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_d_datapath_malicious_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    Ok(boot_hypervisor_from_transfer_datapath_malicious_markers(&result))
}

#[cfg(feature = "datapath-malicious")]
fn boot_hypervisor_from_transfer_datapath_malicious_markers(
    result: &hv_hypervisor_boot::GateDDatapathMaliciousResult,
) -> DatapathMaliciousBootMarkers {
    DatapathMaliciousBootMarkers {
        live: boot_hypervisor_from_transfer_datapath_live_markers(&result.live),
        integrity_checks_passed: result.integrity_checks_passed,
        compromised_scenarios_blocked: result.compromised_scenarios_blocked,
    }
}

/// Runs Gate B validation and Gate D datapath guests init with resident page installation.
#[cfg(feature = "datapath-guests")]
pub fn boot_hypervisor_from_transfer_datapath_guests<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<DatapathGuestsBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_d_datapath_guests_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    use hv_guest_boot::REFERENCE_GUEST_PARTITION_IDS;
    Ok(DatapathGuestsBootMarkers {
        malicious: boot_hypervisor_from_transfer_datapath_malicious_markers(&result.malicious),
        elf_images_installed: result.elf_images_installed,
        multi_partition_vmlaunch: result.multi_launch_seam.launches.len()
            == REFERENCE_GUEST_PARTITION_IDS.len(),
    })
}

/// Runs Gate B validation and Gate D datapath benchmark init with resident page installation.
#[cfg(feature = "datapath-benchmark")]
pub fn boot_hypervisor_from_transfer_datapath_benchmark<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<DatapathBenchmarkBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_d_datapath_benchmark_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    use hv_guest_boot::REFERENCE_GUEST_PARTITION_IDS;
    Ok(DatapathBenchmarkBootMarkers {
        guests: DatapathGuestsBootMarkers {
            malicious: boot_hypervisor_from_transfer_datapath_malicious_markers(
                &result.guests.malicious,
            ),
            elf_images_installed: result.guests.elf_images_installed,
            multi_partition_vmlaunch: result.guests.multi_launch_seam.launches.len()
                == REFERENCE_GUEST_PARTITION_IDS.len(),
        },
        benchmark_target_met: result.benchmark.target_met,
        benchmark_min_mbit_per_sec: result.benchmark.stats.min_mbit_per_sec,
    })
}

/// Runs Gate B validation and Gate D datapath runtime init with resident page installation.
#[cfg(feature = "datapath-runtime")]
pub fn boot_hypervisor_from_transfer_datapath_runtime<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<DatapathRuntimeBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_d_datapath_runtime_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    use hv_guest_boot::REFERENCE_GUEST_PARTITION_IDS;
    Ok(DatapathRuntimeBootMarkers {
        benchmark: DatapathBenchmarkBootMarkers {
            guests: DatapathGuestsBootMarkers {
                malicious: boot_hypervisor_from_transfer_datapath_malicious_markers(
                    &result.benchmark.guests.malicious,
                ),
                elf_images_installed: result.benchmark.guests.elf_images_installed,
                multi_partition_vmlaunch: result.benchmark.guests.multi_launch_seam.launches.len()
                    == REFERENCE_GUEST_PARTITION_IDS.len(),
            },
            benchmark_target_met: result.benchmark.benchmark.target_met,
            benchmark_min_mbit_per_sec: result.benchmark.benchmark.stats.min_mbit_per_sec,
        },
        guest_datapath_frame_forwarded: result.runtime.guest_frame_forwarded,
        datapath_elf_images_installed: result.datapath_elf_images_installed,
        vmexit_dispatch_validated: result.runtime.vmexit_dispatch_validated,
    })
}

/// Runs Gate B validation and Gate D datapath guest-sources init with resident page installation.
#[cfg(feature = "datapath-guest-sources")]
pub fn boot_hypervisor_from_transfer_datapath_guest_sources<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<DatapathGuestSourcesBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_d_datapath_guest_sources_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    Ok(boot_hypervisor_from_transfer_datapath_guest_sources_markers(&result))
}

/// Runs Gate B validation and Gate D datapath guest-live init with resident page installation.
#[cfg(feature = "datapath-guest-live")]
pub fn boot_hypervisor_from_transfer_datapath_guest_live<A: PageAllocator>(
    transfer: &[u8],
    expected_config_digest: &[u8; SHA256_DIGEST_BYTES],
    requirements: &RequirementsSnapshot,
    layout: &LayoutSnapshot,
    allocator: &mut A,
) -> Result<DatapathGuestLiveBootMarkers, HypervisorEfiError> {
    if requirements.config_digest != *expected_config_digest {
        return Err(HypervisorEfiError::new(
            HypervisorEfiErrorKind::Requirements,
            "requirements snapshot digest mismatch",
        ));
    }
    let result = boot_from_transfer_and_init_gate_d_datapath_guest_live_from_snapshots(
        transfer,
        requirements,
        layout,
        allocator,
    )
    .map_err(HypervisorEfiError::from)?;
    Ok(DatapathGuestLiveBootMarkers {
        sources: boot_hypervisor_from_transfer_datapath_guest_sources_markers(&result.sources),
        guest_boot_infos_installed: result.boot_infos_installed,
    })
}

#[cfg(feature = "datapath-guest-sources")]
fn boot_hypervisor_from_transfer_datapath_guest_sources_markers(
    result: &hv_hypervisor_boot::GateDDatapathGuestSourcesResult,
) -> DatapathGuestSourcesBootMarkers {
    use hv_guest_boot::REFERENCE_GUEST_PARTITION_IDS;
    let runtime = &result.runtime;
    DatapathGuestSourcesBootMarkers {
        runtime: DatapathRuntimeBootMarkers {
            benchmark: DatapathBenchmarkBootMarkers {
                guests: DatapathGuestsBootMarkers {
                    malicious: boot_hypervisor_from_transfer_datapath_malicious_markers(
                        &runtime.benchmark.guests.malicious,
                    ),
                    elf_images_installed: runtime.benchmark.guests.elf_images_installed,
                    multi_partition_vmlaunch: runtime.benchmark.guests.multi_launch_seam.launches.len()
                        == REFERENCE_GUEST_PARTITION_IDS.len(),
                },
                benchmark_target_met: runtime.benchmark.benchmark.target_met,
                benchmark_min_mbit_per_sec: runtime.benchmark.benchmark.stats.min_mbit_per_sec,
            },
            guest_datapath_frame_forwarded: runtime.runtime.guest_frame_forwarded,
            datapath_elf_images_installed: runtime.datapath_elf_images_installed,
            vmexit_dispatch_validated: runtime.runtime.vmexit_dispatch_validated,
        },
        guest_source_elfs_installed: runtime.datapath_elf_images_installed,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_hypervisor_boot::{
        layout_snapshot_from_platform_ir, requirements_snapshot_from_platform,
    };
    use hv_loader::{
        build_hypervisor_transfer, build_loader_handoff, encode_qemu_reference_firmware,
        LoaderHandoffInput,
    };
    use hv_observation_types::{
        CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
        CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
        CPUID_80000007_EDX_INVARIANT_TSC_BIT,
    };
    use hv_platform_model::plan_static_platform_ir;
    use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};

    fn reference_snapshots() -> (
        hv_boot_abi::RequirementsSnapshot,
        hv_boot_abi::LayoutSnapshot,
        [u8; SHA256_DIGEST_BYTES],
    ) {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let requirements = requirements_snapshot_from_platform(
            &compiled.requirements,
            compiled.digest.bytes,
            layout.hypervisor_reserve.host_phys.raw(),
            layout.hypervisor_reserve.size.bytes(),
        )
        .expect("snapshot");
        let layout_snapshot = layout_snapshot_from_platform_ir(&layout).expect("layout snapshot");
        (requirements, layout_snapshot, compiled.digest.bytes)
    }

    #[test]
    fn boot_hypervisor_from_transfer_accepts_reference_handoff() {
        let (requirements, layout, digest) = reference_snapshots();
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let firmware = encode_qemu_reference_firmware();
        let handoff = build_loader_handoff(
            &LoaderHandoffInput::with_default_descriptor_size(
                compiled.digest.bytes,
                {
                    let mut memory_map = vec![0u8; 48];
                    memory_map[0..4]
                        .copy_from_slice(&hv_boot_abi::EFI_MEMORY_CONVENTIONAL.to_le_bytes());
                    memory_map[24..32].copy_from_slice(&(2_097_152u64).to_le_bytes());
                    memory_map
                },
                firmware
                    .bytes
                    .get(0x1000..0x1000 + 36)
                    .expect("rsdp")
                    .to_vec(),
                CpuidSnapshot {
                    leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
                    leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
                    leaf1_ebx: (4 << 16) | 4,
                    leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
                    leaf80000008_ecx: Some(3),
                    leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
                    leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
                },
                vec![
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(3),
                        PciFunction::new(0),
                    ),
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(4),
                        PciFunction::new(0),
                    ),
                ],
            ),
            &firmware,
        )
        .expect("handoff");
        let transfer = build_hypervisor_transfer(&handoff).expect("transfer");
        boot_hypervisor_from_transfer(&transfer, &digest, &requirements, &layout).expect("boot");
    }

    #[test]
    fn boot_hypervisor_from_transfer_rejects_digest_mismatch() {
        let (mut requirements, layout, digest) = reference_snapshots();
        requirements.config_digest[0] ^= 0xFF;
        let err = boot_hypervisor_from_transfer(&[0u8; 64], &digest, &requirements, &layout)
            .expect_err("must fail");
        assert_eq!(err.kind, HypervisorEfiErrorKind::Requirements);
    }

    #[test]
    fn boot_hypervisor_from_transfer_rejects_invalid_blob() {
        let (requirements, layout, digest) = reference_snapshots();
        let err = boot_hypervisor_from_transfer(&[0xAA; 16], &digest, &requirements, &layout)
            .expect_err("must fail");
        assert_eq!(err.kind, HypervisorEfiErrorKind::BootInfo);
    }

    #[test]
    fn boot_hypervisor_from_transfer_rejects_layout_reserve_mismatch() {
        let (requirements, mut layout, digest) = reference_snapshots();
        layout.hypervisor_reserve_bytes ^= 1;
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let firmware = encode_qemu_reference_firmware();
        let handoff = build_loader_handoff(
            &LoaderHandoffInput::with_default_descriptor_size(
                compiled.digest.bytes,
                {
                    let mut memory_map = vec![0u8; 48];
                    memory_map[0..4]
                        .copy_from_slice(&hv_boot_abi::EFI_MEMORY_CONVENTIONAL.to_le_bytes());
                    memory_map[24..32].copy_from_slice(&(2_097_152u64).to_le_bytes());
                    memory_map
                },
                firmware
                    .bytes
                    .get(0x1000..0x1000 + 36)
                    .expect("rsdp")
                    .to_vec(),
                CpuidSnapshot {
                    leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
                    leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
                    leaf1_ebx: (4 << 16) | 4,
                    leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
                    leaf80000008_ecx: Some(3),
                    leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
                    leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
                },
                vec![
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(3),
                        PciFunction::new(0),
                    ),
                    PciBdf::new(
                        PciSegment::new(0),
                        PciBus::new(0),
                        PciDevice::new(4),
                        PciFunction::new(0),
                    ),
                ],
            ),
            &firmware,
        )
        .expect("handoff");
        let transfer = build_hypervisor_transfer(&handoff).expect("transfer");
        let err = boot_hypervisor_from_transfer(&transfer, &digest, &requirements, &layout)
            .expect_err("must fail");
        assert_eq!(err.kind, HypervisorEfiErrorKind::Platform);
    }

    #[test]
    fn hypervisor_efi_error_from_boot_check_maps_all_kinds() {
        use hv_hypervisor_boot::{BootCheckError, BootCheckErrorKind};
        let boot_abi: HypervisorEfiError =
            BootCheckError::new(BootCheckErrorKind::BootAbi, "boot").into();
        assert_eq!(boot_abi.kind, HypervisorEfiErrorKind::BootInfo);
        let observation: HypervisorEfiError =
            BootCheckError::new(BootCheckErrorKind::Observation, "obs").into();
        assert_eq!(observation.kind, HypervisorEfiErrorKind::Observation);
        let platform: HypervisorEfiError =
            BootCheckError::new(BootCheckErrorKind::Platform, "plat").into();
        assert_eq!(platform.kind, HypervisorEfiErrorKind::Platform);
        assert!(platform.to_string().contains("Platform"));
    }

    #[test]
    fn hypervisor_efi_error_from_boot_abi_error() {
        let err: HypervisorEfiError = hv_boot_abi::BootError::new(
            hv_boot_abi::BootErrorKind::Parse,
            "bad transfer",
        )
        .into();
        assert_eq!(err.kind, HypervisorEfiErrorKind::Transfer);
    }
}
