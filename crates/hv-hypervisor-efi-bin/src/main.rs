//! UEFI hypervisor application entry point.

#![no_main]
#![no_std]
#![allow(unsafe_code)]

extern crate alloc;

use hv_boot_abi::HypervisorTransferHeader;
#[cfg(any(feature = "datapath-runtime", feature = "datapath-guest-sources"))]
use hv_guest_boot::GUEST_DATAPATH_CAPABLE_MARKER;
#[cfg(feature = "datapath-guest-sources")]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_datapath_guest_sources, DatapathBenchmarkBootMarkers,
    DatapathFoundationBootMarkers, DatapathGuestSourcesBootMarkers, DatapathGuestsBootMarkers,
    DatapathLiveBootMarkers, DatapathMaliciousBootMarkers, DatapathRuntimeBootMarkers,
    GATE_D_BENCHMARK_TARGET_MET_MARKER, GATE_D_BOOT_INFO_BUILT_MARKER, GATE_D_DATAPATH_BENCHMARK_MARKER,
    GATE_D_DATAPATH_GUESTS_MARKER, GATE_D_DATAPATH_LIVE_MARKER, GATE_D_DATAPATH_MALICIOUS_MARKER,
    GATE_D_DATAPATH_RUNTIME_MARKER, GATE_D_E1000_MMIO_MARKER, GATE_D_GUEST_DATAPATH_FRAME_MARKER,
    GATE_D_GUEST_ELF_INSTALLED_MARKER, GATE_D_GUEST_SOURCE_ELF_MARKER, GATE_D_IPC_FORWARD_MARKER,
    GATE_D_IPC_INTEGRITY_MARKER, GATE_D_MULTI_VMLAUNCH_MARKER, RealHwBootMarkers,
    VmxLaunchBootMarkers, REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER,
    REAL_HW_VMLAUNCH_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(all(feature = "datapath-runtime", not(feature = "datapath-guest-sources")))]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_datapath_runtime, DatapathBenchmarkBootMarkers,
    DatapathFoundationBootMarkers, DatapathGuestsBootMarkers, DatapathLiveBootMarkers,
    DatapathMaliciousBootMarkers, DatapathRuntimeBootMarkers, GATE_D_BENCHMARK_TARGET_MET_MARKER,
    GATE_D_BOOT_INFO_BUILT_MARKER, GATE_D_DATAPATH_BENCHMARK_MARKER, GATE_D_DATAPATH_GUESTS_MARKER,
    GATE_D_DATAPATH_LIVE_MARKER, GATE_D_DATAPATH_MALICIOUS_MARKER, GATE_D_DATAPATH_RUNTIME_MARKER,
    GATE_D_E1000_MMIO_MARKER, GATE_D_GUEST_DATAPATH_FRAME_MARKER, GATE_D_GUEST_ELF_INSTALLED_MARKER,
    GATE_D_IPC_FORWARD_MARKER, GATE_D_IPC_INTEGRITY_MARKER, GATE_D_MULTI_VMLAUNCH_MARKER,
    RealHwBootMarkers, VmxLaunchBootMarkers, REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER,
    REAL_HW_VMLAUNCH_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(all(feature = "datapath-benchmark", not(feature = "datapath-runtime")))]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_datapath_benchmark, DatapathBenchmarkBootMarkers,
    DatapathFoundationBootMarkers, DatapathGuestsBootMarkers, DatapathLiveBootMarkers,
    DatapathMaliciousBootMarkers, GATE_D_BENCHMARK_TARGET_MET_MARKER, GATE_D_BOOT_INFO_BUILT_MARKER,
    GATE_D_DATAPATH_BENCHMARK_MARKER, GATE_D_DATAPATH_GUESTS_MARKER, GATE_D_DATAPATH_LIVE_MARKER,
    GATE_D_DATAPATH_MALICIOUS_MARKER, GATE_D_E1000_MMIO_MARKER, GATE_D_GUEST_ELF_INSTALLED_MARKER,
    GATE_D_IPC_FORWARD_MARKER, GATE_D_IPC_INTEGRITY_MARKER, GATE_D_MULTI_VMLAUNCH_MARKER,
    RealHwBootMarkers, VmxLaunchBootMarkers, REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER,
    REAL_HW_VMLAUNCH_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(all(feature = "datapath-guests", not(any(feature = "datapath-benchmark", feature = "datapath-runtime"))))]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_datapath_guests, DatapathFoundationBootMarkers,
    DatapathGuestsBootMarkers, DatapathLiveBootMarkers, DatapathMaliciousBootMarkers,
    GATE_D_BOOT_INFO_BUILT_MARKER, GATE_D_DATAPATH_GUESTS_MARKER, GATE_D_DATAPATH_LIVE_MARKER,
    GATE_D_DATAPATH_MALICIOUS_MARKER, GATE_D_E1000_MMIO_MARKER, GATE_D_GUEST_ELF_INSTALLED_MARKER,
    GATE_D_IPC_FORWARD_MARKER, GATE_D_IPC_INTEGRITY_MARKER, GATE_D_MULTI_VMLAUNCH_MARKER,
    RealHwBootMarkers, VmxLaunchBootMarkers, REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER,
    REAL_HW_VMLAUNCH_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(all(feature = "datapath-malicious", not(any(feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime"))))]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_datapath_malicious, DatapathFoundationBootMarkers,
    DatapathLiveBootMarkers, DatapathMaliciousBootMarkers, GATE_D_BOOT_INFO_BUILT_MARKER,
    GATE_D_DATAPATH_LIVE_MARKER, GATE_D_DATAPATH_MALICIOUS_MARKER, GATE_D_E1000_MMIO_MARKER,
    GATE_D_IPC_FORWARD_MARKER, GATE_D_IPC_INTEGRITY_MARKER, RealHwBootMarkers,
    VmxLaunchBootMarkers, REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER,
    REAL_HW_VMLAUNCH_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(all(feature = "datapath-live", not(any(feature = "datapath-malicious", feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime"))))]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_datapath_live, DatapathFoundationBootMarkers,
    DatapathLiveBootMarkers, GATE_D_BOOT_INFO_BUILT_MARKER, GATE_D_DATAPATH_LIVE_MARKER,
    GATE_D_E1000_MMIO_MARKER, GATE_D_IPC_FORWARD_MARKER, RealHwBootMarkers, VmxLaunchBootMarkers,
    REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER, REAL_HW_VMLAUNCH_EXECUTED_MARKER,
    REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(all(feature = "datapath-foundation", not(any(feature = "datapath-live", feature = "datapath-malicious", feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime"))))]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_datapath_foundation, DatapathFoundationBootMarkers,
    GATE_D_BOOT_INFO_BUILT_MARKER, GATE_D_DATAPATH_FOUNDATION_MARKER, RealHwBootMarkers,
    VmxLaunchBootMarkers, REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER,
    REAL_HW_VMLAUNCH_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(all(feature = "vmx-launch", not(feature = "datapath-foundation")))]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_vmx_launch, RealHwBootMarkers, VmxLaunchBootMarkers,
    REAL_HW_BOOT_SUCCESS_MARKER, REAL_HW_EPT_EXECUTED_MARKER, REAL_HW_VMLAUNCH_EXECUTED_MARKER,
    REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(all(feature = "real-hw-execution", not(feature = "vmx-launch")))]
use hv_hypervisor_efi::{
    boot_hypervisor_from_transfer_real_hw, RealHwBootMarkers, REAL_HW_BOOT_SUCCESS_MARKER,
    REAL_HW_EPT_EXECUTED_MARKER, REAL_HW_VMXON_EXECUTED_MARKER, UefiPageAllocator,
};
#[cfg(not(feature = "real-hw-execution"))]
use hv_hypervisor_efi::boot_hypervisor_from_transfer;
use uefi::prelude::*;
use uefi::system::with_config_table;
use uefi::table::cfg::ConfigTableEntry;
use uefi::{guid, Guid};

include!(concat!(env!("OUT_DIR"), "/embedded_config.rs"));

/// UEFI configuration table GUID for the hypervisor transfer header.
const HV_TRANSFER_TABLE_GUID: Guid = guid!("7502be2e-6d0d-4daf-8df4-0f89a2b3c4d5");

#[entry]
fn efi_main() -> Status {
    if uefi::helpers::init().is_err() {
        return Status::ABORTED;
    }

    match run_hypervisor() {
        Ok(_) => Status::SUCCESS,
        Err(_) => Status::ABORTED,
    }
}

fn run_hypervisor() -> Result<(), &'static str> {
    let transfer = locate_transfer_blob()?;
    #[cfg(feature = "datapath-guest-sources")]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_datapath_guest_sources(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor Gate D datapath guest-sources boot failed: {err}");
            "hypervisor Gate D datapath guest-sources boot failed"
        })?;
        log_datapath_guest_sources_markers(&markers);
        log::info!("{GATE_D_GUEST_SOURCE_ELF_MARKER}");
    }
    #[cfg(all(feature = "datapath-runtime", not(feature = "datapath-guest-sources")))]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_datapath_runtime(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor Gate D datapath runtime boot failed: {err}");
            "hypervisor Gate D datapath runtime boot failed"
        })?;
        log_datapath_runtime_markers(&markers);
        log::info!("{GATE_D_DATAPATH_RUNTIME_MARKER}");
    }
    #[cfg(all(feature = "datapath-benchmark", not(feature = "datapath-runtime")))]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_datapath_benchmark(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor Gate D datapath benchmark boot failed: {err}");
            "hypervisor Gate D datapath benchmark boot failed"
        })?;
        log_datapath_benchmark_markers(&markers);
        log::info!("{GATE_D_DATAPATH_BENCHMARK_MARKER}");
    }
    #[cfg(all(feature = "datapath-guests", not(any(feature = "datapath-benchmark", feature = "datapath-runtime"))))]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_datapath_guests(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor Gate D datapath guests boot failed: {err}");
            "hypervisor Gate D datapath guests boot failed"
        })?;
        log_datapath_guests_markers(&markers);
        log::info!("{GATE_D_DATAPATH_GUESTS_MARKER}");
    }
    #[cfg(all(feature = "datapath-malicious", not(any(feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime"))))]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_datapath_malicious(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor Gate D datapath malicious boot failed: {err}");
            "hypervisor Gate D datapath malicious boot failed"
        })?;
        log_datapath_malicious_markers(&markers);
        log::info!("{GATE_D_DATAPATH_MALICIOUS_MARKER}");
    }
    #[cfg(all(feature = "datapath-live", not(any(feature = "datapath-malicious", feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime"))))]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_datapath_live(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor Gate D datapath live boot failed: {err}");
            "hypervisor Gate D datapath live boot failed"
        })?;
        log_datapath_live_markers(&markers);
        log::info!("{GATE_D_DATAPATH_LIVE_MARKER}");
    }
    #[cfg(all(feature = "datapath-foundation", not(any(feature = "datapath-live", feature = "datapath-malicious", feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime"))))]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_datapath_foundation(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor Gate D datapath foundation boot failed: {err}");
            "hypervisor Gate D datapath foundation boot failed"
        })?;
        log_datapath_foundation_markers(&markers);
        log::info!("{GATE_D_DATAPATH_FOUNDATION_MARKER}");
    }
    #[cfg(all(feature = "vmx-launch", not(feature = "datapath-foundation")))]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_vmx_launch(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor VMX launch boot failed: {err}");
            "hypervisor VMX launch boot and Gate C init failed"
        })?;
        log_vmx_launch_markers(&markers);
        log::info!("{REAL_HW_BOOT_SUCCESS_MARKER}");
    }
    #[cfg(all(feature = "real-hw-execution", not(feature = "vmx-launch")))]
    {
        let mut allocator = UefiPageAllocator::new();
        let markers = boot_hypervisor_from_transfer_real_hw(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
            &mut allocator,
        )
        .map_err(|err| {
            log::error!("hypervisor REAL_HW boot failed: {err}");
            "hypervisor REAL_HW boot and Gate C init failed"
        })?;
        log_real_hw_markers(&markers);
        log::info!("{REAL_HW_BOOT_SUCCESS_MARKER}");
    }
    #[cfg(not(feature = "real-hw-execution"))]
    {
        boot_hypervisor_from_transfer(
            transfer,
            &CONFIG_DIGEST,
            &REQUIREMENTS_SNAPSHOT,
            &LAYOUT_SNAPSHOT,
        )
        .map_err(|err| {
            log::error!("hypervisor boot failed: {err}");
            "hypervisor boot and Gate C init failed"
        })?;
        log::info!("hypervisor Gate C boot succeeded");
    }
    Ok(())
}

