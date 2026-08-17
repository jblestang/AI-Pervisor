//! Loader to hypervisor transfer blob layout.

use core::mem::size_of;

use hv_types::PciBdf;

use crate::error::{BootError, BootErrorKind};

/// Transfer ABI version number.
pub const TRANSFER_ABI_VERSION: u32 = 1;

/// Magic bytes for [`HypervisorTransferHeader`].
pub const TRANSFER_MAGIC: [u8; 8] = *b"HVTFR\0\0\0";

/// UEFI configuration table GUID pointing at a [`HypervisorTransferHeader`].
pub const HV_TRANSFER_TABLE_GUID: Guid = Guid::from_bytes([
    0x75, 0x02, 0xbe, 0x2e, 0x6d, 0x0d, 0xaf, 0x4d, 0x8d, 0xf4, 0x0f, 0x89, 0xa2, 0xb3, 0xc4, 0xd5,
]);

/// Minimal GUID newtype used across loader and hypervisor images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guid {
    bytes: [u8; 16],
}

impl Guid {
    /// Creates a GUID from its little-endian byte order.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self { bytes }
    }

    /// Returns the GUID bytes.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }
}

/// Loader to hypervisor transfer header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HypervisorTransferHeader {
    /// Magic identifier (`HVTFR\0\0\0`).
    pub magic: [u8; 8],
    /// Transfer ABI version number.
    pub version: u32,
    /// Total transfer blob size in bytes.
    pub total_size: u32,
    /// Offset to the boot info blob from the start of this header.
    pub boot_info_offset: u32,
    /// Boot info blob size in bytes.
    pub boot_info_size: u32,
    /// Offset to the observation payload from the start of this header.
    pub observation_offset: u32,
    /// Observation payload size in bytes.
    pub observation_size: u32,
}

/// Fixed observation payload prefix before variable tail bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservationTransferHeader {
    /// CPUID leaf 1 ECX register value.
    pub leaf1_ecx: u32,
    /// CPUID leaf 1 EDX register value.
    pub leaf1_edx: u32,
    /// CPUID leaf 1 EBX register value.
    pub leaf1_ebx: u32,
    /// CPUID leaf 0x8000_0007 EDX when present.
    pub leaf80000007_edx: u32,
    /// CPUID leaf 0x8000_0008 ECX when present.
    pub leaf80000008_ecx: u32,
    /// CPUID leaf 0x480 ECX when present.
    pub leaf480_ecx: u32,
    /// CPUID leaf 0x480 EBX when present.
    pub leaf480_ebx: u32,
    /// Whether leaf 0x8000_0007 EDX is valid.
    pub leaf80000007_present: u8,
    /// Whether leaf 0x8000_0008 ECX is valid.
    pub leaf80000008_present: u8,
    /// Whether leaf 0x480 ECX is valid.
    pub leaf480_ecx_present: u8,
    /// Whether leaf 0x480 EBX is valid.
    pub leaf480_ebx_present: u8,
    /// Size of one UEFI memory map descriptor.
    pub memory_descriptor_size: u32,
    /// Raw memory map byte length following PCI entries.
    pub memory_map_size: u32,
    /// Flattened ACPI table byte length following the memory map.
    pub acpi_tables_size: u32,
    /// Number of PCI BDF entries following the ACPI bytes.
    pub pci_device_count: u32,
}

/// PCI BDF entry stored in the transfer payload.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PciBdfTransfer {
    /// PCI segment number.
    pub segment: u16,
    /// PCI bus number.
    pub bus: u8,
    /// PCI device number.
    pub device: u8,
    /// PCI function number.
    pub function: u8,
    /// Reserved padding byte.
    pub reserved: u8,
}

/// CPUID snapshot stored in a transfer payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuidTransferSnapshot {
    /// CPUID leaf 1 ECX register value.
    pub leaf1_ecx: u32,
    /// CPUID leaf 1 EDX register value.
    pub leaf1_edx: u32,
    /// CPUID leaf 1 EBX register value.
    pub leaf1_ebx: u32,
    /// CPUID leaf 0x8000_0007 EDX when available.
    pub leaf80000007_edx: Option<u32>,
    /// CPUID leaf 0x8000_0008 ECX when available.
    pub leaf80000008_ecx: Option<u32>,
    /// CPUID leaf 0x480 ECX when VMX is enabled.
    pub leaf480_ecx: Option<u32>,
    /// CPUID leaf 0x480 EBX when VMX is enabled.
    pub leaf480_ebx: Option<u32>,
}

