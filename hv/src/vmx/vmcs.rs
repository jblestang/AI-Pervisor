//! Per-vCPU VMCS allocation and field accessors.

use crate::ept::EptGuest;
use crate::guest_mem::GuestId;

/// Logical vCPU identity (one per guest in the demonstrator).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VcpuId {
    Wan = 0,
    Lan = 1,
}

impl VcpuId {
    pub fn guest_id(self) -> GuestId {
        match self {
            VcpuId::Wan => GuestId::Wan,
            VcpuId::Lan => GuestId::Lan,
        }
    }

    pub fn ept_guest(self) -> EptGuest {
        match self {
            VcpuId::Wan => EptGuest::Wan,
            VcpuId::Lan => EptGuest::Lan,
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Self::Wan),
            1 => Some(Self::Lan),
            _ => None,
        }
    }
}

/// Selected VMCS field encodings (Intel SDM Appendix B).
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmcsField {
    GuestRip = 0x681E,
    GuestRsp = 0x681C,
    GuestRflags = 0x6820,
    GuestCr0 = 0x6800,
    GuestCr3 = 0x6802,
    GuestCr4 = 0x6804,
    HostCr0 = 0x6C00,
    HostCr3 = 0x6C02,
    HostCr4 = 0x6C04,
    HostRip = 0x6C16,
    HostRsp = 0x6C14,
    EptPointer = 0x201A,
    VmExitReason = 0x4402,
    VmExitQualification = 0x6400,
    GuestPhysicalAddress = 0x2400,
    VmInstructionError = 0x4400,
    PinBasedVmExecControl = 0x4000,
    CpuBasedVmExecControl = 0x4002,
    SecondaryVmExecControl = 0x401E,
    VmExitControls = 0x400C,
    VmEntryControls = 0x4012,
}

/// 4 KiB VMCS region.
#[repr(C, align(4096))]
pub struct VmcsRegion {
    pub data: [u8; 4096],
}

impl VmcsRegion {
    pub const fn zeroed() -> Self {
        Self { data: [0; 4096] }
    }

    pub fn set_revision(&mut self, revision: u32) {
        self.data[0..4].copy_from_slice(&revision.to_le_bytes());
    }
}

/// Software vCPU control block (safe state + VMCS region).
pub struct Vcpu {
    pub id: VcpuId,
    pub vmcs: VmcsRegion,
    pub halted: bool,
    pub entry_gpa: u64,
    pub boot_info_gpa: u64,
    pub ept_pointer: u64,
}

impl Vcpu {
    pub fn new(id: VcpuId) -> Self {
        Self {
            id,
            vmcs: VmcsRegion::zeroed(),
            halted: false,
            entry_gpa: 0,
            boot_info_gpa: 0,
            ept_pointer: 0,
        }
    }

    pub fn halt(&mut self) {
        self.halted = true;
    }
}

/// VMPTRLD + VMWRITE helpers — freestanding only.
#[cfg(all(target_os = "none", target_arch = "x86_64"))]
pub mod hw {
    use super::{VmcsField, VmcsRegion};

    /// Load VMCS pointer.
    ///
    /// # Safety
    /// `region` must be a valid VMCS with revision set; VMXON already active.
    pub unsafe fn vmclear(region: &mut VmcsRegion) -> Result<(), ()> {
        let phys = region as *mut VmcsRegion as u64;
        let mut fail: u8;
        unsafe {
            core::arch::asm!(
                "vmclear [{}]",
                "setbe {}",
                in(reg) &phys,
                out(reg_byte) fail,
                options(nostack),
            );
        }
        if fail != 0 {
            Err(())
        } else {
            Ok(())
        }
    }

    /// # Safety
    /// See [`vmclear`].
    pub unsafe fn vmptrld(region: &mut VmcsRegion) -> Result<(), ()> {
        let phys = region as *mut VmcsRegion as u64;
        let mut fail: u8;
        unsafe {
            core::arch::asm!(
                "vmptrld [{}]",
                "setbe {}",
                in(reg) &phys,
                out(reg_byte) fail,
                options(nostack),
            );
        }
        if fail != 0 {
            Err(())
        } else {
            Ok(())
        }
    }

    /// # Safety
    /// Current VMCS must be loaded.
    pub unsafe fn vmwrite(field: VmcsField, value: u64) -> Result<(), ()> {
        let mut fail: u8;
        unsafe {
            core::arch::asm!(
                "vmwrite {val}, {field}",
                "setbe {fail}",
                val = in(reg) value,
                field = in(reg) field as u64,
                fail = out(reg_byte) fail,
                options(nostack),
            );
        }
        if fail != 0 {
            Err(())
        } else {
            Ok(())
        }
    }

    /// # Safety
    /// Current VMCS must be loaded.
    pub unsafe fn vmread(field: VmcsField) -> Result<u64, ()> {
        let mut value: u64;
        let mut fail: u8;
        unsafe {
            core::arch::asm!(
                "vmread {val}, {field}",
                "setbe {fail}",
                val = out(reg) value,
                field = in(reg) field as u64,
                fail = out(reg_byte) fail,
                options(nostack),
            );
        }
        if fail != 0 {
            Err(())
        } else {
            Ok(value)
        }
    }

    /// # Safety
    /// Guest/host state must be fully configured; interrupts managed by caller.
    pub unsafe fn vmlaunch() -> Result<(), ()> {
        let mut fail: u8;
        unsafe {
            core::arch::asm!(
                "vmlaunch",
                "setbe {}",
                out(reg_byte) fail,
                options(nostack),
            );
        }
        if fail != 0 {
            Err(())
        } else {
            Ok(())
        }
    }

    /// # Safety
    /// See [`vmlaunch`].
    pub unsafe fn vmresume() -> Result<(), ()> {
        let mut fail: u8;
        unsafe {
            core::arch::asm!(
                "vmresume",
                "setbe {}",
                out(reg_byte) fail,
                options(nostack),
            );
        }
        if fail != 0 {
            Err(())
        } else {
            Ok(())
        }
    }
}
