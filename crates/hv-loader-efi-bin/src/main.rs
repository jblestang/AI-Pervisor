//! UEFI loader application entry point.

#![no_main]
#![no_std]
#![allow(unsafe_code)]

extern crate alloc;

mod collect;
mod physical;
mod publish;

use collect::collect_firmware_inputs;
use hv_loader_efi::{build_hypervisor_transfer_from_entry, UefiLoaderParams};
use physical::{IdentityMappedPhysicalMemory, DEFAULT_MAX_PHYSICAL_ADDRESS};
use publish::{chain_load_hypervisor, publish_hypervisor_transfer};
use uefi::prelude::*;

include!(concat!(env!("OUT_DIR"), "/config_digest.rs"));

#[entry]
fn efi_main() -> Status {
    if uefi::helpers::init().is_err() {
        return Status::ABORTED;
    }

    match run_loader() {
        Ok(_) => Status::SUCCESS,
        Err(_) => Status::ABORTED,
    }
}

fn run_loader() -> Result<(), &'static str> {
    let firmware = collect_firmware_inputs()?;
    let memory = IdentityMappedPhysicalMemory::new(DEFAULT_MAX_PHYSICAL_ADDRESS);
    let (_handoff, transfer) = build_hypervisor_transfer_from_entry(
        UefiLoaderParams {
            config_digest: CONFIG_DIGEST,
            memory_map: firmware.memory_map,
            memory_descriptor_size: firmware.memory_descriptor_size,
            rsdp: firmware.rsdp,
            cpuid: firmware.cpuid,
            pci_devices: firmware.pci_devices,
        },
        &memory,
    )
    .map_err(|_| "loader handoff rejected inputs")?;

    publish_hypervisor_transfer(&transfer).map_err(|_| "failed to publish hypervisor transfer")?;
    chain_load_hypervisor().map_err(|_| "failed to chain-load hypervisor image")?;
    Ok(())
}
