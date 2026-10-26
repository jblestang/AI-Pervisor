//! Host-side forward tests: real SPSC rings + real Inject NicBackend (no mocks).
//! Covers TEST-UNIT-FWD / SWR-011 / SWR-014.

use guest_common::{
    emit_smoke_ok, emit_smoke_ok_bytes, forward_lan_poll, forward_wan_poll, guest_entry_banner,
    ArenaCursor, BootInfo, GuestRole, Stats, BOOT_INFO_MAGIC, BOOT_INFO_VERSION, SMOKE_OK,
};
use pci_vf::{InjectNic, NicBackend, NicError};
use ring::{FrameDesc, SpscRing};

fn frame_64(marker: u8) -> [u8; 64] {
    let mut f = [marker; 64];
    f[0] = b'F';
    f[1] = marker;
    f
}

#[test]
fn boot_info_placeholders_are_zero_until_hv_fills() {
    let bi = BootInfo::placeholder(GuestRole::Wan);
    assert_eq!(bi.magic, BOOT_INFO_MAGIC);
    assert_eq!(bi.version, BOOT_INFO_VERSION);
    assert_eq!(bi.guest_role, GuestRole::Wan);
    // GPAs / UART / NIC tips are filled by hv from DemoConfig — not lab literals in guests.
    assert_eq!(bi.ring_wan_to_lan_gpa, 0);
    assert_eq!(bi.ring_lan_to_wan_gpa, 0);
    assert_eq!(bi.arena_gpa, 0);
    assert_eq!(bi.arena_len, 0);
    assert_eq!(bi.stats_gpa, 0);
    assert_eq!(bi.uart_port, 0);
    assert!(bi.is_valid());
}

