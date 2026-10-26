//! Host-side forward tests: real SPSC rings + real Inject NicBackend (no mocks).
//! Covers TEST-UNIT-FWD / SWR-011 / SWR-014.

use guest_common::{
    forward_lan_poll, forward_wan_poll, ArenaCursor, BootInfo, GuestRole, Stats, BOOT_INFO_MAGIC,
    BOOT_INFO_VERSION,
};
use pci_vf::InjectNic;
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
