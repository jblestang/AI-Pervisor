//! VMX region sizing constants.

/// Minimum VMXON region size required by Intel VMX.
pub const VMXON_REGION_MIN_BYTES: u64 = 4096;

/// Required alignment for VMXON regions.
pub const VMXON_REGION_ALIGNMENT_BYTES: u64 = 4096;
