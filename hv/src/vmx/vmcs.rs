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
    // 16-bit guest
    GuestEsSelector = 0x0800,
    GuestCsSelector = 0x0802,
    GuestSsSelector = 0x0804,
    GuestDsSelector = 0x0806,
    GuestFsSelector = 0x0808,
    GuestGsSelector = 0x080A,
    GuestLdtrSelector = 0x080C,
    GuestTrSelector = 0x080E,
    // 16-bit host
    HostEsSelector = 0x0C00,
    HostCsSelector = 0x0C02,
    HostSsSelector = 0x0C04,
    HostDsSelector = 0x0C06,
    HostFsSelector = 0x0C08,
    HostGsSelector = 0x0C0A,
    HostTrSelector = 0x0C0C,
    // 64-bit control / guest / host
    EptPointer = 0x201A,
    GuestPhysicalAddress = 0x2400,
    VmcsLinkPointer = 0x2800,
    GuestIa32Pat = 0x2804,
    GuestIa32Efer = 0x2806,
    HostIa32Pat = 0x2C00,
    HostIa32Efer = 0x2C02,
    // 32-bit control
    PinBasedVmExecControl = 0x4000,
    CpuBasedVmExecControl = 0x4002,
    VmExitControls = 0x400C,
    VmEntryControls = 0x4012,
    SecondaryVmExecControl = 0x401E,
    VmInstructionError = 0x4400,
    VmExitReason = 0x4402,
    GuestEsLimit = 0x4800,
    GuestCsLimit = 0x4802,
    GuestSsLimit = 0x4804,
    GuestDsLimit = 0x4806,
    GuestFsLimit = 0x4808,
    GuestGsLimit = 0x480A,
    GuestLdtrLimit = 0x480C,
    GuestTrLimit = 0x480E,
    GuestGdtrLimit = 0x4810,
    GuestIdtrLimit = 0x4812,
    GuestEsAccess = 0x4814,
    GuestCsAccess = 0x4816,
    GuestSsAccess = 0x4818,
    GuestDsAccess = 0x481A,
    GuestFsAccess = 0x481C,
    GuestGsAccess = 0x481E,
    GuestLdtrAccess = 0x4820,
    GuestTrAccess = 0x4822,
    GuestInterruptibility = 0x4824,
    GuestActivityState = 0x4826,
    GuestSysenterCs = 0x482A,
    // natural-width guest / host
    GuestCr0 = 0x6800,
    GuestCr3 = 0x6802,
    GuestCr4 = 0x6804,
    GuestEsBase = 0x6806,
    GuestCsBase = 0x6808,
    GuestSsBase = 0x680A,
    GuestDsBase = 0x680C,
    GuestFsBase = 0x680E,
    GuestGsBase = 0x6810,
    GuestLdtrBase = 0x6812,
    GuestTrBase = 0x6814,
    GuestGdtrBase = 0x6816,
    GuestIdtrBase = 0x6818,
    GuestDr7 = 0x681A,
    GuestRsp = 0x681C,
    GuestRip = 0x681E,
    GuestRflags = 0x6820,
    GuestPendingDbg = 0x6822,
    GuestSysenterEsp = 0x6824,
    GuestSysenterEip = 0x6826,
    HostCr0 = 0x6C00,
    HostCr3 = 0x6C02,
    HostCr4 = 0x6C04,
    HostFsBase = 0x6C06,
    HostGsBase = 0x6C08,
    HostTrBase = 0x6C0A,
    HostGdtrBase = 0x6C0C,
    HostIdtrBase = 0x6C0E,
    HostSysenterEsp = 0x6C10,
    HostSysenterEip = 0x6C12,
    HostRsp = 0x6C14,
    HostRip = 0x6C16,
    HostSysenterCs = 0x4C00,
    VmExitQualification = 0x6400,
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
