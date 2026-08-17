//! CPUID snapshot interpretation for runtime platform observation.

use hv_boot_abi::{
    DMAR_FLAG_INTR_REMAP, DMAR_FLAGS_OFFSET, DMAR_MIN_LENGTH, DMAR_SIGNATURE,
    EFI_MEMORY_CONVENTIONAL, UEFI_MEMORY_DESCRIPTOR_MIN_SIZE, UEFI_PAGE_SIZE, UefiMemoryDescriptor,
};
use hv_config_model::SUPPORTED_ARCH;
use hv_types::ByteSize;

use crate::cpuid_constants::{
    CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
    CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
    CPUID_80000007_EDX_INVARIANT_TSC_BIT, DEFAULT_PAGE_SIZES,
};
use crate::error::{PlatformError, PlatformErrorKind};
use crate::observed::ObservedPlatform;
use hv_types::PciBdf;

/// Raw CPUID leaves collected by the loader before hypervisor entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuidSnapshot {
    /// CPUID leaf 1 ECX register value.
    pub leaf1_ecx: u32,
    /// CPUID leaf 1 EDX register value.
    pub leaf1_edx: u32,
    /// CPUID leaf 1 EBX register value.
    pub leaf1_ebx: u32,
    /// CPUID leaf 0x8000_0007 EDX register value when available.
    pub leaf80000007_edx: Option<u32>,
    /// CPUID leaf 0x8000_0008 ECX register value when available.
    pub leaf80000008_ecx: Option<u32>,
    /// CPUID leaf 0x480 ECX register value when VMX is enabled.
    pub leaf480_ecx: Option<u32>,
    /// CPUID leaf 0x480 EBX register value when VMX is enabled.
    pub leaf480_ebx: Option<u32>,
}

impl CpuidSnapshot {
    /// Returns whether VMX is supported.
    pub fn vmx(&self) -> bool {
        bit_set(self.leaf1_ecx, CPUID_1_ECX_VMX_BIT)
    }

    /// Returns whether NX is supported.
    pub fn nx(&self) -> bool {
        bit_set(self.leaf1_edx, CPUID_1_EDX_NX_BIT)
    }

    /// Returns whether x2APIC is supported.
    pub fn x2apic(&self) -> bool {
        bit_set(self.leaf1_ecx, CPUID_1_ECX_X2APIC_BIT)
    }

    /// Returns whether invariant TSC is supported.
    pub fn invariant_tsc(&self) -> bool {
        self.leaf80000007_edx
            .is_some_and(|edx| bit_set(edx, CPUID_80000007_EDX_INVARIANT_TSC_BIT))
    }

    /// Returns whether EPT is supported.
    pub fn ept(&self) -> bool {
        self.leaf480_ecx
            .is_some_and(|ecx| bit_set(ecx, CPUID_480_ECX_EPT_BIT))
    }

    /// Returns whether VPID is supported.
    pub fn vpid(&self) -> bool {
        self.leaf480_ecx
            .is_some_and(|ecx| bit_set(ecx, CPUID_480_ECX_VPID_BIT))
    }

    /// Returns whether the VMX preemption timer is supported.
    pub fn vmx_preemption_timer(&self) -> bool {
        self.leaf480_ebx
            .is_some_and(|ebx| bit_set(ebx, CPUID_480_EBX_PREEMPTION_TIMER_BIT))
    }

    /// Returns the number of logical processors reported in leaf 1 EBX.
    pub fn logical_processors(&self) -> u32 {
        (self.leaf1_ebx >> 16) & 0xFF
    }

    /// Returns the number of physical cores per package when leaf 0x8000_0008 is available.
    pub fn cores_per_package(&self) -> Option<u32> {
        self.leaf80000008_ecx.map(|ecx| (ecx & 0xFF) + 1)
    }

    /// Returns whether SMT appears enabled from CPUID topology.
    pub fn smt_enabled(&self) -> bool {
        match self.cores_per_package() {
            Some(cores) => self.logical_processors() > cores,
            None => false,
        }
    }

    /// Returns the estimated physical core count.
    pub fn physical_cores(&self) -> u32 {
        self.cores_per_package()
            .unwrap_or_else(|| self.logical_processors().max(1))
    }
}

/// Inputs required to observe a platform at boot time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationInputs {
    /// CPUID snapshot collected by the loader.
    pub cpuid: CpuidSnapshot,
    /// Flattened ACPI table bytes collected by the loader (interim contract).
    pub acpi_tables: Vec<u8>,
    /// Raw UEFI memory map bytes.
    pub memory_map: Vec<u8>,
    /// Size of one UEFI memory map descriptor.
    pub memory_descriptor_size: usize,
    /// PCI devices discovered by firmware.
    pub pci_devices: Vec<PciBdf>,
}