#[cfg(all(feature = "real-hw-execution", not(feature = "vmx-launch")))]
fn log_real_hw_markers(markers: &RealHwBootMarkers) {
    if markers.vmxon_executed {
        log::info!("{REAL_HW_VMXON_EXECUTED_MARKER}");
    }
    if markers.ept_executed {
        log::info!("{REAL_HW_EPT_EXECUTED_MARKER}");
    }
}

#[cfg(any(feature = "datapath-foundation", feature = "datapath-live", feature = "datapath-malicious", feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime", feature = "datapath-guest-sources"))]
fn log_datapath_foundation_markers(markers: &DatapathFoundationBootMarkers) {
    log_vmx_launch_markers(&markers.vmx_launch);
    if markers.datapath_boot_infos_built {
        log::info!("{GATE_D_BOOT_INFO_BUILT_MARKER}");
    }
}

#[cfg(any(feature = "datapath-live", feature = "datapath-malicious", feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime", feature = "datapath-guest-sources"))]
fn log_datapath_live_markers(markers: &DatapathLiveBootMarkers) {
    log_datapath_foundation_markers(&markers.foundation);
    if markers.ipc_forward_executed {
        log::info!("{GATE_D_IPC_FORWARD_MARKER}");
    }
    if markers.e1000_mmio_handled {
        log::info!("{GATE_D_E1000_MMIO_MARKER}");
    }
}

