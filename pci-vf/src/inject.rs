//! In-process Inject NIC — real frame queues for degraded lab (not a Sriov mock).

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::traits::{NicBackend, NicError};
use crate::verus_stub::{ensures, requires};

/// Production-degraded NIC: host/peer injects RX frames; guest TX lands in an outbound queue.
#[derive(Debug)]
pub struct InjectNic {
    capacity: usize,
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
}

impl InjectNic {
    /// Create an Inject backend with bounded RX/TX queues (capacity in frames).
    pub fn with_capacity(capacity: usize) -> Self {
        requires!(capacity > 0);
        let cap = if capacity == 0 { 1 } else { capacity };
        ensures!(true);
        Self {
            capacity: cap,
            rx: VecDeque::new(),
            tx: VecDeque::new(),
        }
    }

    /// Host/peer pushes a frame into the guest RX path.
    pub fn inject_rx(&mut self, frame: &[u8]) -> Result<(), NicError> {
        requires!(true);
        if self.rx.len() >= self.capacity {
            return Err(NicError::QueueFull);
        }
        self.rx.push_back(frame.to_vec());
        ensures!(self.rx.len() <= self.capacity);
        Ok(())
    }

    /// Host/peer drains one frame previously sent by [`NicBackend::try_send`].
    pub fn take_tx(&mut self) -> Option<Vec<u8>> {
        self.tx.pop_front()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl NicBackend for InjectNic {
    fn rx_ready(&self) -> bool {
        !self.rx.is_empty()
    }

    fn try_recv(&mut self, buf: &mut [u8]) -> Result<usize, NicError> {
        requires!(true);
        let Some(frame) = self.rx.front() else {
            return Err(NicError::QueueEmpty);
        };
        if buf.len() < frame.len() {
            return Err(NicError::BufferTooSmall);
        }
        let frame = self.rx.pop_front().expect("front checked");
        let n = frame.len();
        buf[..n].copy_from_slice(&frame);
        ensures!(n <= buf.len());
        Ok(n)
    }

    fn try_send(&mut self, frame: &[u8]) -> Result<(), NicError> {
        requires!(true);
        if self.tx.len() >= self.capacity {
            return Err(NicError::QueueFull);
        }
        self.tx.push_back(frame.to_vec());
        ensures!(self.tx.len() <= self.capacity);
        Ok(())
    }
}
