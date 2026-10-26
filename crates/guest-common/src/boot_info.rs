//! Boot info blob prepared by `hv` from `DemoConfig` (`IF-BOOT-*`).
//!
//! Guests must not hardcode lab GPAs, UART ports, or VF tips. All address /
//! tip fields below are **placeholders** (zero) until `hv` fills them before
//! guest entry (Task 6). Schema constants (`magic`, `version`, role tags) are
//! part of the ICD contract, not lab configuration.

/// Magic identifying the spike boot-info blob (`IF-BOOT-001`): ASCII `HVBI`.
pub const BOOT_INFO_MAGIC: u32 = u32::from_le_bytes(*b"HVBI");

/// Schema version for [`BootInfo`] (`IF-BOOT-001`).
pub const BOOT_INFO_VERSION: u32 = 1;

/// Guest role tag in the boot blob (`IF-BOOT-001`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum GuestRole {
    Wan = 1,
    Lan = 2,
}

impl GuestRole {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::Wan),
            2 => Some(Self::Lan),
            _ => None,
        }
    }
}

/// Flattened NIC tip selection mirrored from config (`IF-BOOT-003`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum NicBackendKind {
    Inject = 0,
    Sriov = 1,
}

/// Boot configuration blob mapped into each guest (`IF-BOOT-*`).
///
/// Layout is `#[repr(C)]` so `hv` and guests share the same view. GPA fields
/// are guest-physical addresses filled by the hypervisor — never compile-time
/// lab literals in guest source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct BootInfo {
    pub magic: u32,
    pub version: u32,
    pub guest_role: GuestRole,

    // --- IF-BOOT-002 memory map (filled by hv) ---
    /// GPA of WAN→LAN SPSC ring header region.
    pub ring_wan_to_lan_gpa: u64,
    /// GPA of LAN→WAN SPSC ring header region.
    pub ring_lan_to_wan_gpa: u64,
    /// Slot capacity (power of two) for each ring — from `config` via hv.
    pub ring_capacity_pow2: u32,
    pub _ring_pad: u32,
    /// Payload arena GPA / length (`IF-RING-004`).
    pub arena_gpa: u64,
    pub arena_len: u64,
    /// Shared stats page GPA (`IF-STATS-*`).
    pub stats_gpa: u64,
    /// Optional self-pointer / mirror GPA.
    pub boot_info_gpa: u64,

    // --- IF-BOOT-003 NIC ---
    pub nic_backend: NicBackendKind,
    /// When Sriov: BDF tip is not embedded as a lab string here; hv fills MMIO
    /// queue tips below from `DemoConfig`. Zero until filled.
    pub nic_mmio_gpa: u64,
    pub nic_mmio_len: u64,
    pub inject_queue_capacity: u32,
    pub _nic_pad: u32,

    // --- IF-BOOT-004 UART ---
    /// I/O port for UART exits (`IF-UART-001`). Zero until hv fills from config.
    pub uart_port: u16,
    pub _uart_pad: u16,
    /// Optional hypercall id (0 = use I/O port).
    pub uart_hypercall_id: u32,
}

impl BootInfo {
    /// Construct a schema-valid placeholder blob for `role`.
    ///
    /// All GPA / port / MMIO fields are zero — **hv must fill them** from
    /// `DemoConfig` before entry. Safe for unit tests and for guest binaries
    /// that only need the type layout until Task 6.
    pub const fn placeholder(role: GuestRole) -> Self {
        Self {
            magic: BOOT_INFO_MAGIC,
            version: BOOT_INFO_VERSION,
            guest_role: role,
            ring_wan_to_lan_gpa: 0,
            ring_lan_to_wan_gpa: 0,
            ring_capacity_pow2: 0,
            _ring_pad: 0,
            arena_gpa: 0,
            arena_len: 0,
            stats_gpa: 0,
            boot_info_gpa: 0,
            nic_backend: NicBackendKind::Inject,
            nic_mmio_gpa: 0,
            nic_mmio_len: 0,
            inject_queue_capacity: 0,
            _nic_pad: 0,
            uart_port: 0,
            _uart_pad: 0,
            uart_hypercall_id: 0,
        }
    }

    /// True when magic + version match the spike schema.
    pub fn is_valid(&self) -> bool {
        self.magic == BOOT_INFO_MAGIC && self.version == BOOT_INFO_VERSION
    }
}
