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
pub use pci_vf::{InjectNic, NicBackend, SriovNic};
pub use ring::SpscRing;
pub use stats::{stats_off, Stats, StatsSnapshot, STATS_PAGE_SIZE};
pub use uart::{uart_write, uart_write_byte};

use boot_info::GuestRole as Role;

/// Serial magic token for `cargo xtask smoke` (`IF-SER-001`).
pub const SMOKE_OK: &[u8] = b"SMOKE_OK";

/// Bytes of [`SMOKE_OK`] (helper for hv / guests without embedding a lab string twice).
#[inline]
pub fn emit_smoke_ok_bytes() -> &'static [u8] {
    SMOKE_OK
}

/// Write [`SMOKE_OK`] via the UART tip in `boot` (LAN path after successful forward).
pub fn emit_smoke_ok(boot: &BootInfo) {
    uart_write(boot, SMOKE_OK);
    uart_write(boot, b"\n");
}

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
