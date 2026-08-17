//! Observation type tests.

#![allow(clippy::expect_used)]

use hv_observation_types::{
    CpuidSnapshot, CPUID_1_ECX_VMX_BIT, CPUID_1_ECX_X2APIC_BIT, CPUID_1_EDX_NX_BIT,
    CPUID_480_EBX_PREEMPTION_TIMER_BIT, CPUID_480_ECX_EPT_BIT, CPUID_480_ECX_VPID_BIT,
    CPUID_80000007_EDX_INVARIANT_TSC_BIT,
};

#[test]
fn cpuid_snapshot_reports_reference_capabilities() {
    let snapshot = CpuidSnapshot {
        leaf1_ecx: (1 << CPUID_1_ECX_VMX_BIT) | (1 << CPUID_1_ECX_X2APIC_BIT),
        leaf1_edx: 1 << CPUID_1_EDX_NX_BIT,
        leaf1_ebx: (4 << 16) | 4,
        leaf80000007_edx: Some(1 << CPUID_80000007_EDX_INVARIANT_TSC_BIT),
        leaf80000008_ecx: Some(3),
        leaf480_ecx: Some((1 << CPUID_480_ECX_EPT_BIT) | (1 << CPUID_480_ECX_VPID_BIT)),
        leaf480_ebx: Some(1 << CPUID_480_EBX_PREEMPTION_TIMER_BIT),
    };

    assert!(snapshot.vmx());
    assert!(snapshot.nx());
    assert!(snapshot.x2apic());
    assert!(snapshot.invariant_tsc());
    assert!(snapshot.ept());
    assert!(snapshot.vpid());
    assert!(snapshot.vmx_preemption_timer());
    assert_eq!(snapshot.physical_cores(), 4);
    assert!(!snapshot.smt_enabled());
}