/// Parsed transfer view over an identity-mapped buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HypervisorTransferView<'a> {
    header: HypervisorTransferHeader,
    boot_info: &'a [u8],
    observation: &'a [u8],
}

impl<'a> HypervisorTransferView<'a> {
    /// Parses a transfer blob from `bytes`.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, BootError> {
        let header = read_transfer_header(bytes)?;
        if header.magic != TRANSFER_MAGIC {
            return Err(transfer_error("transfer magic mismatch"));
        }
        if header.version != TRANSFER_ABI_VERSION {
            return Err(transfer_error("unsupported transfer version"));
        }
        if header.total_size as usize != bytes.len() {
            return Err(transfer_error("transfer total size mismatch"));
        }
        validate_canonical_layout(&header)?;
        let boot_info = slice_at(bytes, header.boot_info_offset, header.boot_info_size)?;
        let observation = slice_at(bytes, header.observation_offset, header.observation_size)?;
        Ok(Self {
            header,
            boot_info,
            observation,
        })
    }

    /// Returns the transfer header.
    pub const fn header(&self) -> &HypervisorTransferHeader {
        &self.header
    }

    /// Returns the boot info blob section.
    pub fn boot_info(&self) -> &[u8] {
        self.boot_info
    }

    /// Returns the observation payload section.
    pub fn observation(&self) -> &[u8] {
        self.observation
    }
}

/// Variable observation payload sections used to build a transfer blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationTransferParts<'a> {
    /// CPUID snapshot collected by the loader.
    pub cpuid: CpuidTransferSnapshot,
    /// Raw UEFI memory map bytes.
    pub memory_map: &'a [u8],
    /// Size of one memory map descriptor.
    pub memory_descriptor_size: usize,
    /// Flattened ACPI table bytes.
    pub acpi_tables: &'a [u8],
    /// PCI devices discovered by firmware.
    pub pci_devices: &'a [PciBdf],
}

/// Owned observation payload decoded from a transfer blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationTransferPartsOwned {
    /// CPUID snapshot collected by the loader.
    pub cpuid: CpuidTransferSnapshot,
    /// Raw UEFI memory map bytes.
    pub memory_map: alloc::vec::Vec<u8>,
    /// Size of one memory map descriptor.
    pub memory_descriptor_size: usize,
    /// Flattened ACPI table bytes.
    pub acpi_tables: alloc::vec::Vec<u8>,
    /// PCI devices discovered by firmware.
    pub pci_devices: alloc::vec::Vec<PciBdf>,
}

/// Builds a transfer blob containing boot info and observation payload bytes.
pub fn build_hypervisor_transfer_blob(
    boot_info: &[u8],
    observation: &ObservationTransferParts<'_>,
) -> Result<alloc::vec::Vec<u8>, BootError> {
    let observation_bytes = encode_observation_transfer(observation)?;
    let header_size = size_of::<HypervisorTransferHeader>();
    let boot_info_offset = u32_from_len(header_size)?;
    let boot_info_size = u32_from_len(boot_info.len())?;
    let observation_offset = boot_info_offset
        .checked_add(boot_info_size)
        .ok_or(transfer_error("observation offset overflow"))?;
    let observation_size = u32_from_len(observation_bytes.len())?;
    let total_size = (observation_offset as usize)
        .checked_add(observation_size as usize)
        .ok_or(transfer_error("transfer total size overflow"))?;

    let header = HypervisorTransferHeader {
        magic: TRANSFER_MAGIC,
        version: TRANSFER_ABI_VERSION,
        total_size: u32_from_len(total_size)?,
        boot_info_offset,
        boot_info_size,
        observation_offset,
        observation_size,
    };

    let mut blob = alloc::vec::Vec::with_capacity(total_size);
    write_transfer_header(&mut blob, &header);
    blob.extend_from_slice(boot_info);
    blob.extend_from_slice(&observation_bytes);
    if blob.len() != total_size {
        return Err(transfer_error("transfer blob size mismatch after build"));
    }
    Ok(blob)
}

