//! Shared datapath guest runtime for reference in/mid/out partitions.

#![no_std]
#![allow(unsafe_code)]

mod boot;
mod e1000;
mod ipc;
mod layout;
mod relay;
mod serial;

pub use layout::Role;

/// Official synthetic frame payload for reference datapath smoke.
pub const SYNTHETIC_FRAME_PAYLOAD: &[u8] = b"HVDP18FR";

/// Serial marker for IN partition guests.
pub const GUEST_IN_RUNNING_MARKER: &str = "GUEST: in partition running";
/// Serial marker for MID partition guests.
pub const GUEST_MID_RUNNING_MARKER: &str = "GUEST: mid partition running";
/// Serial marker for OUT partition guests.
pub const GUEST_OUT_RUNNING_MARKER: &str = "GUEST: out partition running";
/// Serial marker for datapath-capable guests.
pub const GUEST_DATAPATH_CAPABLE_MARKER: &str = "GUEST: datapath capable";
/// Serial marker after sustained in→mid→out relay benchmark completes.
pub const GUEST_DATAPATH_RELAY_BENCHMARK_COMPLETE_MARKER: &str =
    "GUEST: datapath relay benchmark complete";

/// Frames each partition relays during the sustained benchmark loop.
pub const GUEST_RELAY_BENCHMARK_FRAMES: u32 = 64;

/// Runs one partition role using boot info when valid, otherwise reference layout.
pub fn run(role: Role, boot_info: *const u8) -> ! {
    let layout = boot::resolve_layout_for_role(role, boot_info);
    serial::write_line(role.running_marker());
    serial::write_line(GUEST_DATAPATH_CAPABLE_MARKER);
    relay::init_relay_measurement(boot_info);

    match role {
        Role::In => run_in_sustained(boot_info, &layout, GUEST_RELAY_BENCHMARK_FRAMES),
        Role::Mid => run_mid_sustained(boot_info, &layout, GUEST_RELAY_BENCHMARK_FRAMES),
        Role::Out => run_out_sustained(boot_info, &layout, GUEST_RELAY_BENCHMARK_FRAMES),
    }

    serial::write_line(GUEST_DATAPATH_RELAY_BENCHMARK_COMPLETE_MARKER);
    serial::write_byte(b'\n');
    halt();
}

fn run_in_sustained(boot_info: *const u8, layout: &layout::ResolvedLayout, frames: u32) {
    for _ in 0..frames {
        run_in(layout);
        let _ = boot_info;
    }
}

fn run_mid_sustained(boot_info: *const u8, layout: &layout::ResolvedLayout, frames: u32) {
    for _ in 0..frames {
        run_mid(layout);
        let _ = boot_info;
    }
}

fn run_out_sustained(boot_info: *const u8, layout: &layout::ResolvedLayout, frames: u32) {
    relay::set_relay_measurement_tsc_start(boot_info);
    for _ in 0..frames {
        if run_out(layout) {
            relay::record_relay_frame_completed(boot_info);
        }
    }
    relay::set_relay_measurement_tsc_end(boot_info);
}

fn run_in(layout: &layout::ResolvedLayout) {
    if let Some(mmio) = layout.e1000_mmio {
        e1000::tx_doorbell(mmio);
    }
    if let Some(queue) = layout.ipc_producer {
        ipc::enqueue(queue, SYNTHETIC_FRAME_PAYLOAD);
    }
}

fn run_mid(layout: &layout::ResolvedLayout) {
    if let (Some(consumer), Some(producer)) = (layout.ipc_consumer, layout.ipc_producer) {
        let mut buffer = [0u8; layout::REFERENCE_SLOT_SIZE as usize];
        if let Some(len) = ipc::dequeue(consumer, &mut buffer) {
            if let Some(payload) = buffer.get(0..len) {
                ipc::enqueue(producer, payload);
            }
        }
    }
}

fn run_out(layout: &layout::ResolvedLayout) -> bool {
    if let Some(queue) = layout.ipc_consumer {
        let mut buffer = [0u8; layout::REFERENCE_SLOT_SIZE as usize];
        if let Some(len) = ipc::dequeue(queue, &mut buffer) {
            if len == SYNTHETIC_FRAME_PAYLOAD.len() {
                let matched = buffer
                    .get(0..len)
                    .is_some_and(|payload| payload == SYNTHETIC_FRAME_PAYLOAD);
                if matched {
                    if let Some(mmio) = layout.e1000_mmio {
                        e1000::rx_advance(mmio);
                    }
                    return true;
                }
            }
        }
    }
    false
}

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}