/// Observes platform capabilities from firmware-provided boot inputs.
pub fn observe_platform(inputs: &ObservationInputs) -> Result<ObservedPlatform, PlatformError> {
    let ram_bytes = sum_conventional_ram(&inputs.memory_map, inputs.memory_descriptor_size)?;
    let (vtd, interrupt_remapping) = scan_acpi_capabilities(&inputs.acpi_tables)?;

    Ok(ObservedPlatform {
        arch: String::from(SUPPORTED_ARCH),
        vmx: inputs.cpuid.vmx(),
        ept: inputs.cpuid.ept(),
        vtd,
        physical_cores: inputs.cpuid.physical_cores(),
        ram_bytes,
        smt_enabled: inputs.cpuid.smt_enabled(),
        interrupt_remapping,
        x2apic: inputs.cpuid.x2apic(),
        invariant_tsc: inputs.cpuid.invariant_tsc(),
        vpid: inputs.cpuid.vpid(),
        vmx_preemption_timer: inputs.cpuid.vmx_preemption_timer(),
        nx: inputs.cpuid.nx(),
        page_sizes: DEFAULT_PAGE_SIZES.to_vec(),
        pci_devices: inputs.pci_devices.clone(),
    })
}

fn bit_set(value: u32, bit: u32) -> bool {
    (value & (1 << bit)) != 0
}

fn sum_conventional_ram(map: &[u8], descriptor_size: usize) -> Result<ByteSize, PlatformError> {
    if descriptor_size < UEFI_MEMORY_DESCRIPTOR_MIN_SIZE {
        return Err(PlatformError::new(
            PlatformErrorKind::Observation,
            "memory descriptor size too small",
        ));
    }

    let mut total = 0u64;
    let mut offset = 0usize;
    while offset < map.len() {
        let end = offset
            .checked_add(descriptor_size)
            .ok_or_else(|| observation_error("memory map offset overflow"))?;
        if end > map.len() {
            break;
        }
        let descriptor_bytes = map.get(offset..end).ok_or_else(|| {
            observation_error("memory map descriptor slice unavailable")
        })?;
        let descriptor = UefiMemoryDescriptor::parse(descriptor_bytes).map_err(boot_to_observation)?;
        if descriptor.typ == EFI_MEMORY_CONVENTIONAL {
            let bytes = descriptor
                .number_of_pages
                .checked_mul(UEFI_PAGE_SIZE)
                .ok_or_else(|| observation_error("conventional memory size overflow"))?;
            total = total
                .checked_add(bytes)
                .ok_or_else(|| observation_error("conventional memory sum overflow"))?;
        }
        offset = end;
    }

    Ok(ByteSize::new(total))
}

fn scan_acpi_capabilities(tables: &[u8]) -> Result<(bool, bool), PlatformError> {
    let mut offset = 0usize;
    while offset < tables.len() {
        let header = tables
            .get(offset..offset + 8)
            .ok_or_else(|| observation_error("ACPI header truncated"))?;
        let length_bytes = tables
            .get(offset + 4..offset + 8)
            .ok_or_else(|| observation_error("ACPI length truncated"))?;
        let chunk: [u8; 4] = length_bytes.try_into().map_err(|_| {
            observation_error("ACPI length truncated")
        })?;
        let length = u32::from_le_bytes(chunk) as usize;
        if length == 0 {
            break;
        }
        let table_end = offset
            .checked_add(length)
            .ok_or_else(|| observation_error("ACPI scan offset overflow"))?;
        if table_end > tables.len() {
            return Err(observation_error("ACPI table exceeds provided buffer"));
        }

        if header.get(0..4).ok_or_else(|| observation_error("ACPI signature missing"))?
            == DMAR_SIGNATURE
        {
            if length < DMAR_MIN_LENGTH {
                return Err(observation_error("DMAR table shorter than minimum length"));
            }
            if table_end <= offset + DMAR_FLAGS_OFFSET {
                return Err(observation_error("DMAR flags unavailable"));
            }
            let flags = tables
                .get(offset + DMAR_FLAGS_OFFSET)
                .copied()
                .ok_or_else(|| observation_error("DMAR flags unavailable"))?;
            let interrupt_remapping = (flags & DMAR_FLAG_INTR_REMAP) != 0;
            return Ok((true, interrupt_remapping));
        }

        offset = table_end;
    }
    Ok((false, false))
}

fn boot_to_observation(err: hv_boot_abi::BootError) -> PlatformError {
    PlatformError::new(PlatformErrorKind::Observation, format!("{err}"))
}

