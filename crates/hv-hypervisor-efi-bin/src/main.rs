//! UEFI hypervisor application entry point.

#![no_main]
#![no_std]
#![allow(unsafe_code)]

extern crate alloc;

use hv_boot_abi::HypervisorTransferHeader;
#[cfg(feature = "real-hw-execution")]
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
    #[cfg(feature = "real-hw-execution")]
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

#[cfg(feature = "real-hw-execution")]
fn log_real_hw_markers(markers: &RealHwBootMarkers) {
    if markers.vmxon_executed {
        log::info!("{REAL_HW_VMXON_EXECUTED_MARKER}");
    }
    if markers.ept_executed {
        log::info!("{REAL_HW_EPT_EXECUTED_MARKER}");
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
