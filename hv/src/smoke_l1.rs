//! L1 inject smoke: same WAN→LAN forward path as guests (`guest_common` + real rings).
//!
//! Used when Multiboot modules are present and config selects Inject (or inject
//! fallback). Emits `SMOKE_OK` on the platform UART once LAN TX counters advance
//! (`IF-SER-001`). Full L2 VMLAUNCH remains best-effort alongside this path.

use guest_common::{
    emit_smoke_ok_bytes, forward_lan_poll, forward_wan_poll, ArenaCursor, InjectNic, SpscRing, Stats,
};

use crate::config::FlatHvConfig;
use crate::uart::UartSink;

/// Run one inject frame through WAN→ring→LAN and write `SMOKE_OK` on success.
pub fn run_inject_smoke<S: UartSink>(cfg: &FlatHvConfig, sink: &mut S) -> bool {
    let cap = cfg.ring_capacity_pow2.max(8);
    let q = cfg.inject_queue_capacity.max(8) as usize;

    // Keep arenas in statics — freestanding boot stack is modest.
    static mut RING_W2L: [u8; 4096] = [0; 4096];
    static mut RING_L2W: [u8; 4096] = [0; 4096];
    static mut ARENA: [u8; 8192] = [0; 8192];
    static mut SCRATCH: [u8; 2048] = [0; 2048];
    // SAFETY: single-threaded L1 smoke owns these buffers exclusively.
    let ring_w2l = unsafe { &mut *core::ptr::addr_of_mut!(RING_W2L) };
    let ring_l2w = unsafe { &mut *core::ptr::addr_of_mut!(RING_L2W) };
    let arena = unsafe { &mut *core::ptr::addr_of_mut!(ARENA) };
    let scratch = unsafe { &mut *core::ptr::addr_of_mut!(SCRATCH) };
    ring_w2l.fill(0);
    ring_l2w.fill(0);
    arena.fill(0);

    let Ok(mut wan_to_lan) = SpscRing::new_in(ring_w2l, cap.min(128)) else {
        write_str(sink, b"hv: smoke ring init failed\n");
        return false;
    };
    let Ok(mut lan_to_wan) = SpscRing::new_in(ring_l2w, cap.min(128)) else {
        write_str(sink, b"hv: smoke ring init failed\n");
        return false;
    };

    let mut wan = InjectNic::with_capacity(q);
    let mut lan = InjectNic::with_capacity(q);
    let stats = Stats::new();
    let mut cursor = ArenaCursor::new();

    let frame = [0xA5u8; 64];
    if wan.inject_rx(&frame).is_err() {
        write_str(sink, b"hv: smoke inject_rx failed\n");
        return false;
    }

    let n_wan = forward_wan_poll(
        &mut wan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        arena,
        &mut cursor,
        scratch,
        &stats,
    );
    let n_lan = forward_lan_poll(
        &mut lan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        arena,
        &mut cursor,
        scratch,
        &stats,
    );

    if n_wan == 0 || n_lan == 0 || stats.lan_tx() == 0 {
        write_str(sink, b"hv: smoke datapath incomplete\n");
        return false;
    }

    write_str(sink, b"hv: inject smoke path ok\n");
    for &b in emit_smoke_ok_bytes() {
        sink.write_byte(b);
    }
    sink.write_byte(b'\n');
    true
}

fn write_str<S: UartSink>(sink: &mut S, s: &[u8]) {
    for &b in s {
        sink.write_byte(b);
    }
}
