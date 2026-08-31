//! Guest boot info blob parsing and validation.

use core::mem::size_of;

use hv_guest_abi::{
    guest_abi_is_compatible, guest_boot_info_has_relay_measurement_tail,
    guest_boot_info_relay_frames_offset, parse_guest_boot_info_relay_measurement,
    GuestBootInfoHeader, GuestDeviceKind, GuestDeviceRegion, GuestIpcRegion, GuestIpcRole,
    GuestMemoryKind, GuestMemoryRegion,
};
use hv_types::{GuestPhysAddr, VcpuId, VmId};

/// Category of guest boot info parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestBootInfoParseErrorKind {
    /// Buffer was too short or malformed.
    Parse,
    /// Declared offsets or counts were inconsistent.
    Bounds,
    /// Header version or magic was unsupported.
    Incompatible,
}

/// Structured guest boot info parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestBootInfoParseError {
    /// Error category.
    pub kind: GuestBootInfoParseErrorKind,
    /// Human-readable message.
    pub message: alloc::string::String,
}

impl GuestBootInfoParseError {
    /// Creates a new guest boot info parse error.
    pub fn new(kind: GuestBootInfoParseErrorKind, message: impl Into<alloc::string::String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Borrowed view over a guest boot info blob.
#[derive(Debug, Clone, Copy)]
pub struct GuestBootInfoView<'a> {
    bytes: &'a [u8],
    header: GuestBootInfoHeader,
}

impl<'a> GuestBootInfoView<'a> {
    /// Parses and validates a guest boot info blob.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, GuestBootInfoParseError> {
        if bytes.len() < size_of::<GuestBootInfoHeader>() {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Parse,
                "guest boot info shorter than header",
            ));
        }
        let header = read_header(bytes)?;
        if !guest_abi_is_compatible(&header) {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Incompatible,
                "unsupported guest boot info header",
            ));
        }
        if header.size as usize > bytes.len() {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "declared guest boot info size exceeds buffer",
            ));
        }
        let view = Self { bytes, header };
        view.validate_layout()?;
        Ok(view)
    }

    /// Returns the parsed header.
    pub const fn header(&self) -> &GuestBootInfoHeader {
        &self.header
    }

    /// Reads one memory region descriptor by index.
    pub fn memory_region(&self, index: u32) -> Result<GuestMemoryRegion, GuestBootInfoParseError> {
        if index >= self.header.memory_region_count {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "memory region index out of range",
            ));
        }
        let offset = self
            .header
            .memory_table_offset
            .checked_add(index.saturating_mul(size_of::<GuestMemoryRegion>() as u32))
            .ok_or_else(|| {
                GuestBootInfoParseError::new(
                    GuestBootInfoParseErrorKind::Bounds,
                    "memory region offset overflow",
                )
            })? as usize;
        read_memory_region(self.slice_at(offset, size_of::<GuestMemoryRegion>())?)
    }

    /// Reads one IPC region descriptor by index.
    pub fn ipc_region(&self, index: u32) -> Result<GuestIpcRegion, GuestBootInfoParseError> {
        if index >= self.header.ipc_region_count {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "ipc region index out of range",
            ));
        }
        let offset = self
            .header
            .ipc_table_offset
            .checked_add(index.saturating_mul(size_of::<GuestIpcRegion>() as u32))
            .ok_or_else(|| {
                GuestBootInfoParseError::new(
                    GuestBootInfoParseErrorKind::Bounds,
                    "ipc region offset overflow",
                )
            })? as usize;
        read_ipc_region(self.slice_at(offset, size_of::<GuestIpcRegion>())?)
    }

    /// Reads one device region descriptor by index.
    pub fn device_region(&self, index: u32) -> Result<GuestDeviceRegion, GuestBootInfoParseError> {
        if index >= self.header.device_region_count {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "device region index out of range",
            ));
        }
        let offset = self
            .header
            .device_table_offset
            .checked_add(index.saturating_mul(size_of::<GuestDeviceRegion>() as u32))
            .ok_or_else(|| {
                GuestBootInfoParseError::new(
                    GuestBootInfoParseErrorKind::Bounds,
                    "device region offset overflow",
                )
            })? as usize;
        read_device_region(self.slice_at(offset, size_of::<GuestDeviceRegion>())?)
    }

    fn validate_layout(&self) -> Result<(), GuestBootInfoParseError> {
        let bounded = self.slice_at(0, self.header.size as usize)?;
        let memory_end = self
            .header
            .memory_table_offset
            .checked_add(
                self.header
                    .memory_region_count
                    .saturating_mul(size_of::<GuestMemoryRegion>() as u32),
            )
            .ok_or_else(|| {
                GuestBootInfoParseError::new(
                    GuestBootInfoParseErrorKind::Bounds,
                    "memory table end overflow",
                )
            })?;
        if memory_end > self.header.ipc_table_offset {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "memory table overlaps ipc table",
            ));
        }
        let ipc_end = self
            .header
            .ipc_table_offset
            .checked_add(
                self.header
                    .ipc_region_count
                    .saturating_mul(size_of::<GuestIpcRegion>() as u32),
            )
            .ok_or_else(|| {
                GuestBootInfoParseError::new(
                    GuestBootInfoParseErrorKind::Bounds,
                    "ipc table end overflow",
                )
            })?;
        if ipc_end > self.header.device_table_offset {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "ipc table overlaps device table",
            ));
        }
        let device_end = self
            .header
            .device_table_offset
            .checked_add(
                self.header
                    .device_region_count
                    .saturating_mul(size_of::<GuestDeviceRegion>() as u32),
            )
            .ok_or_else(|| {
                GuestBootInfoParseError::new(
                    GuestBootInfoParseErrorKind::Bounds,
                    "device table end overflow",
                )
            })?;
        if device_end > self.header.size {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "device table exceeds declared guest boot info size",
            ));
        }
        if self.header.version >= 2
            && guest_boot_info_relay_frames_offset(self.header.size).is_none()
        {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "guest boot info ABI v2 requires relay measurement tail",
            ));
        }
        if guest_boot_info_has_relay_measurement_tail(&self.header)
            && parse_guest_boot_info_relay_measurement(bounded).is_none()
        {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Incompatible,
                "guest boot info relay measurement extension invalid",
            ));
        }
        if bounded.len() < self.header.size as usize {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Bounds,
                "guest boot info bounded slice unavailable",
            ));
        }
        Ok(())
    }

    fn slice_at(&self, offset: usize, len: usize) -> Result<&'a [u8], GuestBootInfoParseError> {
        self.bytes
            .get(offset..offset.checked_add(len).ok_or_else(|| {
                GuestBootInfoParseError::new(
                    GuestBootInfoParseErrorKind::Bounds,
                    "guest boot info slice overflow",
                )
            })?)
            .ok_or_else(|| {
                GuestBootInfoParseError::new(
                    GuestBootInfoParseErrorKind::Bounds,
                    "guest boot info slice out of bounds",
                )
            })
    }
}

