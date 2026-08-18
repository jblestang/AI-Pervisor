//! Reference partition guest ELF images generated at build time.

include!(concat!(env!("OUT_DIR"), "/partition_elfs.rs"));

/// Serial marker emitted when the IN partition guest runs.
pub const GUEST_IN_RUNNING_MARKER: &str = "GUEST: in partition running";
/// Serial marker emitted when the MID partition guest runs.
pub const GUEST_MID_RUNNING_MARKER: &str = "GUEST: mid partition running";
/// Serial marker emitted when the OUT partition guest runs.
pub const GUEST_OUT_RUNNING_MARKER: &str = "GUEST: out partition running";

/// Returns the reference ELF image bytes for a partition id.
pub fn reference_guest_elf(partition_id: &str) -> Option<&'static [u8]> {
    match partition_id {
        "in" => Some(GUEST_IN_ELF),
        "mid" => Some(GUEST_MID_ELF),
        "out" => Some(GUEST_OUT_ELF),
        _ => None,
    }
}

/// Ordered reference partition ids for multi-partition launch.
pub const REFERENCE_GUEST_PARTITION_IDS: &[&str] = &["in", "mid", "out"];

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::elf::parse_elf64;

    #[test]
    fn all_reference_partition_elfs_parse() {
        for partition in REFERENCE_GUEST_PARTITION_IDS {
            let bytes = reference_guest_elf(partition).expect("elf");
            parse_elf64(bytes).expect("parse");
        }
    }
}