#[test]
fn wan_to_lan_forward_with_real_ring_and_inject() {
    let mut ring_w2l_mem = vec![0u8; 4096];
    let mut ring_l2w_mem = vec![0u8; 4096];
    let mut arena = vec![0u8; 8192];
    let mut wan = InjectNic::with_capacity(8);
    let mut lan = InjectNic::with_capacity(8);
    let stats = Stats::new();
    let mut cursor = ArenaCursor::new();
    let mut scratch = [0u8; 2048];

    let mut wan_to_lan = SpscRing::new_in(&mut ring_w2l_mem, 8).unwrap();
    let mut lan_to_wan = SpscRing::new_in(&mut ring_l2w_mem, 8).unwrap();

    let frame = frame_64(0xA1);
    wan.inject_rx(&frame).unwrap();

    // WAN: host NIC → ring (WAN→LAN direction).
    let n = forward_wan_poll(
        &mut wan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    assert!(n > 0, "WAN should forward at least one frame toward LAN");
    assert_eq!(stats.wan_rx(), 1);
    assert_eq!(stats.ring_fwd_wan_to_lan(), 1);

    // LAN: ring → host NIC.
    let n = forward_lan_poll(
        &mut lan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    assert!(n > 0, "LAN should deliver ring frame to NIC");
    assert_eq!(stats.lan_tx(), 1);

    let out = lan.take_tx().expect("LAN TX frame");
    assert_eq!(&out[..], &frame[..]);
}

#[test]
fn lan_to_wan_forward_reverse_path() {
    let mut ring_w2l_mem = vec![0u8; 4096];
    let mut ring_l2w_mem = vec![0u8; 4096];
    let mut arena = vec![0u8; 8192];
    let mut wan = InjectNic::with_capacity(8);
    let mut lan = InjectNic::with_capacity(8);
    let stats = Stats::new();
    let mut cursor = ArenaCursor::new();
    let mut scratch = [0u8; 2048];

    let mut wan_to_lan = SpscRing::new_in(&mut ring_w2l_mem, 8).unwrap();
    let mut lan_to_wan = SpscRing::new_in(&mut ring_l2w_mem, 8).unwrap();

    let frame = frame_64(0xB2);
    lan.inject_rx(&frame).unwrap();

    let n = forward_lan_poll(
        &mut lan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    assert!(n > 0);
    assert_eq!(stats.lan_rx(), 1);
    assert_eq!(stats.ring_fwd_lan_to_wan(), 1);

    let n = forward_wan_poll(
        &mut wan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    assert!(n > 0);
    assert_eq!(stats.wan_tx(), 1);

    let out = wan.take_tx().expect("WAN TX frame");
    assert_eq!(&out[..], &frame[..]);
}

#[test]
fn ring_full_drops_and_counts() {
    let mut ring_w2l_mem = vec![0u8; 4096];
    let mut ring_l2w_mem = vec![0u8; 4096];
    let mut arena = vec![0u8; 65_536];
    let mut wan = InjectNic::with_capacity(8);
    let stats = Stats::new();
    let mut cursor = ArenaCursor::new();
    let mut scratch = [0u8; 2048];

    // Capacity 2 → two pushes fill; third must drop (SWR-013).
    let mut wan_to_lan = SpscRing::new_in(&mut ring_w2l_mem, 2).unwrap();
    let mut lan_to_wan = SpscRing::new_in(&mut ring_l2w_mem, 2).unwrap();

    for i in 0..3u8 {
        wan.inject_rx(&frame_64(i)).unwrap();
        forward_wan_poll(
            &mut wan,
            &mut wan_to_lan,
            &mut lan_to_wan,
            &mut arena,
            &mut cursor,
            &mut scratch,
            &stats,
        );
    }

    assert_eq!(stats.wan_rx(), 3);
    assert_eq!(stats.ring_fwd_wan_to_lan(), 2);
    assert_eq!(stats.ring_drop_full(), 1);
}

#[test]
fn bidirectional_one_round_each_way() {
    let mut ring_w2l_mem = vec![0u8; 4096];
    let mut ring_l2w_mem = vec![0u8; 4096];
    let mut arena = vec![0u8; 8192];
    let mut wan = InjectNic::with_capacity(8);
    let mut lan = InjectNic::with_capacity(8);
    let stats = Stats::new();
    let mut cursor = ArenaCursor::new();
    let mut scratch = [0u8; 2048];

    let mut wan_to_lan = SpscRing::new_in(&mut ring_w2l_mem, 8).unwrap();
    let mut lan_to_wan = SpscRing::new_in(&mut ring_l2w_mem, 8).unwrap();

    let up = frame_64(0x11);
    let down = frame_64(0x22);
    wan.inject_rx(&up).unwrap();
    lan.inject_rx(&down).unwrap();

    forward_wan_poll(
        &mut wan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    forward_lan_poll(
        &mut lan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    // Second poll each side drains the peer direction that the first poll may have
    // only enqueued (WAN RX→ring and LAN RX→ring in the first pair of polls).
    forward_wan_poll(
        &mut wan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    forward_lan_poll(
        &mut lan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );

    assert_eq!(&lan.take_tx().unwrap()[..], &up[..]);
    assert_eq!(&wan.take_tx().unwrap()[..], &down[..]);
    assert_eq!(stats.ring_fwd_wan_to_lan(), 1);
    assert_eq!(stats.ring_fwd_lan_to_wan(), 1);
}

#[test]
fn unused_frame_desc_type_linked() {
    // Sanity: guest-common tests sit on the real ring ABI.
    let d = FrameDesc {
        offset: 0,
        len: 64,
    };
    assert_eq!(d.len, 64);
}

#[test]
fn guest_entry_banner_and_smoke_helpers() {
    assert_eq!(emit_smoke_ok_bytes(), SMOKE_OK);
    let wan = BootInfo::placeholder(GuestRole::Wan);
    assert!(guest_entry_banner(&wan));
    let lan = BootInfo::placeholder(GuestRole::Lan);
    assert!(guest_entry_banner(&lan));
    // uart_port == 0 → UART helpers are no-ops (safe on host).
    emit_smoke_ok(&wan);

    let mut bad = BootInfo::placeholder(GuestRole::Wan);
    bad.magic = 0;
    assert!(!guest_entry_banner(&bad));
}

#[test]
fn guest_role_from_u32_and_stats_byte_counters() {
    assert_eq!(GuestRole::from_u32(1), Some(GuestRole::Wan));
    assert_eq!(GuestRole::from_u32(2), Some(GuestRole::Lan));
    assert_eq!(GuestRole::from_u32(0), None);

    let stats = Stats::default();
    assert_eq!(stats.bytes_wan_rx(), 0);
    assert_eq!(stats.bytes_wan_tx(), 0);
    assert_eq!(stats.bytes_lan_rx(), 0);
    assert_eq!(stats.bytes_lan_tx(), 0);

    let mut ring_w2l_mem = vec![0u8; 4096];
    let mut ring_l2w_mem = vec![0u8; 4096];
    let mut arena = vec![0u8; 8192];
    let mut wan = InjectNic::with_capacity(4);
    let mut cursor = ArenaCursor::new();
    let mut scratch = [0u8; 2048];
    let mut wan_to_lan = SpscRing::new_in(&mut ring_w2l_mem, 4).unwrap();
    let mut lan_to_wan = SpscRing::new_in(&mut ring_l2w_mem, 4).unwrap();
    let frame = frame_64(0x7E);
    wan.inject_rx(&frame).unwrap();
    forward_wan_poll(
        &mut wan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    assert_eq!(stats.bytes_wan_rx(), 64);
}

#[test]
fn arena_cursor_wrap_and_oversized_frame() {
    let mut cursor = ArenaCursor::new();
    assert!(cursor.alloc(64, 0).is_none());
    assert!(cursor.alloc(64, 128).is_none());
    let (o1, l1) = cursor.alloc(64, 40).unwrap();
    assert_eq!((o1, l1), (0, 40));
    // Remaining 24 bytes — next 40 must wrap to 0.
    let (o2, l2) = cursor.alloc(64, 40).unwrap();
    assert_eq!((o2, l2), (0, 40));
}

#[test]
fn forward_handles_buffer_too_small_and_corrupt_desc() {
    let mut ring_w2l_mem = vec![0u8; 4096];
    let mut ring_l2w_mem = vec![0u8; 4096];
    let mut arena = vec![0u8; 128];
    let mut wan = InjectNic::with_capacity(4);
    let mut lan = InjectNic::with_capacity(1);
    let stats = Stats::new();
    let mut cursor = ArenaCursor::new();
    // Scratch too small for a 64 B frame → BufferTooSmall → drop.
    let mut scratch = [0u8; 8];
    let mut wan_to_lan = SpscRing::new_in(&mut ring_w2l_mem, 4).unwrap();
    let mut lan_to_wan = SpscRing::new_in(&mut ring_l2w_mem, 4).unwrap();

    wan.inject_rx(&frame_64(0x01)).unwrap();
    let n = forward_wan_poll(
        &mut wan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    assert_eq!(n, 0);
    assert!(stats.ring_drop_full() >= 1);

    // Corrupt descriptor past arena end → drop on LAN pop path.
    let mut scratch2 = [0u8; 2048];
    wan_to_lan
        .try_push(FrameDesc {
            offset: 10_000,
            len: 64,
        })
        .unwrap();
    let before = stats.ring_drop_full();
    forward_lan_poll(
        &mut lan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch2,
        &stats,
    );
    assert!(stats.ring_drop_full() > before);
}

#[test]
fn lan_tx_drop_when_inject_tx_queue_full() {
    let mut ring_w2l_mem = vec![0u8; 4096];
    let mut ring_l2w_mem = vec![0u8; 4096];
    let mut arena = vec![0u8; 8192];
    let mut lan = InjectNic::with_capacity(1);
    let stats = Stats::new();
    let mut cursor = ArenaCursor::new();
    let mut scratch = [0u8; 2048];
    let mut wan_to_lan = SpscRing::new_in(&mut ring_w2l_mem, 4).unwrap();
    let mut lan_to_wan = SpscRing::new_in(&mut ring_l2w_mem, 4).unwrap();

    // Fill LAN TX queue first.
    lan.try_send(b"busy").unwrap();
    assert!(matches!(lan.try_send(b"x"), Err(NicError::QueueFull)));

    // Place a valid frame on WAN→LAN ring; LAN pop→send must drop on QueueFull.
    let frame = frame_64(0x55);
    arena[..64].copy_from_slice(&frame);
    wan_to_lan
        .try_push(FrameDesc {
            offset: 0,
            len: 64,
        })
        .unwrap();
    let before = stats.ring_drop_full();
    forward_lan_poll(
        &mut lan,
        &mut wan_to_lan,
        &mut lan_to_wan,
        &mut arena,
        &mut cursor,
        &mut scratch,
        &stats,
    );
    assert!(stats.ring_drop_full() > before);
}
