//! Machine-checked lemmas for region / permission geometry (Verus).
//!
//! Exec [`crate::MemoryMap`] stays on host/`no_std` via stubs; this module is
//! verified by `cargo xtask verify` / `cargo verus verify`.

use vstd::prelude::*;

verus! {

/// Spec permissions bitset (mirrors exec `Perms.bits`).
pub open spec fn perms_contains(have: u8, need: u8) -> bool {
    (have & need) == need
}

pub proof fn lemma_perms_subset_refl(p: u8)
    ensures
        perms_contains(p, p),
{
    assert((p & p) == p) by (bit_vector);
}

pub proof fn lemma_perms_empty_only_zero(need: u8)
    ensures
        perms_contains(0u8, need) ==> need == 0u8,
{
    assert(((0u8 & need) == need) ==> need == 0u8) by (bit_vector);
}

/// Half-open ranges `[base, base+size)` intersect (caller ensures no overflow).
pub open spec fn ranges_overlap(a_base: int, a_size: int, b_base: int, b_size: int) -> bool {
    a_size > 0 && b_size > 0 && a_base < b_base + b_size && b_base < a_base + a_size
}

pub proof fn lemma_overlap_symmetric(a_base: int, a_size: int, b_base: int, b_size: int)
    ensures
        ranges_overlap(a_base, a_size, b_base, b_size) == ranges_overlap(
            b_base,
            b_size,
            a_base,
            a_size,
        ),
{
}

/// Adjacent regions (`b_base == a_base + a_size`) do not overlap.
pub proof fn lemma_adjacent_no_overlap(a_base: int, a_size: int, b_size: int)
    requires
        a_size > 0,
        b_size > 0,
    ensures
        !ranges_overlap(a_base, a_size, a_base + a_size, b_size),
{
}

/// Access `[addr, addr+len)` contained in `[base, base+size)`.
pub open spec fn range_contains(base: int, size: int, addr: int, len: int) -> bool {
    size > 0 && len > 0 && addr >= base && addr + len <= base + size
}

pub proof fn lemma_contains_implies_in_bounds(base: int, size: int, addr: int, len: int)
    requires
        range_contains(base, size, addr, len),
    ensures
        addr >= base,
        addr + len <= base + size,
{
}

} // verus!
