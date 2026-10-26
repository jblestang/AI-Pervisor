//! Verus annotation stubs that compile on stable/`no_std` without the Verus toolchain.
//!
//! When `cargo xtask verify` can invoke Verus successfully, prefer real `verus!`
//! / `requires` / `ensures` from the Verus prelude. Until then these macros keep
//! proof intent visible in source without breaking host or freestanding builds.

#![allow(unused_macros)]

/// Precondition note. Evaluated only under Verus; ignored at runtime here.
macro_rules! requires {
    ($($tt:tt)*) => {};
}

/// Postcondition note. Evaluated only under Verus; ignored at runtime here.
macro_rules! ensures {
    ($($tt:tt)*) => {};
}

pub(crate) use ensures;
pub(crate) use requires;
