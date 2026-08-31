//! Guest boot info construction and smoke guest images for VMX launch bring-up.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod boot_info;
mod elf;
mod parse;
mod partition_images;
mod smoke;

pub use boot_info::{
    build_guest_boot_info_for_partition, build_guest_boot_info_for_vm_id,
    build_guest_boot_infos_all_partitions, GuestBootInfoBuildError, GuestBootInfoBuildErrorKind,
};
pub use parse::{
    GuestBootInfoParseError, GuestBootInfoParseErrorKind, GuestBootInfoView,
};
pub use elf::{
    guest_entry_phys_for_region, parse_elf64, GuestElfError, GuestElfErrorKind, GuestElfImage,
    GuestElfLoadSegment,
};
pub use partition_images::{
    reference_datapath_guest_elf, reference_guest_elf, reference_guest_elf_for_kind,
    reference_guest_elf_for_vm_id, GuestElfKind, GUEST_DATAPATH_CAPABLE_MARKER,
    GUEST_IN_RUNNING_MARKER, GUEST_MID_RUNNING_MARKER, GUEST_OUT_RUNNING_MARKER,
    REFERENCE_GUEST_PARTITION_IDS,
};
pub use smoke::{GUEST_SMOKE_IMAGE, GUEST_SMOKE_RUNNING_MARKER};