#[cfg(any(feature = "datapath-malicious", feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime", feature = "datapath-guest-sources"))]
fn log_datapath_malicious_markers(markers: &DatapathMaliciousBootMarkers) {
    log_datapath_live_markers(&markers.live);
    if markers.integrity_checks_passed {
        log::info!("{GATE_D_IPC_INTEGRITY_MARKER}");
    }
}

#[cfg(any(feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime", feature = "datapath-guest-sources"))]
fn log_datapath_guests_markers(markers: &DatapathGuestsBootMarkers) {
    log_datapath_malicious_markers(&markers.malicious);
    if markers.elf_images_installed == 3 {
        log::info!("{GATE_D_GUEST_ELF_INSTALLED_MARKER}");
    }
    if markers.multi_partition_vmlaunch {
        log::info!("{GATE_D_MULTI_VMLAUNCH_MARKER}");
    }
}

#[cfg(any(feature = "datapath-benchmark", feature = "datapath-runtime", feature = "datapath-guest-sources"))]
fn log_datapath_benchmark_markers(markers: &DatapathBenchmarkBootMarkers) {
    log_datapath_guests_markers(&markers.guests);
    if markers.benchmark_target_met {
        log::info!("{GATE_D_BENCHMARK_TARGET_MET_MARKER}");
    }
}

