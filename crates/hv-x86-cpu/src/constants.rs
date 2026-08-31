//! Shared constants for x86 CPU instruction and resident install paths.

/// Environment variable that opts into host live privileged instruction execution.
pub const HV_X86_LIVE_INSTRUCTIONS_ENV: &str = "HV_X86_LIVE_INSTRUCTIONS";

/// Value of [`HV_X86_LIVE_INSTRUCTIONS_ENV`] that enables live execution.
pub const HV_X86_LIVE_INSTRUCTIONS_ENABLED: &str = "1";

/// Value of [`HV_X86_LIVE_INSTRUCTIONS_ENV`] that explicitly disables live execution.
pub const HV_X86_LIVE_INSTRUCTIONS_DISABLED: &str = "0";

/// x86 code-segment privilege-level mask (CPL in bits 1:0).
pub const X86_CPL_MASK: u16 = 0x3;

/// Ring-0 privilege level.
pub const X86_RING_0: u8 = 0;

/// VMCS revision identifier prefix length in bytes.
pub const VMXON_REVISION_PREFIX_BYTES: usize = 4;

/// VMCS encoded field number for the EPT pointer.
pub const VMCS_EPT_POINTER_FIELD: u32 = 0x0000_201A;

/// INVEPT type for single-context invalidation (all mappings for one EPT pointer).
pub const INVEPT_TYPE_SINGLE_CONTEXT: u64 = 1;

/// INVEPT descriptor size in bytes (128-bit in-memory operand).
pub const INVEPT_DESCRIPTOR_BYTES: usize = 16;

/// CR4.VMXE bit index.
pub const CR4_VMXE_BIT: u64 = 13;

/// Error when live VMXON is unavailable in the current environment.
pub const HV_X86_LIVE_VMXON_UNAVAILABLE: &str =
    "live VMXON requires HV_X86_LIVE_INSTRUCTIONS=1 in ring 0";

/// Error when live VT-d enable is unavailable in the current environment.
pub const HV_X86_LIVE_VTD_UNAVAILABLE: &str =
    "live VT-d enable requires HV_X86_LIVE_INSTRUCTIONS=1 in ring 0";

/// Error when live VMCS field programming is unavailable in the current environment.
pub const HV_X86_LIVE_VMCS_FIELDS_UNAVAILABLE: &str =
    "live VMCS field programming requires HV_X86_LIVE_INSTRUCTIONS=1 in ring 0";

/// Error when live VMLAUNCH is unavailable in the current environment.
pub const HV_X86_LIVE_VMLAUNCH_UNAVAILABLE: &str =
    "live VMLAUNCH requires HV_X86_LIVE_INSTRUCTIONS=1 in ring 0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_unavailable_messages_document_env_gate() {
        assert!(HV_X86_LIVE_VMXON_UNAVAILABLE.contains(HV_X86_LIVE_INSTRUCTIONS_ENV));
        assert!(HV_X86_LIVE_VMXON_UNAVAILABLE.contains(HV_X86_LIVE_INSTRUCTIONS_ENABLED));
        assert!(HV_X86_LIVE_VTD_UNAVAILABLE.contains(HV_X86_LIVE_INSTRUCTIONS_ENV));
    }
}