fn read_header(bytes: &[u8]) -> Result<GuestBootInfoHeader, GuestBootInfoParseError> {
    let magic = bytes
        .get(0..8)
        .ok_or_else(|| {
            GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Parse,
                "guest boot info magic unavailable",
            )
        })?
        .try_into()
        .map_err(|_| {
            GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Parse,
                "guest boot info magic conversion failed",
            )
        })?;
    Ok(GuestBootInfoHeader {
        magic,
        version: read_u32(bytes, 8)?,
        size: read_u32(bytes, 12)?,
        vm_id: VmId::new(read_u32(bytes, 16)?),
        vcpu_id: VcpuId::new(read_u32(bytes, 20)?),
        memory_table_offset: read_u32(bytes, 24)?,
        memory_region_count: read_u32(bytes, 28)?,
        ipc_table_offset: read_u32(bytes, 32)?,
        ipc_region_count: read_u32(bytes, 36)?,
        device_table_offset: read_u32(bytes, 40)?,
        device_region_count: read_u32(bytes, 44)?,
    })
}

fn read_memory_region(bytes: &[u8]) -> Result<GuestMemoryRegion, GuestBootInfoParseError> {
    let kind = match read_u32(bytes, 0)? {
        1 => GuestMemoryKind::Ram,
        2 => GuestMemoryKind::Mmio,
        _ => {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Parse,
                "unknown guest memory kind",
            ))
        }
    };
    Ok(GuestMemoryRegion {
        kind,
        guest_phys: GuestPhysAddr::new(read_u64(bytes, 8)?),
        size: read_u64(bytes, 16)?,
    })
}

