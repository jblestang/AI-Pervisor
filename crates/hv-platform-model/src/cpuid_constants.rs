//! CPUID leaf bit positions used during platform observation.

/// CPUID leaf 1 ECX VMX bit.
pub const CPUID_1_ECX_VMX_BIT: u32 = 5;
/// CPUID leaf 1 ECX x2APIC bit.
pub const CPUID_1_ECX_X2APIC_BIT: u32 = 21;
/// CPUID leaf 1 EDX NX bit.
pub const CPUID_1_EDX_NX_BIT: u32 = 20;
/// CPUID leaf 0x8000_0007 EDX invariant TSC bit.
pub const CPUID_80000007_EDX_INVARIANT_TSC_BIT: u32 = 8;
/// CPUID leaf 0x480 ECX EPT bit.
pub const CPUID_480_ECX_EPT_BIT: u32 = 0;
/// CPUID leaf 0x480 ECX VPID bit.
pub const CPUID_480_ECX_VPID_BIT: u32 = 5;
/// CPUID leaf 0x480 EBX VMX preemption timer bit.
pub const CPUID_480_EBX_PREEMPTION_TIMER_BIT: u32 = 0;

/// Default supported guest page sizes in bytes.
pub const DEFAULT_PAGE_SIZES: [u64; 2] = [4096, 2_097_152];
