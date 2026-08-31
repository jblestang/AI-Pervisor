//! Host-side malicious IPC queue tests.

#![allow(clippy::expect_used)]

use hv_datapath::{
    DatapathErrorKind, IpcQueueView, REFERENCE_IPC_QUEUE_SLOTS, REFERENCE_IPC_SLOT_SIZE_BYTES,
};

#[test]
fn enqueue_rejects_oversized_payload() {
    let mut bytes = vec![0u8; hv_datapath::queue_storage_bytes(4, 64).expect("storage")];
    let mut queue = IpcQueueView::open(&mut bytes, 4, 64).expect("open");
    let oversized = vec![0u8; 65];
    let err = queue.enqueue(&oversized).expect_err("must fail");
    assert_eq!(err.kind, DatapathErrorKind::IpcViolation);
}

#[test]
fn dequeue_rejects_empty_queue() {
    let mut bytes = vec![0u8; hv_datapath::queue_storage_bytes(4, 64).expect("storage")];
    let mut queue = IpcQueueView::open(&mut bytes, 4, 64).expect("open");
    let mut out = [0u8; 64];
    let err = queue.dequeue(&mut out).expect_err("must fail");
    assert_eq!(err.kind, DatapathErrorKind::IpcViolation);
}

#[test]
fn open_rejects_truncated_shared_region() {
    let mut bytes = vec![0u8; 16];
    assert!(IpcQueueView::open(
        &mut bytes,
        REFERENCE_IPC_QUEUE_SLOTS,
        REFERENCE_IPC_SLOT_SIZE_BYTES
    )
    .is_err());
}
