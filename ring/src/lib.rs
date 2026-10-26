//! Shared SPSC ring buffer between WAN and LAN guests (`IF-RING-*`).
//!
//! Exec path uses [`verus_stub`] annotations for proof intent visible without
//! Verus. Machine-checked lemmas live in [`proofs`] and are verified by
//! `cargo xtask verify` (`cargo verus verify -p ring`).

#![no_std]

extern crate alloc;

mod proofs;
mod spsc;
mod verus_stub;

pub use spsc::{FrameDesc, RingEmpty, RingFull, RingInitError, SpscRing};