fn observation_error(message: &'static str) -> PlatformError {
    PlatformError::new(PlatformErrorKind::Observation, message)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hv_boot_abi::encode_reference_dmar_with_intr_remap;
    use hv_types::{PciBus, PciDevice, PciFunction, PciSegment};

    fn reference_cpuid() -> CpuidSnapshot {
        CpuidSnapshot {
            leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
            leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
            leaf1_ebx: (4 << 16) | 4,
            leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
            leaf80000008_ecx: Some(3),
            leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
            leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
        }
    }

    fn encode_descriptor(descriptor: UefiMemoryDescriptor) -> Vec<u8> {
        let mut bytes = vec![0u8; 48];
        bytes[0..4].copy_from_slice(&descriptor.typ.to_le_bytes());
        bytes[8..16].copy_from_slice(&descriptor.physical_start.to_le_bytes());
        bytes[24..32].copy_from_slice(&descriptor.number_of_pages.to_le_bytes());
        bytes
    }

    #[test]
    fn observe_platform_from_cpuid_and_firmware_inputs() {
        let memory_map = encode_descriptor(UefiMemoryDescriptor {
            typ: EFI_MEMORY_CONVENTIONAL,
            padding: 0,
            physical_start: 0,
            virtual_start: 0,
            number_of_pages: 1024,
            attribute: 0,
            reserved: 0,
        });

        let inputs = ObservationInputs {
            cpuid: reference_cpuid(),
            acpi_tables: encode_reference_dmar_with_intr_remap().to_vec(),
            memory_map,
            memory_descriptor_size: 48,
            pci_devices: vec![PciBdf {
                segment: PciSegment::new(0),
                bus: PciBus::new(0),
                device: PciDevice::new(3),
                function: PciFunction::new(0),
            }],
        };
        let observed = observe_platform(&inputs).expect("observe");
        assert!(observed.vmx);
        assert!(observed.vtd);
        assert!(observed.interrupt_remapping);
        assert_eq!(observed.ram_bytes.bytes(), 4096 * 1024);
    }

    #[test]
    fn scan_acpi_stops_on_zero_length_table() {
        let mut tables = Vec::new();
        tables.extend_from_slice(b"TEST");
        tables.extend_from_slice(&0u32.to_le_bytes());
        let (vtd, interrupt_remapping) = scan_acpi_capabilities(&tables).expect("scan");
        assert!(!vtd);
        assert!(!interrupt_remapping);
    }

    #[test]
    fn scan_acpi_rejects_short_dmar_table() {
        let mut tables = vec![0u8; 16];
        tables[0..4].copy_from_slice(b"DMAR");
        tables[4..8].copy_from_slice(&16u32.to_le_bytes());
        if let Some(flag) = tables.get_mut(DMAR_FLAGS_OFFSET) {
            *flag = DMAR_FLAG_INTR_REMAP;
        }
        let err = scan_acpi_capabilities(&tables).expect_err("must fail");
        assert_eq!(err.kind, PlatformErrorKind::Observation);
    }

    #[test]
    fn sum_conventional_ram_accepts_40_byte_descriptor_stride() {
        let mut memory_map = vec![0u8; 40];
        memory_map[0..4].copy_from_slice(&EFI_MEMORY_CONVENTIONAL.to_le_bytes());
        memory_map[24..32].copy_from_slice(&1u64.to_le_bytes());
        let ram = sum_conventional_ram(&memory_map, 40).expect("sum");
        assert_eq!(ram.bytes(), UEFI_PAGE_SIZE);
    }

    #[test]
    fn scan_acpi_rejects_table_length_beyond_buffer() {
        let mut tables = vec![0u8; 8];
        tables[0..4].copy_from_slice(b"TEST");
        tables[4..8].copy_from_slice(&64u32.to_le_bytes());
        let err = scan_acpi_capabilities(&tables).expect_err("must fail");
        assert_eq!(err.kind, PlatformErrorKind::Observation);
    }

    #[test]
    fn sum_conventional_ram_skips_trailing_partial_descriptor() {
        let memory_map = encode_descriptor(UefiMemoryDescriptor {
            typ: EFI_MEMORY_CONVENTIONAL,
            padding: 0,
            physical_start: 0,
            virtual_start: 0,
            number_of_pages: 1,
            attribute: 0,
            reserved: 0,
        });
        let mut with_trailing_byte = memory_map;
        with_trailing_byte.push(0);
        let ram = sum_conventional_ram(&with_trailing_byte, 48).expect("sum");
        assert_eq!(ram.bytes(), UEFI_PAGE_SIZE);
    }
}