/// Decodes an observation payload into structured inputs.
pub fn decode_observation_transfer(
    bytes: &[u8],
) -> Result<ObservationTransferPartsOwned, BootError> {
    let header = read_observation_header(bytes)?;
    let header_size = size_of::<ObservationTransferHeader>();
    let pci_bytes = (header.pci_device_count as usize)
        .checked_mul(size_of::<PciBdfTransfer>())
        .ok_or(transfer_error("pci table size overflow"))?;
    let tail_start = header_size
        .checked_add(pci_bytes)
        .ok_or(transfer_error("observation tail offset overflow"))?;
    let memory_map_end = tail_start
        .checked_add(header.memory_map_size as usize)
        .ok_or(transfer_error("memory map end overflow"))?;
    let acpi_end = memory_map_end
        .checked_add(header.acpi_tables_size as usize)
        .ok_or(transfer_error("acpi tables end overflow"))?;
    if acpi_end > bytes.len() {
        return Err(transfer_error("observation payload truncated"));
    }

    let cpuid = CpuidTransferSnapshot {
        leaf1_ecx: header.leaf1_ecx,
        leaf1_edx: header.leaf1_edx,
        leaf1_ebx: header.leaf1_ebx,
        leaf80000007_edx: optional_u32(header.leaf80000007_present, header.leaf80000007_edx),
        leaf80000008_ecx: optional_u32(header.leaf80000008_present, header.leaf80000008_ecx),
        leaf480_ecx: optional_u32(header.leaf480_ecx_present, header.leaf480_ecx),
        leaf480_ebx: optional_u32(header.leaf480_ebx_present, header.leaf480_ebx),
    };

    let pci_start = header_size;
    let mut pci_devices = alloc::vec::Vec::with_capacity(header.pci_device_count as usize);
    for index in 0..header.pci_device_count as usize {
        let offset = pci_start
            .checked_add(
                index
                    .checked_mul(size_of::<PciBdfTransfer>())
                    .ok_or(transfer_error("pci entry offset overflow"))?,
            )
            .ok_or(transfer_error("pci entry offset overflow"))?;
        pci_devices.push(read_pci_bdf_transfer(bytes, offset)?.into());
    }

    Ok(ObservationTransferPartsOwned {
        cpuid,
        memory_map: bytes
            .get(tail_start..memory_map_end)
            .ok_or(transfer_error("memory map slice out of bounds"))?
            .to_vec(),
        memory_descriptor_size: header.memory_descriptor_size as usize,
        acpi_tables: bytes
            .get(memory_map_end..acpi_end)
            .ok_or(transfer_error("acpi tables slice out of bounds"))?
            .to_vec(),
        pci_devices,
    })
}

impl From<PciBdfTransfer> for PciBdf {
    fn from(value: PciBdfTransfer) -> Self {
        Self {
            segment: hv_types::PciSegment::new(value.segment),
            bus: hv_types::PciBus::new(value.bus),
            device: hv_types::PciDevice::new(value.device),
            function: hv_types::PciFunction::new(value.function),
        }
    }
}

impl From<PciBdf> for PciBdfTransfer {
    fn from(value: PciBdf) -> Self {
        Self {
            segment: value.segment.raw(),
            bus: value.bus.raw(),
            device: value.device.raw(),
            function: value.function.raw(),
            reserved: 0,
        }
    }
}

