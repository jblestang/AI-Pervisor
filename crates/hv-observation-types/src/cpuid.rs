//! CPUID snapshot collected by the loader before hypervisor entry.

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

/// Raw CPUID leaves collected by the loader before hypervisor entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuidSnapshot {
    /// CPUID leaf 1 ECX register value.
    pub leaf1_ecx: u32,
    /// CPUID leaf 1 EDX register value.
    pub leaf1_edx: u32,
    /// CPUID leaf 1 EBX register value.
    pub leaf1_ebx: u32,
    /// CPUID leaf 0x8000_0007 EDX register value when available.
    pub leaf80000007_edx: Option<u32>,
    /// CPUID leaf 0x8000_0008 ECX register value when available.
    pub leaf80000008_ecx: Option<u32>,
    /// CPUID leaf 0x480 ECX register value when VMX is enabled.
    pub leaf480_ecx: Option<u32>,
    /// CPUID leaf 0x480 EBX register value when VMX is enabled.
    pub leaf480_ebx: Option<u32>,
}

impl CpuidSnapshot {
    /// Returns whether VMX is supported.
    pub fn vmx(&self) -> bool {
        bit_set(self.leaf1_ecx, CPUID_1_ECX_VMX_BIT)
    }

    /// Returns whether NX is supported.
    pub fn nx(&self) -> bool {
        bit_set(self.leaf1_edx, CPUID_1_EDX_NX_BIT)
    }

    /// Returns whether x2APIC is supported.
    pub fn x2apic(&self) -> bool {
        bit_set(self.leaf1_ecx, CPUID_1_ECX_X2APIC_BIT)
    }

    /// Returns whether invariant TSC is supported.
    pub fn invariant_tsc(&self) -> bool {
        self.leaf80000007_edx
            .is_some_and(|edx| bit_set(edx, CPUID_80000007_EDX_INVARIANT_TSC_BIT))
    }

    /// Returns whether EPT is supported.
    pub fn ept(&self) -> bool {
        self.leaf480_ecx
            .is_some_and(|ecx| bit_set(ecx, CPUID_480_ECX_EPT_BIT))
    }

    /// Returns whether VPID is supported.
    pub fn vpid(&self) -> bool {
        self.leaf480_ecx
            .is_some_and(|ecx| bit_set(ecx, CPUID_480_ECX_VPID_BIT))
    }

    /// Returns whether the VMX preemption timer is supported.
    pub fn vmx_preemption_timer(&self) -> bool {
        self.leaf480_ebx
            .is_some_and(|ebx| bit_set(ebx, CPUID_480_EBX_PREEMPTION_TIMER_BIT))
    }

    /// Returns the number of logical processors reported in leaf 1 EBX.
    pub fn logical_processors(&self) -> u32 {
        (self.leaf1_ebx >> 16) & 0xFF
    }

    /// Returns the number of physical cores per package when leaf 0x8000_0008 is available.
    pub fn cores_per_package(&self) -> Option<u32> {
        self.leaf80000008_ecx.map(|ecx| (ecx & 0xFF) + 1)
    }

    /// Returns whether SMT appears enabled from CPUID topology.
    pub fn smt_enabled(&self) -> bool {
        match self.cores_per_package() {
            Some(cores) => self.logical_processors() > cores,
            None => false,
        }
    }

    /// Returns the estimated physical core count.
    pub fn physical_cores(&self) -> u32 {
        self.cores_per_package()
            .unwrap_or_else(|| self.logical_processors().max(1))
    }
}

fn bit_set(value: u32, bit: u32) -> bool {
    (value & (1 << bit)) != 0
}
