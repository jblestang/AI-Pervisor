//! Shared guest datapath: boot info blob, stats page, WAN/LAN forward (`IF-BOOT-*`, `IF-STATS-*`).
//!
//! Forward logic is exercised on the host with real [`ring::SpscRing`] and
//! [`pci_vf::InjectNic`] backends (not mocks). Guests link this crate `no_std`.

#![no_std]

extern crate alloc;

mod boot_info;
mod bump_alloc;
mod forward;
mod stats;
mod uart;

pub use boot_info::{
    BootInfo, GuestRole, NicBackendKind, BOOT_INFO_MAGIC, BOOT_INFO_VERSION,
};
pub use bump_alloc::BumpAlloc;
pub use forward::{forward_lan_poll, forward_wan_poll, ArenaCursor};
pub use pci_vf::{InjectNic, NicBackend};
pub use ring::SpscRing;
pub use stats::Stats;
pub use uart::{uart_write, uart_write_byte};

use boot_info::GuestRole as Role;

/// Guest entry helper: validate boot blob and emit a short UART banner.
///
/// Returns `false` if the blob is missing/invalid (caller should halt).
/// Datapath poll loops are role-specific ([`forward_wan_poll`] /
/// [`forward_lan_poll`]) and require hv-mapped rings/NIC (Task 6).
pub fn guest_entry_banner(boot: &BootInfo) -> bool {
    if !boot.is_valid() {
        return false;
    }
    let msg: &[u8] = match boot.guest_role {
        Role::Wan => b"guest-wan ready\n",
        Role::Lan => b"guest-lan ready\n",
    };
    uart_write(boot, msg);
    true
}
