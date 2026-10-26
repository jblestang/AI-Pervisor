//! Host-side integration tests for the shared SPSC ring.
//! Real in-process memory — no mocks.

use ring::{FrameDesc, RingEmpty, RingFull, SpscRing};

#[test]
fn push_pop_roundtrip() {
    let mut arena = vec![0u8; 4096];
    let mut ring = SpscRing::new_in(&mut arena[..], 8).unwrap();
    let d = FrameDesc {
        offset: 0,
        len: 64,
    };
    ring.try_push(d).unwrap();
    assert_eq!(ring.try_pop().unwrap().len, 64);
}

#[test]
fn full_returns_error_without_overwrite() {
    let mut arena = vec![0u8; 4096];
    let mut ring = SpscRing::new_in(&mut arena[..], 2).unwrap();
    ring.try_push(FrameDesc {
        offset: 0,
        len: 64,
    })
    .unwrap();
    ring.try_push(FrameDesc {
        offset: 64,
        len: 64,
    })
    .unwrap();
    assert!(ring
        .try_push(FrameDesc {
            offset: 128,
            len: 64,
        })
        .is_err());
    // First descriptor must still be intact (no overwrite on full push).
    let first = ring.try_pop().unwrap();
    assert_eq!(first.offset, 0);
    assert_eq!(first.len, 64);
}

#[test]
fn empty_pop_errors() {
    let mut arena = vec![0u8; 4096];
    let mut ring = SpscRing::new_in(&mut arena[..], 4).unwrap();
    assert!(matches!(ring.try_pop(), Err(RingEmpty)));
}

#[test]
fn rejects_non_power_of_two_capacity() {
    let mut arena = vec![0u8; 4096];
    assert!(SpscRing::new_in(&mut arena[..], 3).is_err());
    assert!(SpscRing::new_in(&mut arena[..], 0).is_err());
}

#[test]
fn rejects_undersized_arena() {
    let mut arena = vec![0u8; 8];
    assert!(SpscRing::new_in(&mut arena[..], 8).is_err());
}

#[test]
fn wrap_around_preserves_order() {
    let mut arena = vec![0u8; 4096];
    let mut ring = SpscRing::new_in(&mut arena[..], 4).unwrap();
    for i in 0u32..4 {
        ring.try_push(FrameDesc {
            offset: i * 64,
            len: 64,
        })
        .unwrap();
    }
    assert!(matches!(
        ring.try_push(FrameDesc {
            offset: 999,
            len: 1
        }),
        Err(RingFull)
    ));
    for i in 0u32..4 {
        let d = ring.try_pop().unwrap();
        assert_eq!(d.offset, i * 64);
    }
    assert!(matches!(ring.try_pop(), Err(RingEmpty)));
    // After drain, ring is usable again (indices wrapped).
    ring.try_push(FrameDesc {
        offset: 42,
        len: 7,
    })
    .unwrap();
    let d = ring.try_pop().unwrap();
    assert_eq!(d.offset, 42);
    assert_eq!(d.len, 7);
}

#[test]
fn fifo_order_under_partial_fill() {
    let mut arena = vec![0u8; 4096];
    let mut ring = SpscRing::new_in(&mut arena[..], 8).unwrap();
    ring.try_push(FrameDesc {
        offset: 10,
        len: 1,
    })
    .unwrap();
    ring.try_push(FrameDesc {
        offset: 20,
        len: 2,
    })
    .unwrap();
    assert_eq!(ring.try_pop().unwrap().offset, 10);
    ring.try_push(FrameDesc {
        offset: 30,
        len: 3,
    })
    .unwrap();
    assert_eq!(ring.try_pop().unwrap().offset, 20);
    assert_eq!(ring.try_pop().unwrap().offset, 30);
}
