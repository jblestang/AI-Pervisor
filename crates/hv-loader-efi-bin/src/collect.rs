//! Firmware input collection via UEFI boot services.

use alloc::vec::Vec;

use hv_observation_types::CpuidSnapshot;
use hv_types::PciBdf;
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
    /// PCI devices discovered by firmware (Phase 7: not yet enumerated).
    pub pci_devices: Vec<PciBdf>,
}

/// Collects memory map, RSDP, and CPUID inputs from the running firmware.
pub fn collect_firmware_inputs() -> Result<FirmwareInputs, &'static str> {
    let memory = collect_memory_map()?;
    let rsdp = locate_rsdp()?;
    let cpuid = collect_cpuid_snapshot();
    Ok(FirmwareInputs {
        memory_map: memory.bytes,
        memory_descriptor_size: memory.descriptor_size,
        rsdp,
        cpuid,
        pci_devices: Vec::new(),
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

fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let mut eax = leaf;
    let mut ecx = subleaf;
    let mut edx = 0u32;
    let mut ebx = 0u32;
    unsafe {
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "mov {ebx_out}, ebx",
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
