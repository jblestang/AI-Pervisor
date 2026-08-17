//! Publishes the hypervisor transfer blob into the UEFI configuration table.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;

use alloc::vec;
use alloc::vec::Vec;
use hv_boot_abi::HypervisorTransferHeader;
use uefi::boot::{self, AllocateType};
use uefi::cstr16;
use uefi::mem::memory_map::MemoryType;
use uefi::proto::media::file::{File, FileAttribute, FileMode, FileType};
use uefi::table::boot::LoadImageSource;
use uefi::{guid, Guid};

/// UEFI configuration table GUID for the hypervisor transfer header.
pub const HV_TRANSFER_TABLE_GUID: Guid = guid!("7502be2e-6d0d-4daf-8df4-0f89a2b3c4d5");

/// Copies `transfer` into runtime services memory and installs the configuration table entry.
pub fn publish_hypervisor_transfer(transfer: &[u8]) -> Result<(), &'static str> {
    if transfer.len() < size_of::<HypervisorTransferHeader>() {
        return Err("transfer blob shorter than header");
    }

    let page_count = pages_for_bytes(transfer.len())?;
    let pages = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::RUNTIME_SERVICES_DATA,
        page_count,
    )
    .map_err(|_| "failed to allocate runtime transfer memory")?;

    let destination = pages.as_ptr();
    copy_transfer_bytes(destination, transfer)?;

    let table_ptr = destination.cast::<c_void>();
    unsafe {
        boot::install_configuration_table(&HV_TRANSFER_TABLE_GUID, table_ptr)
            .map_err(|_| "failed to install hypervisor transfer configuration table")?;
    }

    Ok(())
}

/// Loads and starts the hypervisor UEFI image from the loader ESP.
pub fn chain_load_hypervisor() -> Result<(), &'static str> {
    let mut file_system = boot::get_image_file_system(boot::image_handle())
        .map_err(|_| "failed to open loader file system")?;
    let mut root = file_system
        .open_volume()
        .map_err(|_| "failed to open loader volume")?;
    let file = root
        .open(
            cstr16!("\\hv-hypervisor.efi"),
            FileMode::Read,
            FileAttribute::READ_ONLY,
        )
        .map_err(|_| "failed to open hypervisor image")?;
    let mut file = match file
        .into_type()
        .map_err(|_| "failed to inspect hypervisor image")?
    {
        FileType::Regular(regular) => regular,
        FileType::Dir(_) => return Err("hypervisor path is a directory"),
    };

    let image_bytes = read_regular_file(&mut file)?;
    let hypervisor_handle = boot::load_image(
        boot::image_handle(),
        LoadImageSource::FromBuffer {
            buffer: &image_bytes,
            file_path: None,
        },
    )
    .map_err(|_| "failed to load hypervisor image")?;

    boot::start_image(hypervisor_handle).map_err(|_| "hypervisor image start failed")?;
    Ok(())
}

fn read_regular_file(file: &mut uefi::proto::media::file::RegularFile) -> Result<Vec<u8>, &'static str> {
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut image = Vec::new();
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "failed to read hypervisor image bytes")?;
        if read == 0 {
            break;
        }
        let chunk = buffer
            .get(0..read)
            .ok_or("hypervisor image read truncated")?;
        image.extend_from_slice(chunk);
    }
    if image.is_empty() {
        return Err("hypervisor image is empty");
    }
    Ok(image)
}

fn pages_for_bytes(len: usize) -> Result<usize, &'static str> {
    let page_size = uefi::table::boot::PAGE_SIZE;
    len.checked_add(page_size - 1)
        .ok_or("transfer size overflow")?
        .checked_div(page_size)
        .ok_or("transfer page count overflow")
}

fn copy_transfer_bytes(destination: *mut u8, transfer: &[u8]) -> Result<(), &'static str> {
    let mut offset = 0usize;
    while offset < transfer.len() {
        let byte = transfer
            .get(offset)
            .copied()
            .ok_or("transfer copy truncated")?;
        unsafe {
            ptr::write_volatile(destination.add(offset), byte);
        }
        offset = offset.saturating_add(1);
    }
    Ok(())
}
