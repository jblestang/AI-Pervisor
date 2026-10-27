//! Host-testable VMCS execution-control adjustment (Intel SDM Vol. 3C).
//!
//! Capability MSRs encode allowed-0 (low 32) and allowed-1 (high 32) masks.
//! Hardware `vmwrite` of the adjusted values remains TCB (`docs/TCB.md`).

/// True when every bit required by `wanted` is allowed by the capability MSR value.
#[inline]
pub fn controls_allowed(msr: u64, wanted: u32) -> bool {
    let allowed0 = msr as u32;
    let allowed1 = (msr >> 32) as u32;
    // Bits in allowed0 must be 1 in the control; bits cleared in allowed1 must be 0.
    (wanted & allowed0) == allowed0 && (wanted & !allowed1) == 0
}

/// Adjust `wanted` to a value legal for `msr` (allowed-0 | wanted) & allowed-1.
#[inline]
pub fn adjust_controls(msr: u64, wanted: u32) -> u32 {
    let allowed0 = msr as u32;
    let allowed1 = (msr >> 32) as u32;
    (wanted | allowed0) & allowed1
}

/// Spike primary/secondary/pin/exit/entry wants (before MSR adjust).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlWants {
    pub pin: u32,
    pub primary: u32,
    pub secondary: u32,
    pub exit: u32,
    pub entry: u32,
}

impl ControlWants {
    /// Demonstrator defaults: EPT + unconditional I/O + HLT exiting + IA-32e guest.
    pub const fn spike() -> Self {
        // Pin: external-interrupt exiting (bit 0).
        let pin = 1 << 0;
        // Primary: HLT exiting (7), unconditional I/O (24), activate secondary (31).
        let primary = (1 << 7) | (1 << 24) | (1 << 31);
        // Secondary: enable EPT (1).
        let secondary = 1 << 1;
        // Exit: host address-space size / 64-bit host (9), ack interrupt on exit (15).
        let exit = (1 << 9) | (1 << 15);
        // Entry: IA-32e mode guest (9).
        let entry = 1 << 9;
        Self {
            pin,
            primary,
            secondary,
            exit,
            entry,
        }
    }
}

/// Capability MSR bundle used to adjust [`ControlWants`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlCaps {
    pub pin: u64,
    pub primary: u64,
    pub secondary: u64,
    pub exit: u64,
    pub entry: u64,
}

/// Fully adjusted control fields ready for `vmwrite`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdjustedControls {
    pub pin: u32,
    pub primary: u32,
    pub secondary: u32,
    pub exit: u32,
    pub entry: u32,
}

impl AdjustedControls {
    pub fn from_wants(wants: ControlWants, caps: ControlCaps) -> Self {
        Self {
            pin: adjust_controls(caps.pin, wants.pin),
            primary: adjust_controls(caps.primary, wants.primary),
            secondary: adjust_controls(caps.secondary, wants.secondary),
            exit: adjust_controls(caps.exit, wants.exit),
            entry: adjust_controls(caps.entry, wants.entry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjust_forces_allowed0_bits() {
        // allowed0 bit0 must be set; allowed1 allows bits 0 and 7.
        let msr = (0b1000_0001u64 << 32) | 0b1;
        assert_eq!(adjust_controls(msr, 0), 1);
        assert_eq!(adjust_controls(msr, 1 << 7), (1 << 7) | 1);
    }

    #[test]
    fn adjust_clears_disallowed_bits() {
        let msr = (0b11u64 << 32) | 0; // only bits 0..1 allowed as 1
        assert_eq!(adjust_controls(msr, 0b1111), 0b11);
    }

    #[test]
    fn controls_allowed_checks_masks() {
        let msr = (0b101u64 << 32) | 0b001;
        assert!(controls_allowed(msr, 0b001));
        assert!(controls_allowed(msr, 0b101));
        assert!(!controls_allowed(msr, 0b010)); // bit1 not in allowed1
        assert!(!controls_allowed(msr, 0)); // missing required allowed0 bit0
    }

    #[test]
    fn spike_wants_include_ept_and_io() {
        let w = ControlWants::spike();
        assert_ne!(w.primary & (1 << 24), 0, "unconditional I/O exiting");
        assert_ne!(w.primary & (1 << 31), 0, "secondary controls");
        assert_ne!(w.secondary & (1 << 1), 0, "EPT");
        assert_ne!(w.entry & (1 << 9), 0, "IA-32e guest");
        assert_ne!(w.exit & (1 << 9), 0, "64-bit host");
    }

    #[test]
    fn from_wants_applies_all_msrs() {
        let wants = ControlWants::spike();
        // Permissive caps: allowed1 = all ones, allowed0 = 0.
        let caps = ControlCaps {
            pin: u64::from(u32::MAX) << 32,
            primary: u64::from(u32::MAX) << 32,
            secondary: u64::from(u32::MAX) << 32,
            exit: u64::from(u32::MAX) << 32,
            entry: u64::from(u32::MAX) << 32,
        };
        let adj = AdjustedControls::from_wants(wants, caps);
        assert_eq!(adj.pin, wants.pin);
        assert_eq!(adj.primary, wants.primary);
        assert_eq!(adj.secondary, wants.secondary);
        assert_eq!(adj.exit, wants.exit);
        assert_eq!(adj.entry, wants.entry);
    }
}
