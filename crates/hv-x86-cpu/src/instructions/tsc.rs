//! Host timestamp counter reads for hypervisor-side relay measurement.

#![allow(clippy::needless_return)]

use crate::error::{CpuSeamError, CpuSeamErrorKind};

/// Reads the host timestamp counter when live execution is permitted.
#[cfg(all(target_arch = "x86_64", feature = "execute-instructions"))]
pub fn read_timestamp_counter() -> Result<u64, CpuSeamError> {
    use super::environment::live_execution_environment_ready;
    if !live_execution_environment_ready() {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::Unavailable,
            "live RDTSC requires live execution opt-in in ring 0",
        ));
    }
    #[cfg(not(any(test, coverage)))]
    {
        super::live_asm::rdtsc()
    }
    #[cfg(any(test, coverage))]
    {
        Err(CpuSeamError::new(
            CpuSeamErrorKind::ExecutionFailed,
            "RDTSC skipped in test harness",
        ))
    }
}

/// Without live execution support, host RDTSC is unavailable.
#[cfg(not(all(target_arch = "x86_64", feature = "execute-instructions")))]
pub fn read_timestamp_counter() -> Result<u64, CpuSeamError> {
    Err(CpuSeamError::new(
        CpuSeamErrorKind::Unavailable,
        "live RDTSC unavailable in this build",
    ))
}

/// Returns elapsed host TSC ticks when `end >= start`.
pub fn hypervisor_elapsed_tsc(start: u64, end: u64) -> u64 {
    end.saturating_sub(start)
}

/// Rejects inverted hypervisor TSC brackets before publish.
pub fn validate_hypervisor_tsc_bracket(start: u64, end: u64) -> Result<(), CpuSeamError> {
    if end < start {
        return Err(CpuSeamError::new(
            CpuSeamErrorKind::InvalidInput,
            "hypervisor TSC bracket is inverted",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_timestamp_counter_unavailable_without_live_environment() {
        assert!(read_timestamp_counter().is_err());
    }

    #[cfg(feature = "execute-instructions")]
    #[test]
    fn read_timestamp_counter_covers_live_path_in_test_harness() {
        use crate::instructions::environment::test_force_live_environment_ready;
        test_force_live_environment_ready(true);
        let result = read_timestamp_counter();
        test_force_live_environment_ready(false);
        assert!(result.is_err());
    }

    #[test]
    fn hypervisor_elapsed_tsc_saturates_on_underflow() {
        assert_eq!(hypervisor_elapsed_tsc(100, 250), 150);
        assert_eq!(hypervisor_elapsed_tsc(250, 100), 0);
    }

    #[test]
    fn validate_hypervisor_tsc_bracket_rejects_inverted_range() {
        assert!(validate_hypervisor_tsc_bracket(200, 100).is_err());
        assert!(validate_hypervisor_tsc_bracket(100, 100).is_ok());
        assert!(validate_hypervisor_tsc_bracket(100, 200).is_ok());
    }
}
