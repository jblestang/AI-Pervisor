//! Reference L2 Ethernet frames for independent host-interface attach (no IP stack).

/// Minimum reference L2 frame size used by the in→mid→out relay benchmark.
pub const REFERENCE_L2_FRAME_LEN: usize = 64;

/// Ethertype for hypervisor L2 relay frames (local experimental, not IP).
pub const L2_RELAY_FRAME_ETHERTYPE: [u8; 2] = [0x88, 0xB5];

/// Writes the canonical reference L2 frame into `out`.
#[allow(clippy::indexing_slicing)]
pub fn write_reference_l2_frame(out: &mut [u8]) -> Result<usize, ()> {
    if out.len() < REFERENCE_L2_FRAME_LEN {
        return Err(());
    }
    out.fill(0);
    // Broadcast destination.
    out[0..6].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    // Locally administered source MAC.
    out[6..12].copy_from_slice(&[0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
    out[12..14].copy_from_slice(&L2_RELAY_FRAME_ETHERTYPE);
    let payload = b"HVDP-L2-BRIDGE";
    out[14..14 + payload.len()].copy_from_slice(payload);
    Ok(REFERENCE_L2_FRAME_LEN)
}

/// Returns whether `frame` matches the reference L2 layout and payload prefix.
pub fn is_reference_l2_frame(frame: &[u8]) -> bool {
    if frame.len() < REFERENCE_L2_FRAME_LEN {
        return false;
    }
    let mut expected = [0u8; REFERENCE_L2_FRAME_LEN];
    write_reference_l2_frame(&mut expected).ok();
    frame == expected
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn reference_l2_frame_has_expected_length_and_ethertype() {
        let mut frame = [0u8; REFERENCE_L2_FRAME_LEN];
        let len = write_reference_l2_frame(&mut frame).expect("write");
        assert_eq!(len, REFERENCE_L2_FRAME_LEN);
        assert_eq!(&frame[12..14], L2_RELAY_FRAME_ETHERTYPE);
        assert!(is_reference_l2_frame(&frame));
    }
}