#[cfg(any(feature = "datapath-runtime", feature = "datapath-guest-sources"))]
fn log_datapath_runtime_markers(markers: &DatapathRuntimeBootMarkers) {
    log_datapath_benchmark_markers(&markers.benchmark);
    if markers.datapath_elf_images_installed == 3 {
        log::info!("{GUEST_DATAPATH_CAPABLE_MARKER}");
    }
    if markers.guest_datapath_frame_forwarded {
        log::info!("{GATE_D_GUEST_DATAPATH_FRAME_MARKER}");
    }
}

#[cfg(feature = "datapath-guest-sources")]
fn log_datapath_guest_sources_markers(markers: &DatapathGuestSourcesBootMarkers) {
    log_datapath_runtime_markers(&markers.runtime);
    if markers.guest_source_elfs_installed == 3 {
        log::info!("{GATE_D_GUEST_SOURCE_ELF_MARKER}");
    }
}

#[cfg(any(feature = "vmx-launch", feature = "datapath-foundation", feature = "datapath-live", feature = "datapath-malicious", feature = "datapath-guests", feature = "datapath-benchmark", feature = "datapath-runtime", feature = "datapath-guest-sources"))]
fn log_vmx_launch_markers(markers: &VmxLaunchBootMarkers) {
    if markers.real_hw.vmxon_executed {
        log::info!("{REAL_HW_VMXON_EXECUTED_MARKER}");
    }
    if markers.real_hw.ept_executed {
        log::info!("{REAL_HW_EPT_EXECUTED_MARKER}");
    }
    if markers.vmlaunch_executed {
        log::info!("{REAL_HW_VMLAUNCH_EXECUTED_MARKER}");
    }
}

fn locate_transfer_blob() -> Result<&'static [u8], &'static str> {
    with_config_table(transfer_from_config_table)
        .ok_or("hypervisor transfer configuration table not found")
}

fn transfer_from_config_table(entries: &[ConfigTableEntry]) -> Option<&'static [u8]> {
    for entry in entries {
        if entry.guid != HV_TRANSFER_TABLE_GUID {
            continue;
        }
        if entry.address.is_null() {
            return None;
        }
        let header_ptr = entry.address.cast::<HypervisorTransferHeader>();
        let header = unsafe { core::ptr::read_volatile(header_ptr) };
        if header.magic != hv_boot_abi::TRANSFER_MAGIC {
            return None;
        }
        let published_alloc_size = header.published_alloc_size as usize;
        let total_size = header.total_size as usize;
        if published_alloc_size < total_size {
            return None;
        }
        if published_alloc_size < core::mem::size_of::<HypervisorTransferHeader>() {
            return None;
        }
        return Some(unsafe {
            core::slice::from_raw_parts(entry.address.cast::<u8>(), published_alloc_size)
        });
    }
    None
}
