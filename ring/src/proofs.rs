//! Machine-checked lemmas for SPSC ring capacity / mask arithmetic (Verus).
//!
//! Exec `SpscRing` remains host/`no_std` compatible via [`crate::verus_stub`];
//! this module is verified by `cargo xtask verify` / `cargo verus verify`.

use vstd::prelude::*;

verus! {

/// Spec: non-zero power of two (matches `u32::is_power_of_two` for `n > 0`).
pub open spec fn is_pow2(n: u32) -> bool {
    n > 0 && (n & ((n - 1) as u32)) == 0
}

/// Slot index uses masking; for pow2 capacity, result is always `< capacity`.
pub proof fn lemma_mask_bounded(idx: u32, capacity: u32)
    requires
        is_pow2(capacity),
    ensures
        (idx & ((capacity - 1) as u32)) < capacity,
{
    assert((idx & ((capacity - 1) as u32)) < capacity) by (bit_vector)
        requires
            capacity > 0 && (capacity & ((capacity - 1) as u32)) == 0,
    ;
}

/// `is_pow2` implies capacity is non-zero (used by try_push/try_pop requires).
pub proof fn lemma_pow2_nonzero(capacity: u32)
    requires
        is_pow2(capacity),
    ensures
        capacity > 0,
{
}

/// Mask equals capacity - 1 for pow2 capacities (SPSC slot indexing).
pub proof fn lemma_mask_eq_capacity_minus_one(capacity: u32)
    requires
        is_pow2(capacity),
    ensures
        (capacity - 1) as u32 == capacity - 1,
{
}

} // verus!
