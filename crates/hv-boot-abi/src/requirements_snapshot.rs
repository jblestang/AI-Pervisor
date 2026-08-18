//! Embedded platform requirements snapshot for UEFI hypervisor entry.

use hv_types::SHA256_DIGEST_BYTES;

/// Maximum page sizes stored in a requirements snapshot.
pub const MAX_REQUIREMENTS_PAGE_SIZES: usize = 4;

/// Maximum expected PCI devices stored in a requirements snapshot.
pub const MAX_REQUIREMENTS_PCI_DEVICES: usize = 8;

/// Architecture discriminator for [`RequirementsSnapshot::arch`].
pub const REQUIREMENTS_ARCH_X86_64: u32 = 0;

/// Feature requirement discriminator values.
pub const FEATURE_REQUIRED: u32 = 0;
/// Feature requirement level: preferred but not mandatory.
pub const FEATURE_PREFERRED: u32 = 1;
/// Feature requirement level: optional enhancement.
pub const FEATURE_OPTIONAL: u32 = 2;
/// Feature requirement level: explicitly disabled.
pub const FEATURE_DISABLED: u32 = 3;

/// SMT policy discriminator values.
pub const SMT_POLICY_DISABLED: u32 = 0;
/// SMT policy: one logical CPU per physical core, exclusive assignment.
pub const SMT_POLICY_EXCLUSIVE_CORE: u32 = 1;
/// SMT policy: siblings may be used within the same partition.
pub const SMT_POLICY_SAME_PARTITION_SIBLINGS: u32 = 2;
/// SMT policy: siblings may be shared across partitions.
pub const SMT_POLICY_ALLOW_CROSS_PARTITION: u32 = 3;

/// Fixed-size platform requirements snapshot embedded in the hypervisor image.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequirementsSnapshot {
    /// Required architecture discriminator.
    pub arch: u32,
    /// VMX requirement level.
    pub vmx: u32,
    /// EPT requirement level.
    pub ept: u32,
    /// VT-d requirement level.
    pub vtd: u32,
    /// Minimum physical core count.
    pub min_physical_cores: u32,
    /// SMT policy discriminator.
    pub smt_policy: u32,
    /// Minimum platform RAM in bytes.
    pub min_ram_bytes: u64,
    /// Interrupt remapping requirement level.
    pub interrupt_remapping: u32,
    /// x2APIC requirement level.
    pub x2apic: u32,
    /// Invariant TSC requirement level.
    pub invariant_tsc: u32,
    /// VPID requirement level.
    pub vpid: u32,
    /// VMX preemption timer requirement level.
    pub vmx_preemption_timer: u32,
    /// NX requirement level.
    pub nx: u32,
    /// Number of valid entries in [`Self::page_sizes`].
    pub page_size_count: u32,
    /// Required page sizes in bytes.
    pub page_sizes: [u64; MAX_REQUIREMENTS_PAGE_SIZES],
    /// Number of valid entries in [`Self::expected_pci`].
    pub expected_pci_count: u32,
    /// Expected PCI devices from configuration.
    pub expected_pci: [ExpectedPciSnapshot; MAX_REQUIREMENTS_PCI_DEVICES],
    /// Planned host physical base for the hypervisor private reserve (VMXON region).
    pub hypervisor_reserve_phys: u64,
    /// Planned hypervisor reserve size in bytes.
    pub hypervisor_reserve_bytes: u64,
    /// SHA-256 digest of the normalized configuration.
    pub config_digest: [u8; SHA256_DIGEST_BYTES],
}

/// Expected PCI device stored in a requirements snapshot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedPciSnapshot {
    /// Owning VM id.
    pub vm_id: u32,
    /// PCI segment number.
    pub segment: u16,
    /// PCI bus number.
    pub bus: u8,
    /// PCI device number.
    pub device: u8,
    /// PCI function number.
    pub function: u8,
    /// Reserved padding bytes.
    pub reserved: [u8; 3],
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn requirements_snapshot_layout_is_stable() {
        use core::mem::{align_of, size_of};

        assert_eq!(size_of::<ExpectedPciSnapshot>(), 12);
        assert_eq!(align_of::<RequirementsSnapshot>(), 8);
        assert!(size_of::<RequirementsSnapshot>() > size_of::<ExpectedPciSnapshot>());
    }
}