fn encode_observation_transfer(
    parts: &ObservationTransferParts<'_>,
) -> Result<alloc::vec::Vec<u8>, BootError> {
    let header = ObservationTransferHeader {
        leaf1_ecx: parts.cpuid.leaf1_ecx,
        leaf1_edx: parts.cpuid.leaf1_edx,
        leaf1_ebx: parts.cpuid.leaf1_ebx,
        leaf80000007_edx: parts.cpuid.leaf80000007_edx.unwrap_or(0),
        leaf80000008_ecx: parts.cpuid.leaf80000008_ecx.unwrap_or(0),
        leaf480_ecx: parts.cpuid.leaf480_ecx.unwrap_or(0),
        leaf480_ebx: parts.cpuid.leaf480_ebx.unwrap_or(0),
        leaf80000007_present: parts.cpuid.leaf80000007_edx.is_some().into(),
        leaf80000008_present: parts.cpuid.leaf80000008_ecx.is_some().into(),
        leaf480_ecx_present: parts.cpuid.leaf480_ecx.is_some().into(),
        leaf480_ebx_present: parts.cpuid.leaf480_ebx.is_some().into(),
        memory_descriptor_size: u32_from_len(parts.memory_descriptor_size)?,
        memory_map_size: u32_from_len(parts.memory_map.len())?,
        acpi_tables_size: u32_from_len(parts.acpi_tables.len())?,
        pci_device_count: u32_from_len(parts.pci_devices.len())?,
    };

    let header_size = size_of::<ObservationTransferHeader>();
    let pci_bytes = parts
        .pci_devices
        .len()
        .checked_mul(size_of::<PciBdfTransfer>())
        .ok_or(transfer_error("pci table size overflow"))?;
    let total_size = header_size
        .checked_add(pci_bytes)
        .and_then(|size| size.checked_add(parts.memory_map.len()))
        .and_then(|size| size.checked_add(parts.acpi_tables.len()))
        .ok_or(transfer_error("observation payload size overflow"))?;

    let mut bytes = alloc::vec::Vec::with_capacity(total_size);
    write_observation_header(&mut bytes, &header);
    for device in parts.pci_devices {
        write_pci_bdf_transfer(&mut bytes, &PciBdfTransfer::from(*device));
    }
    bytes.extend_from_slice(parts.memory_map);
    bytes.extend_from_slice(parts.acpi_tables);
    Ok(bytes)
}

fn validate_canonical_layout(header: &HypervisorTransferHeader) -> Result<(), BootError> {
    let header_size = size_of::<HypervisorTransferHeader>();
    if header.boot_info_offset as usize != header_size {
        return Err(transfer_error(
            "boot info offset must immediately follow header",
        ));
    }
    let boot_end = (header.boot_info_offset as usize)
        .checked_add(header.boot_info_size as usize)
        .ok_or(transfer_error("boot info end overflow"))?;
    if boot_end != header.observation_offset as usize {
        return Err(transfer_error(
            "observation must immediately follow boot info",
        ));
    }
    let observation_end = (header.observation_offset as usize)
        .checked_add(header.observation_size as usize)
        .ok_or(transfer_error("observation end overflow"))?;
    if observation_end != header.total_size as usize {
        return Err(transfer_error(
            "transfer total size must equal section span",
        ));
    }
    Ok(())
}

fn read_transfer_header(bytes: &[u8]) -> Result<HypervisorTransferHeader, BootError> {
    if bytes.len() < size_of::<HypervisorTransferHeader>() {
        return Err(transfer_error("transfer blob shorter than header"));
    }
    let header_bytes = bytes
        .get(0..size_of::<HypervisorTransferHeader>())
        .ok_or(transfer_error("transfer header truncated"))?;
    let mut offset = 0usize;
    let mut magic = [0u8; 8];
    copy_field(&mut magic, header_bytes, &mut offset)?;
    Ok(HypervisorTransferHeader {
        magic,
        version: read_u32(header_bytes, &mut offset)?,
        total_size: read_u32(header_bytes, &mut offset)?,
        boot_info_offset: read_u32(header_bytes, &mut offset)?,
        boot_info_size: read_u32(header_bytes, &mut offset)?,
        observation_offset: read_u32(header_bytes, &mut offset)?,
        observation_size: read_u32(header_bytes, &mut offset)?,
    })
}

fn write_transfer_header(out: &mut alloc::vec::Vec<u8>, header: &HypervisorTransferHeader) {
    out.extend_from_slice(&header.magic);
    write_u32(out, header.version);
    write_u32(out, header.total_size);
    write_u32(out, header.boot_info_offset);
    write_u32(out, header.boot_info_size);
    write_u32(out, header.observation_offset);
    write_u32(out, header.observation_size);
}

