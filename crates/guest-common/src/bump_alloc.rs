//! Minimal bump allocator for freestanding guests (`no_std` + `alloc`).
//!
//! Bootstrap heap only: size is not a lab/demo constant (BDF / GPA / UART).
//! Task 6 may replace this with an hv-provided heap region from `DemoConfig`
//! via [`crate::BootInfo`] once mapped.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Bootstrap heap bytes for freestanding guest images (not a lab address).
const BOOTSTRAP_HEAP_LEN: usize = 256 * 1024;

struct Heap([UnsafeCell<u8>; BOOTSTRAP_HEAP_LEN]);

// SAFETY: freestanding guests are uniprocessor at the spike; bump `next` serializes.
unsafe impl Sync for Heap {}

static HEAP: Heap = {
    // const-init zeroed cells
    const CELL: UnsafeCell<u8> = UnsafeCell::new(0);
    Heap([CELL; BOOTSTRAP_HEAP_LEN])
};

/// Process-wide bump allocator for guest binaries.
pub struct BumpAlloc {
    next: AtomicUsize,
}

impl BumpAlloc {
    pub const fn new() -> Self {
        Self {
            next: AtomicUsize::new(0),
        }
    }
}

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();
        let mut offset = self.next.load(Ordering::Relaxed);
        loop {
            let aligned = (offset + align - 1) & !(align - 1);
            let Some(end) = aligned.checked_add(size) else {
                return null_mut();
            };
            if end > BOOTSTRAP_HEAP_LEN {
                return null_mut();
            }
            match self
                .next
                .compare_exchange_weak(offset, end, Ordering::SeqCst, Ordering::Relaxed)
            {
                Ok(_) => {
                    // SAFETY: `aligned..end` claimed exclusively via `next`; within HEAP.
                    return HEAP.0[aligned].get();
                }
                Err(cur) => offset = cur,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator: no reclaim (guest datapath does not free Inject frames
        // in the freestanding image until a fuller heap lands with hv).
    }
}
