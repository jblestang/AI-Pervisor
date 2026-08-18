//! Host CPUID capability probes for Gate C instruction seams.

#[cfg(test)]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
static TEST_FORCE_VMX_UNAVAILABLE: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static TEST_FORCE_EPT_UNAVAILABLE: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static TEST_FORCE_VTD_UNAVAILABLE: AtomicBool = AtomicBool::new(false);

/// Test hook forcing VMX CPUID probes to report unavailable.
#[cfg(test)]
pub fn test_force_vmx_unavailable(force: bool) {
    TEST_FORCE_VMX_UNAVAILABLE.store(force, Ordering::Relaxed);
}

/// Test hook forcing EPT CPUID probes to report unavailable.
#[cfg(test)]
pub fn test_force_ept_unavailable(force: bool) {
    TEST_FORCE_EPT_UNAVAILABLE.store(force, Ordering::Relaxed);
}

/// Test hook forcing VT-d CPU seam eligibility off.
#[cfg(test)]
pub fn test_force_vtd_unavailable(force: bool) {
    TEST_FORCE_VTD_UNAVAILABLE.store(force, Ordering::Relaxed);
}

/// Executes `cpuid` with the given leaf and sub-leaf.
#[cfg(target_arch = "x86_64")]
pub fn cpuid_raw(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let result = unsafe { core::arch::x86_64::__cpuid_count(leaf, subleaf) };
    (result.eax, result.ebx, result.ecx, result.edx)
}

/// Non-x86 targets report no CPUID results.
#[cfg(not(target_arch = "x86_64"))]
pub fn cpuid_raw(_leaf: u32, _subleaf: u32) -> (u32, u32, u32, u32) {
    (0, 0, 0, 0)
}

/// Returns whether leaf 1 ECX advertises VMX support.
pub fn cpuid_vmx_available() -> bool {
    #[cfg(test)]
    if TEST_FORCE_VMX_UNAVAILABLE.load(Ordering::Relaxed) {
        return false;
    }
    let (_eax, _ebx, ecx, _edx) = cpuid_raw(1, 0);
    ecx & (1 << 5) != 0
}

/// Returns whether leaf 0x480 ECX advertises EPT support.
pub fn cpuid_ept_available() -> bool {
    #[cfg(test)]
    if TEST_FORCE_EPT_UNAVAILABLE.load(Ordering::Relaxed) {
        return false;
    }
    if !cpuid_vmx_available() {
        return false;
    }
    let (_eax, _ebx, ecx, _edx) = cpuid_raw(0x480, 0);
    ecx & (1 << 6) != 0
}

/// Returns whether the host is eligible for VT-d CPU seams.
///
/// VT-d is validated via platform observation (DMAR / firmware inputs), not CPUID.
pub fn cpuid_vtd_available() -> bool {
    #[cfg(test)]
    if TEST_FORCE_VTD_UNAVAILABLE.load(Ordering::Relaxed) {
        return false;
    }
    cfg!(target_arch = "x86_64")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn cpuid_raw_returns_register_values_on_x86_64() {
        if !cfg!(target_arch = "x86_64") {
            return;
        }
        let (eax, ebx, ecx, edx) = cpuid_raw(0, 0);
        assert_ne!(eax, 0);
        assert_ne!(ebx, 0);
        assert_ne!(ecx, 0);
        assert_ne!(edx, 0);
    }

    #[test]
    fn cpuid_vmx_available_matches_leaf_one_ecx() {
        if !cfg!(target_arch = "x86_64") {
            assert!(!cpuid_vmx_available());
            return;
        }
        let (_eax, _ebx, ecx, _edx) = cpuid_raw(1, 0);
        assert_eq!(cpuid_vmx_available(), ecx & (1 << 5) != 0);
    }

    #[test]
    fn cpuid_ept_available_requires_vmx_and_leaf_480_ecx() {
        if !cfg!(target_arch = "x86_64") {
            assert!(!cpuid_ept_available());
            return;
        }
        if !cpuid_vmx_available() {
            assert!(!cpuid_ept_available());
            return;
        }
        let (_eax, _ebx, ecx, _edx) = cpuid_raw(0x480, 0);
        assert_eq!(cpuid_ept_available(), ecx & (1 << 6) != 0);
    }

    #[test]
    fn cpuid_vtd_available_is_architecture_gated() {
        assert_eq!(cpuid_vtd_available(), cfg!(target_arch = "x86_64"));
    }

    #[test]
    fn cpuid_test_force_hooks_toggle_availability() {
        test_force_vmx_unavailable(true);
        assert!(!cpuid_vmx_available());
        test_force_vmx_unavailable(false);

        test_force_ept_unavailable(true);
        assert!(!cpuid_ept_available());
        test_force_ept_unavailable(false);

        test_force_vtd_unavailable(true);
        assert!(!cpuid_vtd_available());
        test_force_vtd_unavailable(false);
    }
}