fn read_observation_header(bytes: &[u8]) -> Result<ObservationTransferHeader, BootError> {
    if bytes.len() < size_of::<ObservationTransferHeader>() {
        return Err(transfer_error("observation payload shorter than header"));
    }
    let header_bytes = bytes
        .get(0..size_of::<ObservationTransferHeader>())
        .ok_or(transfer_error("observation header truncated"))?;
    let mut offset = 0usize;
    Ok(ObservationTransferHeader {
        leaf1_ecx: read_u32(header_bytes, &mut offset)?,
        leaf1_edx: read_u32(header_bytes, &mut offset)?,
        leaf1_ebx: read_u32(header_bytes, &mut offset)?,
        leaf80000007_edx: read_u32(header_bytes, &mut offset)?,
        leaf80000008_ecx: read_u32(header_bytes, &mut offset)?,
        leaf480_ecx: read_u32(header_bytes, &mut offset)?,
        leaf480_ebx: read_u32(header_bytes, &mut offset)?,
        leaf80000007_present: read_u8(header_bytes, &mut offset)?,
        leaf80000008_present: read_u8(header_bytes, &mut offset)?,
        leaf480_ecx_present: read_u8(header_bytes, &mut offset)?,
        leaf480_ebx_present: read_u8(header_bytes, &mut offset)?,
        memory_descriptor_size: read_u32(header_bytes, &mut offset)?,
        memory_map_size: read_u32(header_bytes, &mut offset)?,
        acpi_tables_size: read_u32(header_bytes, &mut offset)?,
        pci_device_count: read_u32(header_bytes, &mut offset)?,
    })
}

fn write_observation_header(out: &mut alloc::vec::Vec<u8>, header: &ObservationTransferHeader) {
    write_u32(out, header.leaf1_ecx);
    write_u32(out, header.leaf1_edx);
    write_u32(out, header.leaf1_ebx);
    write_u32(out, header.leaf80000007_edx);
    write_u32(out, header.leaf80000008_ecx);
    write_u32(out, header.leaf480_ecx);
    write_u32(out, header.leaf480_ebx);
    out.push(header.leaf80000007_present);
    out.push(header.leaf80000008_present);
    out.push(header.leaf480_ecx_present);
    out.push(header.leaf480_ebx_present);
    write_u32(out, header.memory_descriptor_size);
    write_u32(out, header.memory_map_size);
    write_u32(out, header.acpi_tables_size);
    write_u32(out, header.pci_device_count);
}

fn read_pci_bdf_transfer(bytes: &[u8], offset: usize) -> Result<PciBdfTransfer, BootError> {
    let end = offset
        .checked_add(size_of::<PciBdfTransfer>())
        .ok_or(transfer_error("pci entry end overflow"))?;
    let entry = bytes
        .get(offset..end)
        .ok_or(transfer_error("pci entry truncated"))?;
    let mut cursor = 0usize;
    Ok(PciBdfTransfer {
        segment: read_u16(entry, &mut cursor)?,
        bus: read_u8(entry, &mut cursor)?,
        device: read_u8(entry, &mut cursor)?,
        function: read_u8(entry, &mut cursor)?,
        reserved: read_u8(entry, &mut cursor)?,
    })
}

fn write_pci_bdf_transfer(out: &mut alloc::vec::Vec<u8>, entry: &PciBdfTransfer) {
    write_u16(out, entry.segment);
    out.push(entry.bus);
    out.push(entry.device);
    out.push(entry.function);
    out.push(entry.reserved);
}

fn optional_u32(present: u8, value: u32) -> Option<u32> {
    if present == 0 {
        None
    } else {
        Some(value)
    }
}

fn slice_at(bytes: &[u8], offset: u32, size: u32) -> Result<&[u8], BootError> {
    let start = offset as usize;
    let end = start
        .checked_add(size as usize)
        .ok_or(transfer_error("section end overflow"))?;
    bytes
        .get(start..end)
        .ok_or(transfer_error("section out of bounds"))
}

fn u32_from_len(len: usize) -> Result<u32, BootError> {
    u32::try_from(len).map_err(|_| transfer_error("length exceeds u32"))
}

fn transfer_error(message: &'static str) -> BootError {
    BootError::new(BootErrorKind::Bounds, message)
}

fn copy_field(out: &mut [u8], input: &[u8], offset: &mut usize) -> Result<(), BootError> {
    let end = offset
        .checked_add(out.len())
        .ok_or(transfer_error("field copy overflow"))?;
    let slice = input
        .get(*offset..end)
        .ok_or(transfer_error("field copy truncated"))?;
    out.copy_from_slice(slice);
    *offset = end;
    Ok(())
}

