//! Reference L2 Ethernet frames for guest datapath (no IP stack).

/// Minimum reference L2 frame size used by the in→mid→out relay benchmark.
pub const REFERENCE_L2_FRAME_LEN: usize = 64;

/// Builds the canonical reference L2 frame into `out`.
pub fn write_reference_l2_frame(out: &mut [u8]) -> Option<usize> {
    if out.len() < REFERENCE_L2_FRAME_LEN {
        return None;
    }
    out.fill(0);
    out[0..6].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    out[6..12].copy_from_slice(&[0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
    out[12..14].copy_from_slice(&[0x88, 0xB5]);
    let payload = b"HVDP-L2-BRIDGE";
    out[14..14 + payload.len()].copy_from_slice(payload);
    Some(REFERENCE_L2_FRAME_LEN)
}

/// Returns whether `frame` matches the reference L2 layout.
pub fn is_reference_l2_frame(frame: &[u8]) -> bool {
    if frame.len() < REFERENCE_L2_FRAME_LEN {
        return false;
    }
    let mut expected = [0u8; REFERENCE_L2_FRAME_LEN];
    write_reference_l2_frame(&mut expected);
    frame == expected
}

/// Scratch buffer holding one reference L2 frame for IPC enqueue.
pub fn reference_l2_frame_bytes() -> [u8; REFERENCE_L2_FRAME_LEN] {
    let mut frame = [0u8; REFERENCE_L2_FRAME_LEN];
    let _ = write_reference_l2_frame(&mut frame);
    frame
}
