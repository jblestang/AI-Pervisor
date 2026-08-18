//! Host execution environment probes for live privileged instructions.

#[cfg(test)]
use core::sync::atomic::{AtomicBool, Ordering};

/// Returns the current x86 privilege level (CPL) from the CS selector.
#[cfg(target_arch = "x86_64")]
pub fn current_privilege_level() -> u8 {
    let cs: u16;
    // SAFETY: reading CS is always safe on x86_64.
    unsafe {
        core::arch::asm!(
            "mov {0:x}, cs",
            out(reg) cs,
            options(nomem, nostack, preserves_flags),
        );
    }
    (cs & 0x3) as u8
}

/// Non-x86 targets report ring 3 so live execution stays disabled.
#[cfg(not(target_arch = "x86_64"))]
pub fn current_privilege_level() -> u8 {
    3
}

/// Returns whether live privileged instruction execution is enabled at runtime.
#[cfg(all(feature = "execute-instructions", feature = "std"))]
pub fn live_execution_runtime_enabled() -> bool {
    if firmware_live_execution_enabled() {
        return true;
    }
    match std::env::var("HV_X86_LIVE_INSTRUCTIONS") {
        Ok(value) => value == "1",
        Err(_) => false,
    }
}

/// Firmware builds opt in at compile time via `firmware-live-execution`.
#[cfg(all(feature = "execute-instructions", not(feature = "std")))]
pub fn live_execution_runtime_enabled() -> bool {
    firmware_live_execution_enabled()
}

/// Compile-time firmware opt-in for ring-0 live execution without env vars.
/// Returns whether this build opted into firmware ring-0 live execution.
#[cfg(feature = "firmware-live-execution")]
pub fn firmware_live_execution_enabled() -> bool {
    true
}

/// Returns whether this build opted into firmware ring-0 live execution.
#[cfg(not(feature = "firmware-live-execution"))]
pub fn firmware_live_execution_enabled() -> bool {
    false
}

/// When the compile-time feature is disabled, runtime live execution is off.
#[cfg(not(feature = "execute-instructions"))]
pub fn live_execution_runtime_enabled() -> bool {
    false
}

#[cfg(test)]
static TEST_FORCE_LIVE_ENVIRONMENT: AtomicBool = AtomicBool::new(false);

/// Test hook forcing live execution environment readiness.
#[cfg(test)]
pub fn test_force_live_environment_ready(force: bool) {
    TEST_FORCE_LIVE_ENVIRONMENT.store(force, Ordering::Relaxed);
}

/// Returns whether the current environment may attempt live VMX/EPT/VT-d instructions.
pub fn live_execution_environment_ready() -> bool {
    #[cfg(test)]
    if TEST_FORCE_LIVE_ENVIRONMENT.load(Ordering::Relaxed) {
        return true;
    }
    if !live_execution_runtime_enabled() {
        return false;
    }
    current_privilege_level() == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_privilege_level_is_valid_cpl_on_x86_64() {
        let cpl = current_privilege_level();
        assert!(cpl <= 3);
    }

    #[test]
    fn live_execution_runtime_disabled_without_env_var() {
        if cfg!(feature = "firmware-live-execution") {
            assert!(live_execution_runtime_enabled());
            return;
        }
        assert!(!live_execution_runtime_enabled());
    }

    #[test]
    fn live_execution_runtime_rejects_zero_env_var() {
        if cfg!(feature = "firmware-live-execution") {
            return;
        }
        std::env::set_var("HV_X86_LIVE_INSTRUCTIONS", "0");
        assert!(!live_execution_runtime_enabled());
        std::env::remove_var("HV_X86_LIVE_INSTRUCTIONS");
    }

    #[test]
    fn firmware_live_execution_enabled_reflects_feature() {
        if cfg!(feature = "firmware-live-execution") {
            assert!(firmware_live_execution_enabled());
        } else {
            assert!(!firmware_live_execution_enabled());
        }
    }

    #[test]
    fn live_execution_runtime_honors_env_var_when_set() {
        if cfg!(feature = "firmware-live-execution") {
            return;
        }
        std::env::set_var("HV_X86_LIVE_INSTRUCTIONS", "1");
        assert!(live_execution_runtime_enabled());
        std::env::remove_var("HV_X86_LIVE_INSTRUCTIONS");
    }

    #[test]
    fn live_execution_environment_honors_test_force_hook() {
        test_force_live_environment_ready(true);
        assert!(live_execution_environment_ready());
        test_force_live_environment_ready(false);
        assert!(!live_execution_environment_ready());
    }

    #[test]
    fn live_execution_environment_not_ready_in_userspace() {
        if cfg!(target_arch = "x86_64") && live_execution_runtime_enabled() {
            assert_ne!(current_privilege_level(), 0);
        }
        assert!(!live_execution_environment_ready());
    }
}
