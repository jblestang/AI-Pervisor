//! Shared SPSC ring buffer between WAN and LAN guests (`IF-RING-*`).
//!
//! Safe ring logic is annotated with Verus-oriented `requires!` / `ensures!`
//! stubs (see [`verus_stub`]). Real Verus verification is driven by
//! `cargo xtask verify` when the Verus toolchain is available.

#![no_std]

extern crate alloc;

mod spsc;
mod verus_stub;

pub use spsc::{FrameDesc, RingEmpty, RingFull, RingInitError, SpscRing};
