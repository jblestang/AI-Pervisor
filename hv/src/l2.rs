//! L2 guest place / BootInfo install / VMLAUNCH orchestration (host-testable pieces).

use guest_common::{BootInfo, GuestRole};

use crate::boot_blob::fill_boot_info;
use crate::config::FlatHvConfig;
use crate::guest_load::{load_elf64, load_raw_image, ElfLoad, LoadError};
use crate::guest_mem::{GuestId, GuestLayout, GpaRegion};
use crate::vmx::controls::{AdjustedControls, ControlCaps, ControlWants};
use crate::vmx::enter::{prepare_launch_image, write_guest_gdt, GuestLaunchImage, LaunchTips};
use crate::vmx::guest_pt::{pack_guest_pt, spike_guest_maps, GuestPtError};
use crate::PAGE_SIZE;

/// Host-testable description of one prepared L2 guest.
#[derive(Clone, Debug)]
pub struct PreparedGuest {
    pub role: GuestRole,
    pub entry_va: u64,
    pub link_base: u64,
    pub image_span: u64,
    pub boot_info_gpa: u64,
    pub private: GpaRegion,
    pub launch: GuestLaunchImage,
}

/// Errors while preparing L2 guest memory / launch image (no VT-x yet).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrepareError {
    Load(LoadError),
    Pt(GuestPtError),
    Launch,
    Buffer,
}

impl From<LoadError> for PrepareError {
    fn from(e: LoadError) -> Self {
        Self::Load(e)
    }
}

impl From<GuestPtError> for PrepareError {
    fn from(e: GuestPtError) -> Self {
        Self::Pt(e)
    }
}

/// Write BootInfo into a page-sized buffer (caller maps it at `boot.boot_info_gpa`).
pub fn install_boot_info(page: &mut [u8], boot: &BootInfo) -> Result<(), PrepareError> {
    if page.len() < core::mem::size_of::<BootInfo>() {
        return Err(PrepareError::Buffer);
    }
    // SAFETY: BootInfo is repr(C) POD; page is caller-owned.
    unsafe {
        core::ptr::write(page.as_mut_ptr() as *mut BootInfo, *boot);
    }
    Ok(())
}

/// Load guest image into `dest` (private RAM window) and return ELF metadata.
pub fn load_into_private(
    dest: &mut [u8],
    region: GpaRegion,
    image: &[u8],
) -> Result<ElfLoad, PrepareError> {
    if image.len() >= 4 && &image[0..4] == b"\x7fELF" {
        Ok(load_elf64(dest, region, image)?)
    } else {
        let entry = load_raw_image(dest, region, image)?;
        Ok(ElfLoad {
            entry_va: entry,
            link_base: region.base,
            image_span: image.len() as u64,
        })
    }
}

/// Build identity GPA list for guest page tables from layout + role.
pub fn identity_tips(layout: &GuestLayout, role: GuestRole) -> alloc::vec::Vec<u64> {
    let mut v = alloc::vec![
        layout.ring_wan_to_lan.base,
        layout.ring_lan_to_wan.base,
        layout.arena.base,
        layout.stats.base,
    ];
    match role {
        GuestRole::Wan => v.push(layout.boot_info_wan.base),
        GuestRole::Lan => v.push(layout.boot_info_lan.base),
    }
    v
}

/// Pack guest PT into `pt_storage` (**must already live at the final identity GPA**),
/// write GDT into `gdt_page` (caller copies to `gdt_gpa`), build launch image.
///
/// `controls` must already be MSR-adjusted (freestanding) or permissive (host tests).
pub fn prepare_guest(
    cfg: &FlatHvConfig,
    layout: &GuestLayout,
    role: GuestRole,
    elf: &ElfLoad,
    eptp: u64,
    pt_storage: &mut [u8],
    gdt_page: &mut [u8],
    gdt_gpa: u64,
    controls: AdjustedControls,
) -> Result<PreparedGuest, PrepareError> {
    let private = match role {
        GuestRole::Wan => layout.wan_private,
        GuestRole::Lan => layout.lan_private,
    };
    let boot_info_gpa = match role {
        GuestRole::Wan => layout.boot_info_wan.base,
        GuestRole::Lan => layout.boot_info_lan.base,
    };
    let tips_gpa = identity_tips(layout, role);
    let maps = spike_guest_maps(elf.link_base, private.base, elf.image_span, &tips_gpa)?;
    let pack = pack_guest_pt(pt_storage, &maps)?;

    if gdt_page.len() < 40 {
        return Err(PrepareError::Buffer);
    }
    write_guest_gdt(gdt_page, gdt_gpa + PAGE_SIZE);

    let rsp = private
        .base
        .saturating_add(private.size.saturating_sub(16));
    let tips = LaunchTips {
        entry_rip: elf.entry_va,
        entry_rsp: rsp,
        boot_info_gpa,
        guest_cr3: pack.cr3_gpa,
        eptp,
        guest_gdtr_base: gdt_gpa,
    };
    let launch = prepare_launch_image(tips, controls).map_err(|_| PrepareError::Launch)?;
    let _ = fill_boot_info(cfg, layout, role);
    Ok(PreparedGuest {
        role,
        entry_va: elf.entry_va,
        link_base: elf.link_base,
        image_span: elf.image_span,
        boot_info_gpa,
        private,
        launch,
    })
}

