//! Overflow-safe arithmetic helpers.

/// Error returned when an arithmetic operation overflows or violates alignment rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticError {
    /// Addition overflowed.
    AddOverflow,
    /// Multiplication overflowed.
    MulOverflow,
    /// Subtraction underflowed.
    SubUnderflow,
    /// Alignment is not a power of two.
    InvalidAlignment,
    /// Value is not aligned to the requested boundary.
    Misaligned,
}

/// Checked addition for `usize`.
pub fn checked_add_usize(a: usize, b: usize) -> Result<usize, ArithmeticError> {
    a.checked_add(b).ok_or(ArithmeticError::AddOverflow)
}

/// Checked multiplication for `usize`.
pub fn checked_mul_usize(a: usize, b: usize) -> Result<usize, ArithmeticError> {
    a.checked_mul(b).ok_or(ArithmeticError::MulOverflow)
}

/// Returns `value` rounded up to `alignment` if `alignment` is a power of two.
pub fn align_up(value: usize, alignment: usize) -> Result<usize, ArithmeticError> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(ArithmeticError::InvalidAlignment);
    }
    let mask = alignment - 1;
    value
        .checked_add(mask)
        .ok_or(ArithmeticError::AddOverflow)
        .map(|v| v & !mask)
}

/// Returns `value` rounded down to `alignment` if `alignment` is a power of two.
pub fn align_down(value: usize, alignment: usize) -> Result<usize, ArithmeticError> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(ArithmeticError::InvalidAlignment);
    }
    Ok(value & !(alignment - 1))
}

/// Returns whether `value` is aligned to `alignment`.
pub fn is_aligned(value: usize, alignment: usize) -> Result<bool, ArithmeticError> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(ArithmeticError::InvalidAlignment);
    }
    Ok(value & (alignment - 1) == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_add_at_limit() {
        assert_eq!(checked_add_usize(1, 2), Ok(3));
        assert_eq!(
            checked_add_usize(usize::MAX, 1),
            Err(ArithmeticError::AddOverflow)
        );
    }

    #[test]
    fn checked_mul_at_limit() {
        assert_eq!(checked_mul_usize(2, 3), Ok(6));
        assert_eq!(
            checked_mul_usize(usize::MAX, 2),
            Err(ArithmeticError::MulOverflow)
        );
    }

    #[test]
    fn align_up_down() {
        assert_eq!(align_up(5, 4), Ok(8));
        assert_eq!(align_down(5, 4), Ok(4));
        assert_eq!(align_up(5, 3), Err(ArithmeticError::InvalidAlignment));
    }

    #[test]
    fn is_aligned_checks() {
        assert_eq!(is_aligned(8, 4), Ok(true));
        assert_eq!(is_aligned(9, 4), Ok(false));
        assert_eq!(is_aligned(9, 0), Err(ArithmeticError::InvalidAlignment));
    }

    proptest::proptest! {
        #[test]
        fn align_up_never_decreases(value in 0usize..1024, shift in 0u32..8) {
            use proptest::prop_assert;
            use proptest::prop_assert_eq;
            let alignment = 1usize << shift;
            if let Ok(aligned) = align_up(value, alignment) {
                prop_assert!(aligned >= value);
                prop_assert_eq!(aligned & (alignment - 1), 0);
            }
        }
    }
}
