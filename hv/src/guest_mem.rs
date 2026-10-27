//! Guest GPA layout packed from [`crate::config::FlatHvConfig`] sizes.

use crate::config::FlatHvConfig;
use crate::PAGE_SIZE;

/// Which guest domain owns a private region / image slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuestId {
    Wan,
    Lan,
}

/// Page-aligned GPA extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpaRegion {
    pub base: u64,
    pub size: u64,
}

impl GpaRegion {
    #[inline]
    pub fn end(self) -> Option<u64> {
        self.base.checked_add(self.size)
    }
}

/// Full dual-guest + shared layout derived only from flat config sizes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestLayout {
    pub wan_private: GpaRegion,
    pub lan_private: GpaRegion,
    pub ring_wan_to_lan: GpaRegion,
    pub ring_lan_to_wan: GpaRegion,
    pub arena: GpaRegion,
    pub stats: GpaRegion,
    pub boot_info_wan: GpaRegion,
    pub boot_info_lan: GpaRegion,
}

impl GuestLayout {
    /// Private memory slot used as the guest image / RAM window.
    pub fn image_region(&self, id: GuestId) -> GpaRegion {
        match id {
            GuestId::Wan => self.wan_private,
            GuestId::Lan => self.lan_private,
        }
    }

    pub fn boot_info_region(&self, id: GuestId) -> GpaRegion {
        match id {
            GuestId::Wan => self.boot_info_wan,
            GuestId::Lan => self.boot_info_lan,
        }
    }
}

/// Why layout construction failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    ZeroSize,
    Overflow,
    UnalignedBase,
}

fn align_up(value: u64, align: u64) -> Option<u64> {
    if align == 0 || !align.is_power_of_two() {
        return None;
    }
    let mask = align - 1;
    value.checked_add(mask).map(|v| v & !mask)
}

fn mb_to_bytes(mb: u64) -> Option<u64> {
    mb.checked_mul(1024 * 1024)
}

/// Bytes needed for one SPSC ring (header + descriptor slots), page-rounded.
pub fn ring_bytes(capacity_pow2: u32) -> Option<u64> {
    if capacity_pow2 == 0 || !capacity_pow2.is_power_of_two() {
        return None;
    }
    // RingHeader is 16 bytes; FrameDesc is 8 bytes (`ring` crate / ICD).
    let header = 16u64;
    let slots = (capacity_pow2 as u64).checked_mul(8)?;
    let raw = header.checked_add(slots)?;
    align_up(raw, PAGE_SIZE)
}

/// Payload arena: `capacity_pow2 * frame_capacity`, page-rounded.
pub fn arena_bytes(capacity_pow2: u32, frame_capacity: u32) -> Option<u64> {
    let raw = (capacity_pow2 as u64).checked_mul(frame_capacity as u64)?;
    let raw = core::cmp::max(raw, PAGE_SIZE);
    align_up(raw, PAGE_SIZE)
}

struct Packer {
    cursor: u64,
}

impl Packer {
    fn new(base: u64) -> Result<Self, LayoutError> {
        if base % PAGE_SIZE != 0 {
            return Err(LayoutError::UnalignedBase);
        }
        Ok(Self { cursor: base })
    }

    fn alloc(&mut self, size: u64) -> Result<GpaRegion, LayoutError> {
        if size == 0 {
            return Err(LayoutError::ZeroSize);
        }
        let size = align_up(size, PAGE_SIZE).ok_or(LayoutError::Overflow)?;
        let base = self.cursor;
        self.cursor = base.checked_add(size).ok_or(LayoutError::Overflow)?;
        Ok(GpaRegion { base, size })
    }
}

/// Pack WAN/LAN private + shared ring/arena/stats/boot-info from config sizes.
pub fn build_layout(cfg: &FlatHvConfig) -> Result<GuestLayout, LayoutError> {
    let mut pack = Packer::new(cfg.gpa_window_base)?;

    let wan_sz = mb_to_bytes(cfg.wan_memory_mb).ok_or(LayoutError::Overflow)?;
    let lan_sz = mb_to_bytes(cfg.lan_memory_mb).ok_or(LayoutError::Overflow)?;
    if wan_sz == 0 || lan_sz == 0 {
        return Err(LayoutError::ZeroSize);
    }

    let ring_sz = ring_bytes(cfg.ring_capacity_pow2).ok_or(LayoutError::Overflow)?;
    let arena_sz =
        arena_bytes(cfg.ring_capacity_pow2, cfg.frame_capacity).ok_or(LayoutError::Overflow)?;

    let wan_private = pack.alloc(wan_sz)?;
    let lan_private = pack.alloc(lan_sz)?;
    let ring_wan_to_lan = pack.alloc(ring_sz)?;
    let ring_lan_to_wan = pack.alloc(ring_sz)?;
    let arena = pack.alloc(arena_sz)?;
    let stats = pack.alloc(PAGE_SIZE)?;
    let boot_info_wan = pack.alloc(PAGE_SIZE)?;
    let boot_info_lan = pack.alloc(PAGE_SIZE)?;

    Ok(GuestLayout {
        wan_private,
        lan_private,
        ring_wan_to_lan,
        ring_lan_to_wan,
        arena,
        stats,
        boot_info_wan,
        boot_info_lan,
    })
}