fn read_u8(bytes: &[u8], offset: &mut usize) -> Result<u8, BootError> {
    let value = bytes
        .get(*offset)
        .copied()
        .ok_or(transfer_error("u8 read truncated"))?;
    *offset = offset.saturating_add(1);
    Ok(value)
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> Result<u16, BootError> {
    let start = *offset;
    let end = start
        .checked_add(size_of::<u16>())
        .ok_or(transfer_error("u16 read overflow"))?;
    let slice = bytes
        .get(start..end)
        .ok_or(transfer_error("u16 read truncated"))?;
    let mut value = [0u8; 2];
    value.copy_from_slice(slice);
    *offset = end;
    Ok(u16::from_le_bytes(value))
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, BootError> {
    let start = *offset;
    let end = start
        .checked_add(size_of::<u32>())
        .ok_or(transfer_error("u32 read overflow"))?;
    let slice = bytes
        .get(start..end)
        .ok_or(transfer_error("u32 read truncated"))?;
    let mut value = [0u8; 4];
    value.copy_from_slice(slice);
    *offset = end;
    Ok(u32::from_le_bytes(value))
}

fn write_u16(out: &mut alloc::vec::Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut alloc::vec::Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hv_types::{PciBus, PciDevice, PciFunction, PciSegment};

    #[test]
    fn transfer_roundtrip_preserves_optional_cpuid_leaves() {
        let parts = ObservationTransferParts {
            cpuid: CpuidTransferSnapshot {
                leaf1_ecx: 1,
                leaf1_edx: 2,
                leaf1_ebx: 3,
                leaf80000007_edx: Some(4),
                leaf80000008_ecx: Some(5),
                leaf480_ecx: Some(6),
                leaf480_ebx: Some(7),
            },
            memory_map: &[0x11; 16],
            memory_descriptor_size: 48,
            acpi_tables: &[0x22; 8],
            pci_devices: &[],
        };
        let blob = build_hypervisor_transfer_blob(&[0xBB; 16], &parts).expect("build");
        let view = HypervisorTransferView::parse(&blob).expect("parse");
        let decoded = decode_observation_transfer(view.observation()).expect("decode");
        assert_eq!(decoded.cpuid, parts.cpuid);
    }

    #[test]
    fn pci_bdf_transfer_conversions_roundtrip() {
        let bdf = PciBdf::new(
            PciSegment::new(1),
            PciBus::new(2),
            PciDevice::new(3),
            PciFunction::new(4),
        );
        let transfer = PciBdfTransfer::from(bdf);
        let restored = PciBdf::from(transfer);
        assert_eq!(bdf, restored);
    }

    #[test]
    fn transfer_roundtrip_preserves_boot_info_and_observation() {
        let boot_info = vec![0xAA; 64];
        let parts = ObservationTransferParts {
            cpuid: CpuidTransferSnapshot {
                leaf1_ecx: 1,
                leaf1_edx: 2,
                leaf1_ebx: 3,
                leaf80000007_edx: Some(4),
                leaf80000008_ecx: None,
                leaf480_ecx: Some(5),
                leaf480_ebx: None,
            },
            memory_map: &[0x11; 48],
            memory_descriptor_size: 48,
            acpi_tables: &[0x22; 16],
            pci_devices: &[PciBdf::new(
                PciSegment::new(0),
                PciBus::new(0),
                PciDevice::new(3),
                PciFunction::new(0),
            )],
        };
        let blob = build_hypervisor_transfer_blob(&boot_info, &parts).expect("build");
        let view = HypervisorTransferView::parse(&blob).expect("parse");
        assert_eq!(view.boot_info(), boot_info.as_slice());
        let decoded = decode_observation_transfer(view.observation()).expect("decode");
        assert_eq!(decoded.cpuid, parts.cpuid);
        assert_eq!(decoded.memory_map, parts.memory_map);
        assert_eq!(decoded.acpi_tables, parts.acpi_tables);
        assert_eq!(decoded.pci_devices, parts.pci_devices);
    }

    #[test]
    fn transfer_parse_rejects_bad_magic_and_truncated_blob() {
        assert!(HypervisorTransferView::parse(&[]).is_err());
        let mut blob = build_hypervisor_transfer_blob(
            &[0xAA; 8],
            &ObservationTransferParts {
                cpuid: CpuidTransferSnapshot {
                    leaf1_ecx: 0,
                    leaf1_edx: 0,
                    leaf1_ebx: 0,
                    leaf80000007_edx: None,
                    leaf80000008_ecx: None,
                    leaf480_ecx: None,
                    leaf480_ebx: None,
                },
                memory_map: &[],
                memory_descriptor_size: 48,
                acpi_tables: &[],
                pci_devices: &[],
            },
        )
        .expect("build");
        blob[0] = b'X';
        assert!(HypervisorTransferView::parse(&blob).is_err());
    }

    #[test]
    fn transfer_parse_rejects_version_and_size_mismatch() {
        let blob = build_hypervisor_transfer_blob(
            &[0xAA; 8],
            &ObservationTransferParts {
                cpuid: CpuidTransferSnapshot {
                    leaf1_ecx: 0,
                    leaf1_edx: 0,
                    leaf1_ebx: 0,
                    leaf80000007_edx: None,
                    leaf80000008_ecx: None,
                    leaf480_ecx: None,
                    leaf480_ebx: None,
                },
                memory_map: &[],
                memory_descriptor_size: 48,
                acpi_tables: &[],
                pci_devices: &[],
            },
        )
        .expect("build");
        let mut wrong_version = blob.clone();
        wrong_version[8..12].copy_from_slice(&(TRANSFER_ABI_VERSION + 1).to_le_bytes());
        assert!(HypervisorTransferView::parse(&wrong_version).is_err());
        let mut wrong_size = blob;
        wrong_size.pop();
        assert!(HypervisorTransferView::parse(&wrong_size).is_err());
    }

    #[test]
    fn decode_observation_transfer_rejects_truncated_payload() {
        assert!(decode_observation_transfer(&[0u8; 4]).is_err());
    }

    #[test]
    fn transfer_header_and_observation_header_layouts_are_stable() {
        use core::mem::{align_of, size_of};

        assert_eq!(size_of::<HypervisorTransferHeader>(), 32);
        assert_eq!(align_of::<HypervisorTransferHeader>(), 4);
        assert_eq!(size_of::<ObservationTransferHeader>(), 48);
    }

    #[test]
    fn transfer_parse_rejects_non_canonical_section_layout() {
        let blob = build_hypervisor_transfer_blob(
            &[0xAA; 8],
            &ObservationTransferParts {
                cpuid: CpuidTransferSnapshot {
                    leaf1_ecx: 0,
                    leaf1_edx: 0,
                    leaf1_ebx: 0,
                    leaf80000007_edx: None,
                    leaf80000008_ecx: None,
                    leaf480_ecx: None,
                    leaf480_ebx: None,
                },
                memory_map: &[],
                memory_descriptor_size: 48,
                acpi_tables: &[],
                pci_devices: &[],
            },
        )
        .expect("build");
        let header_size = core::mem::size_of::<HypervisorTransferHeader>();
        assert_eq!(header_size, 32);
        let mut non_canonical = blob.clone();
        non_canonical[16..20].copy_from_slice(&64u32.to_le_bytes());
        assert!(HypervisorTransferView::parse(&non_canonical).is_err());
    }

    #[test]
    fn transfer_parse_rejects_gap_between_boot_info_and_observation() {
        let blob = build_hypervisor_transfer_blob(
            &[0xAA; 8],
            &ObservationTransferParts {
                cpuid: CpuidTransferSnapshot {
                    leaf1_ecx: 0,
                    leaf1_edx: 0,
                    leaf1_ebx: 0,
                    leaf80000007_edx: None,
                    leaf80000008_ecx: None,
                    leaf480_ecx: None,
                    leaf480_ebx: None,
                },
                memory_map: &[],
                memory_descriptor_size: 48,
                acpi_tables: &[],
                pci_devices: &[],
            },
        )
        .expect("build");
        let boot_info_offset = u32::from_le_bytes(blob[16..20].try_into().expect("offset"));
        let boot_info_size = u32::from_le_bytes(blob[20..24].try_into().expect("size"));
        let mut gapped = blob;
        gapped[24..28].copy_from_slice(&(boot_info_offset + boot_info_size + 4).to_le_bytes());
        assert!(HypervisorTransferView::parse(&gapped).is_err());
    }

    #[test]
    fn transfer_constants_match_expected_guid_bytes() {
        assert_eq!(TRANSFER_MAGIC, *b"HVTFR\0\0\0");
        assert_eq!(HV_TRANSFER_TABLE_GUID.as_bytes()[0], 0x75);
    }
}
