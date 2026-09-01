//! Minimal Intel e1000 MMIO writes for guest datapath smoke.

use hv_types::GuestPhysAddr;

const E1000_REG_TDT: u64 = 0x3818;
const E1000_REG_RDT: u64 = 0x2818;

/// Writes the TX doorbell (TDT increment).
pub fn tx_doorbell(mmio_base: GuestPhysAddr) {
    mmio_write32(mmio_base, E1000_REG_TDT, 1);
}

/// Advances RX tail after a frame is observed at OUT.
pub fn rx_advance(mmio_base: GuestPhysAddr) {
    mmio_write32(mmio_base, E1000_REG_RDT, 1);
}

fn mmio_write32(base: GuestPhysAddr, offset: u64, value: u32) {
    let address = base.raw().saturating_add(offset) as *mut u32;
    unsafe {
        core::ptr::write_volatile(address, value);
    }
}
