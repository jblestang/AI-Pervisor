//! Reference partition guest ELF images generated at build time.

use hv_types::VmId;

include!(concat!(env!("OUT_DIR"), "/partition_elfs.rs"));

/// Serial marker emitted when the IN partition guest runs.
pub const GUEST_IN_RUNNING_MARKER: &str = "GUEST: in partition running";
/// Serial marker emitted when the MID partition guest runs.
pub const GUEST_MID_RUNNING_MARKER: &str = "GUEST: mid partition running";
/// Serial marker emitted when the OUT partition guest runs.
pub const GUEST_OUT_RUNNING_MARKER: &str = "GUEST: out partition running";
/// Serial marker emitted when a datapath-capable guest image runs.
pub const GUEST_DATAPATH_CAPABLE_MARKER: &str = "GUEST: datapath capable";

/// Which reference guest ELF image to install for a partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestElfKind {
    /// Standard partition smoke guest.
    Standard,
    /// Datapath-capable partition guest (build-time stub).
    Datapath,
    /// Real partition guest built from `guests/` source trees.
    Source,
}

/// Returns the reference ELF image bytes for a partition id.
pub fn reference_guest_elf(partition_id: &str) -> Option<&'static [u8]> {
    reference_guest_elf_for_kind(partition_id, GuestElfKind::Standard)
}

/// Returns the datapath-capable reference ELF image bytes for a partition id.
pub fn reference_datapath_guest_elf(partition_id: &str) -> Option<&'static [u8]> {
    reference_guest_elf_for_kind(partition_id, GuestElfKind::Datapath)
}

/// Returns the source-tree ELF image bytes for a partition id.
pub fn reference_guest_source_elf(partition_id: &str) -> Option<&'static [u8]> {
    reference_guest_elf_for_kind(partition_id, GuestElfKind::Source)
}

/// Returns the reference ELF image bytes for a partition id and image kind.
pub fn reference_guest_elf_for_kind(partition_id: &str, kind: GuestElfKind) -> Option<&'static [u8]> {
    match kind {
        GuestElfKind::Standard => match partition_id {
            "in" => Some(GUEST_IN_ELF),
            "mid" => Some(GUEST_MID_ELF),
            "out" => Some(GUEST_OUT_ELF),
            _ => None,
        },
        GuestElfKind::Datapath => match partition_id {
            "in" => Some(GUEST_IN_DATAPATH_ELF),
            "mid" => Some(GUEST_MID_DATAPATH_ELF),
            "out" => Some(GUEST_OUT_DATAPATH_ELF),
            _ => None,
        },
        GuestElfKind::Source if GUEST_SOURCE_ELFS_AVAILABLE => match partition_id {
            "in" => Some(GUEST_IN_SOURCE_ELF),
            "mid" => Some(GUEST_MID_SOURCE_ELF),
            "out" => Some(GUEST_OUT_SOURCE_ELF),
            _ => None,
        },
        GuestElfKind::Source => None,
    }
}

/// Returns the reference ELF image bytes for a VM id and image kind.
pub fn reference_guest_elf_for_vm_id(vm_id: VmId, kind: GuestElfKind) -> Option<&'static [u8]> {
    REFERENCE_GUEST_PARTITION_IDS
        .get(vm_id.raw() as usize)
        .and_then(|partition_id| reference_guest_elf_for_kind(partition_id, kind))
}

/// Ordered reference partition ids for multi-partition launch.
pub const REFERENCE_GUEST_PARTITION_IDS: &[&str] = &["in", "mid", "out"];

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::elf::parse_elf64;

    #[test]
    fn all_reference_datapath_partition_elfs_parse() {
        for partition in REFERENCE_GUEST_PARTITION_IDS {
            let bytes = reference_datapath_guest_elf(partition).expect("datapath elf");
            parse_elf64(bytes).expect("parse");
        }
    }

    #[test]
    fn source_partition_elfs_parse_when_embedded() {
        if !GUEST_SOURCE_ELFS_AVAILABLE {
            return;
        }
        for partition in REFERENCE_GUEST_PARTITION_IDS {
            let bytes = reference_guest_source_elf(partition).expect("source elf");
            parse_elf64(bytes).expect("parse source elf");
        }
    }
}