/// Permissive controls for host unit tests (no real VMX MSRs).
pub fn permissive_controls() -> AdjustedControls {
    let wants = ControlWants::spike();
    let caps = ControlCaps {
        pin: u64::from(u32::MAX) << 32,
        primary: u64::from(u32::MAX) << 32,
        secondary: u64::from(u32::MAX) << 32,
        exit: u64::from(u32::MAX) << 32,
        entry: u64::from(u32::MAX) << 32,
    };
    AdjustedControls::from_wants(wants, caps)
}

/// Guest id for role.
pub fn role_guest_id(role: GuestRole) -> GuestId {
    match role {
        GuestRole::Wan => GuestId::Wan,
        GuestRole::Lan => GuestId::Lan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guest_mem::build_layout;

    fn sample_cfg() -> FlatHvConfig {
        FlatHvConfig {
            wan_memory_mb: 2,
            lan_memory_mb: 2,
            wan_vcpus: 1,
            lan_vcpus: 1,
            ring_capacity_pow2: 8,
            frame_capacity: 64,
            gpa_window_base: 0x1000_0000,
            uart_io_port: 0x3F8,
            _uart_pad: 0,
            inject_queue_capacity: 8,
            nic_backend_inject: 1,
            wan_nic_mmio_gpa: 0x2000_0000,
            wan_nic_mmio_len: 0x1000,
            lan_nic_mmio_gpa: 0x2000_1000,
            lan_nic_mmio_len: 0x1000,
        }
    }

    #[test]
    fn boot_info_nic_mmio_from_flat_config() {
        let cfg = sample_cfg();
        let layout = build_layout(&cfg).unwrap();
        let wan = fill_boot_info(&cfg, &layout, GuestRole::Wan);
        let lan = fill_boot_info(&cfg, &layout, GuestRole::Lan);
        assert_eq!(wan.nic_mmio_gpa, 0x2000_0000);
        assert_eq!(wan.nic_mmio_len, 0x1000);
        assert_eq!(lan.nic_mmio_gpa, 0x2000_1000);
        assert_eq!(lan.nic_mmio_len, 0x1000);
    }

    #[test]
    fn install_boot_info_roundtrip() {
        let cfg = sample_cfg();
        let layout = build_layout(&cfg).unwrap();
        let boot = fill_boot_info(&cfg, &layout, GuestRole::Lan);
        let mut page = [0u8; 4096];
        install_boot_info(&mut page, &boot).unwrap();
        let loaded = unsafe { core::ptr::read(page.as_ptr() as *const BootInfo) };
        assert!(loaded.is_valid());
        assert_eq!(loaded.guest_role, GuestRole::Lan);
        assert_eq!(loaded.nic_mmio_len, 0x1000);
    }

    #[repr(C, align(4096))]
    struct PtBuf([u8; 16 * 1024]);

    #[test]
    fn prepare_guest_builds_launch_image() {
        let cfg = sample_cfg();
        let layout = build_layout(&cfg).unwrap();
        let elf = ElfLoad {
            entry_va: 0x203390,
            link_base: 0x200000,
            image_span: 0x100000,
        };
        let mut pt = PtBuf([0; 16 * 1024]);
        // Point CR3 at the buffer — pack_guest_pt uses storage address as CR3 GPA.
        let mut gdt = [0u8; 64];
        let gdt_gpa = layout.wan_private.base + layout.wan_private.size - 2 * PAGE_SIZE;
        let prep = prepare_guest(
            &cfg,
            &layout,
            GuestRole::Wan,
            &elf,
            0xEEE0_001E,
            &mut pt.0,
            &mut gdt,
            gdt_gpa,
            permissive_controls(),
        )
        .expect("prepare");
        assert_eq!(prep.entry_va, 0x203390);
        assert_eq!(prep.launch.eptp, 0xEEE0_001E);
        assert_eq!(prep.launch.entry_rdi, layout.boot_info_wan.base);
        assert_ne!(prep.launch.guest_cr3, 0);
    }
}
