//! Firmware input collection via UEFI boot services.

use alloc::vec::Vec;

use hv_observation_types::CpuidSnapshot;
use hv_types::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};
use uefi::boot;
use uefi::mem::memory_map::MemoryMap;
use uefi::system::with_config_table;
use uefi::table::boot::MemoryType;
use uefi::table::cfg::{ACPI2_GUID, ACPI_GUID};

/// Collected firmware inputs for loader handoff.
pub struct FirmwareInputs {
    /// Raw UEFI memory map bytes.
    pub memory_map: Vec<u8>,
    /// Size of one memory map descriptor.
    pub memory_descriptor_size: usize,
    /// ACPI RSDP bytes copied from the configuration table.
    pub rsdp: Vec<u8>,
    /// CPUID snapshot collected at boot.
    pub cpuid: CpuidSnapshot,
    /// PCI devices discovered by firmware.
    pub pci_devices: Vec<PciBdf>,
}

/// Collects memory map, RSDP, and CPUID inputs from the running firmware.
pub fn collect_firmware_inputs() -> Result<FirmwareInputs, &'static str> {
    let memory = collect_memory_map()?;
    let rsdp = locate_rsdp()?;
    let cpuid = collect_cpuid_snapshot();
    let pci_devices = enumerate_pci_devices();
    Ok(FirmwareInputs {
        memory_map: memory.bytes,
        memory_descriptor_size: memory.descriptor_size,
        rsdp,
        cpuid,
        pci_devices,
    })
}

struct MemoryMapCapture {
    bytes: Vec<u8>,
    descriptor_size: usize,
}

fn collect_memory_map() -> Result<MemoryMapCapture, &'static str> {
    let owned = boot::memory_map(MemoryType::LOADER_DATA).map_err(|_| "failed to read memory map")?;
    Ok(MemoryMapCapture {
        bytes: owned.buffer().to_vec(),
        descriptor_size: owned.meta().desc_size,
    })
}

fn locate_rsdp() -> Result<Vec<u8>, &'static str> {
    with_config_table(|entries| {
        let entry = entries
            .iter()
            .find(|entry| entry.guid == ACPI2_GUID || entry.guid == ACPI_GUID)
            .ok_or("ACPI RSDP configuration table not found")?;
        read_rsdp_bytes(entry.address)
    })
}

fn read_rsdp_bytes(address: *const core::ffi::c_void) -> Result<Vec<u8>, &'static str> {
    if address.is_null() {
        return Err("ACPI RSDP pointer is null");
    }

    let rsdp_ptr = address.cast::<u8>();
    let mut rsdp = [0u8; 36];
    for (index, slot) in rsdp.iter_mut().enumerate() {
        *slot = unsafe { core::ptr::read_volatile(rsdp_ptr.add(index)) };
    }

    Ok(rsdp.to_vec())
}

fn collect_cpuid_snapshot() -> CpuidSnapshot {
    let (_, leaf1_ebx, leaf1_ecx, leaf1_edx) = cpuid(1, 0);
    let leaf80000007_edx = if cpuid(0x8000_0000, 0).0 >= 0x8000_0007 {
        Some(cpuid(0x8000_0007, 0).3)
    } else {
        None
    };
    let leaf80000008_ecx = if cpuid(0x8000_0000, 0).0 >= 0x8000_0008 {
        Some(cpuid(0x8000_0008, 0).2)
    } else {
        None
    };
    let (leaf480_ecx, leaf480_ebx) = if leaf1_ecx & (1 << 5) != 0 {
        let (_, ebx, ecx, _) = cpuid(0x480, 0);
        (Some(ecx), Some(ebx))
    } else {
        (None, None)
    };

    CpuidSnapshot {
        leaf1_ecx,
        leaf1_edx,
        leaf1_ebx,
        leaf480_ecx,
        leaf480_ebx,
        leaf80000007_edx,
        leaf80000008_ecx,
    }
}

/// Maximum PCI devices recorded during firmware enumeration.
const MAX_PCI_DEVICES: usize = 256;

fn enumerate_pci_devices() -> Vec<PciBdf> {
    let mut devices = Vec::new();
    for bus in 0u8..=255 {
        if !scan_pci_bus(bus, &mut devices) {
            break;
        }
        if devices.len() >= MAX_PCI_DEVICES {
            break;
        }
    }
    devices
}

fn scan_pci_bus(bus: u8, devices: &mut Vec<PciBdf>) -> bool {
    let mut bus_present = false;
    for device in 0u8..=31 {
        let header = pci_config_read32(bus, device, 0, 0);
        let vendor = header & 0xFFFF;
        if vendor == 0xFFFF {
            continue;
        }
        bus_present = true;
        let header_type = ((pci_config_read32(bus, device, 0, 0x0C) >> 16) & 0xFF) as u8;
        let multifunction = header_type & 0x80 != 0;
        let max_function = if multifunction { 7 } else { 0 };
        for function in 0u8..=max_function {
            let id = pci_config_read32(bus, device, function, 0);
            if (id & 0xFFFF) == 0xFFFF {
                continue;
            }
            devices.push(PciBdf::new(
                PciSegment::new(0),
                PciBus::new(bus),
                PciDevice::new(device),
                PciFunction::new(function),
            ));
            if devices.len() >= MAX_PCI_DEVICES {
                return bus_present;
            }
        }
    }
    bus_present
}

fn pci_config_read32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = 0x8000_0000u32
        | u32::from(bus) << 16
        | u32::from(device & 0x1F) << 11
        | u32::from(function & 0x7) << 8
        | u32::from(offset & 0xFC);
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") 0xCF8u16,
            in("eax") address,
            options(nomem, nostack, preserves_flags)
        );
        let mut value: u32;
        core::arch::asm!(
            "in eax, dx",
            in("dx") 0xCFCu16,
            out("eax") value,
            options(nomem, nostack, preserves_flags)
        );
        value
    }
}

fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let mut eax = leaf;
    let mut ecx = subleaf;
    let mut edx = 0u32;
    let mut ebx = 0u32;
    unsafe {
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "mov {ebx_out:e}, ebx",
            "pop rbx",
            ebx_out = lateout(reg) ebx,
            inout("eax") eax,
            inout("ecx") ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    (eax, ebx, ecx, edx)
}