fn read_ipc_region(bytes: &[u8]) -> Result<GuestIpcRegion, GuestBootInfoParseError> {
    let role = match read_u32(bytes, 4)? {
        1 => GuestIpcRole::Producer,
        2 => GuestIpcRole::Consumer,
        _ => {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Parse,
                "unknown guest ipc role",
            ))
        }
    };
    Ok(GuestIpcRegion {
        channel_id: read_u32(bytes, 0)?,
        role,
        guest_phys: GuestPhysAddr::new(read_u64(bytes, 8)?),
        size: read_u64(bytes, 16)?,
    })
}

fn read_device_region(bytes: &[u8]) -> Result<GuestDeviceRegion, GuestBootInfoParseError> {
    let kind = match read_u32(bytes, 0)? {
        1 => GuestDeviceKind::NicE1000,
        _ => {
            return Err(GuestBootInfoParseError::new(
                GuestBootInfoParseErrorKind::Parse,
                "unknown guest device kind",
            ))
        }
    };
    Ok(GuestDeviceRegion {
        kind,
        mmio_guest_phys: GuestPhysAddr::new(read_u64(bytes, 8)?),
        mmio_size: read_u64(bytes, 16)?,
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, GuestBootInfoParseError> {
    let slice = bytes.get(offset..offset + 4).ok_or_else(|| {
        GuestBootInfoParseError::new(
            GuestBootInfoParseErrorKind::Parse,
            "guest boot info u32 out of bounds",
        )
    })?;
    Ok(u32::from_le_bytes([
        slice.first().copied().unwrap_or(0),
        slice.get(1).copied().unwrap_or(0),
        slice.get(2).copied().unwrap_or(0),
        slice.get(3).copied().unwrap_or(0),
    ]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, GuestBootInfoParseError> {
    let slice = bytes.get(offset..offset + 8).ok_or_else(|| {
        GuestBootInfoParseError::new(
            GuestBootInfoParseErrorKind::Parse,
            "guest boot info u64 out of bounds",
        )
    })?;
    Ok(u64::from_le_bytes([
        slice.first().copied().unwrap_or(0),
        slice.get(1).copied().unwrap_or(0),
        slice.get(2).copied().unwrap_or(0),
        slice.get(3).copied().unwrap_or(0),
        slice.get(4).copied().unwrap_or(0),
        slice.get(5).copied().unwrap_or(0),
        slice.get(6).copied().unwrap_or(0),
        slice.get(7).copied().unwrap_or(0),
    ]))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use hv_config_model::compile_config_from_str;
    use hv_guest_abi::{GUEST_ABI_VERSION, GUEST_BOOT_INFO_MAGIC};
    use hv_platform_model::plan_static_platform_ir;

    #[test]
    fn parse_round_trips_reference_in_partition_blob() {
        let yaml = include_str!("../../../configs/qemu.yaml");
        let compiled = compile_config_from_str(yaml).expect("compile");
        let layout = plan_static_platform_ir(&compiled.intent).expect("plan");
        let blob = crate::boot_info::build_guest_boot_info_for_partition(&layout, "in").expect("build");
        let view = GuestBootInfoView::parse(&blob).expect("parse");
        assert_eq!(view.header().ipc_region_count, 1);
        assert_eq!(view.header().device_region_count, 1);
    }

    #[test]
    fn parse_rejects_truncated_blob() {
        let blob = [0u8; 8];
        assert!(GuestBootInfoView::parse(&blob).is_err());
    }

    #[test]
    fn parse_rejects_bad_magic() {
        let mut header = GuestBootInfoHeader {
            magic: GUEST_BOOT_INFO_MAGIC,
            version: GUEST_ABI_VERSION,
            size: size_of::<GuestBootInfoHeader>() as u32,
            vm_id: VmId::new(0),
            vcpu_id: VcpuId::new(0),
            memory_table_offset: size_of::<GuestBootInfoHeader>() as u32,
            memory_region_count: 0,
            ipc_table_offset: size_of::<GuestBootInfoHeader>() as u32,
            ipc_region_count: 0,
            device_table_offset: size_of::<GuestBootInfoHeader>() as u32,
            device_region_count: 0,
        };
        header.magic = *b"BADVERS\0";
        let mut bytes = vec![0u8; header.size as usize];
        if let Some(prefix) = bytes.get_mut(0..8) {
            prefix.copy_from_slice(&header.magic);
        }
        assert!(GuestBootInfoView::parse(&bytes).is_err());
    }
}
