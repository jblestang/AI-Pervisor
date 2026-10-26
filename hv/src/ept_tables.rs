//! Software construction of EPT paging structures from map entries.
//!
//! Filling physical pages and loading EPTP into the VMCS is TCB (`unsafe` at
//! the call site in freestanding bring-up). The permission packing helpers here
//! are host-testable.

use crate::ept::{map_entries, EptLeafPerms, EptMapEntry};
use crate::PAGE_SIZE;
use isolation::{IsolationFault, MemoryMap};

/// One EPT PTE/PDE value (bits match SDM: R/W/X + phys addr).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EptPte(pub u64);

impl EptPte {
    pub fn leaf(hpa: u64, perms: EptLeafPerms) -> Self {
        // Bits 51:12 = HPA, low 3 = R/W/X; bit 7 (ignored for 4K) clear.
        let addr = hpa & !((PAGE_SIZE) - 1);
        Self(addr | perms.as_ept_bits())
    }

    pub fn is_present(self) -> bool {
        (self.0 & 0b111) != 0
    }
}

/// Identity GPA→HPA plan for a guest map (HPA = GPA for nested identity spike).
pub fn identity_entries(map: &MemoryMap) -> Result<alloc::vec::Vec<(EptMapEntry, EptPte)>, IsolationFault> {
    let entries = map_entries(map)?;
    let mut out = alloc::vec::Vec::with_capacity(entries.len());
    for e in entries {
        let pte = EptPte::leaf(e.gpa, e.perms);
        out.push((e, pte));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FlatHvConfig;
    use crate::ept::build_ept_intent;
    use crate::guest_mem::build_layout;
    use isolation::Perms;

    #[test]
    fn identity_ptes_carry_rw_for_shared_ring() {
        let cfg = FlatHvConfig {
            wan_memory_mb: 1,
            lan_memory_mb: 1,
            wan_vcpus: 1,
            lan_vcpus: 1,
            ring_capacity_pow2: 4,
            frame_capacity: 64,
            gpa_window_base: 0x2000_0000,
            uart_io_port: 0x3F8,
            inject_queue_capacity: 4,
            nic_backend_inject: true,
        };
        let layout = build_layout(&cfg).unwrap();
        let intent = build_ept_intent(&layout).unwrap();
        let pt = identity_entries(&intent.wan).unwrap();
        let ring = pt
            .iter()
            .find(|(e, _)| e.gpa == layout.ring_wan_to_lan.base)
            .expect("ring entry");
        assert!(ring.1.is_present());
        assert_eq!(ring.0.perms, EptLeafPerms::from_isolation(Perms::READ | Perms::WRITE));
        assert_eq!(ring.1.0 & 0b111, 0b011);
    }
}
