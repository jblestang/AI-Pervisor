//! Size and memory unit newtypes.

use crate::arith::{checked_mul_usize, ArithmeticError};

/// Byte size wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ByteSize(pub u64);

impl ByteSize {
    /// Creates a byte size from a raw count.
    pub const fn new(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Returns the raw byte count.
    pub const fn bytes(self) -> u64 {
        self.0
    }

    /// Converts to `usize` when the value fits the target platform.
    pub const fn as_usize(self) -> Option<usize> {
        if self.0 <= usize::MAX as u64 {
            Some(self.0 as usize)
        } else {
            None
        }
    }
}

/// Page count wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageCount(pub u64);

impl PageCount {
    /// Creates a page count from a raw value.
    pub const fn new(pages: u64) -> Self {
        Self(pages)
    }

    /// Returns the raw page count.
    pub const fn pages(self) -> u64 {
        self.0
    }
}

/// Gibibyte quantity used at configuration boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Gibibyte(pub u64);

impl Gibibyte {
    const BYTES_PER_GIB: u64 = 1024 * 1024 * 1024;

    /// Creates a gibibyte quantity from a whole GiB count.
    pub const fn new(gib: u64) -> Self {
        Self(gib)
    }

    /// Returns the whole gibibyte count.
    pub const fn gib(self) -> u64 {
        self.0
    }

    /// Converts gibibytes to bytes with overflow checking.
    pub fn to_bytes(self) -> Result<ByteSize, ArithmeticError> {
        let bytes = checked_mul_usize(self.0 as usize, Self::BYTES_PER_GIB as usize)?;
        Ok(ByteSize::new(bytes as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gib_to_bytes() {
        assert_eq!(
            Gibibyte::new(1).to_bytes(),
            Ok(ByteSize::new(1_073_741_824))
        );
    }

    #[test]
    fn byte_size_as_usize() {
        assert_eq!(ByteSize::new(64).as_usize(), Some(64));
    }

    #[test]
    fn checked_add_bytes() {
        use crate::checked_add_usize;
        let total = checked_add_usize(1, 2);
        assert_eq!(total, Ok(3));
    }
}
