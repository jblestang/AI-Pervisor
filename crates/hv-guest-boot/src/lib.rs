//! Guest boot info construction and smoke guest images for VMX launch bring-up.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod boot_info;
mod parse;
mod smoke;

pub use boot_info::{
    build_guest_boot_info_for_partition, build_guest_boot_info_for_vm_id,
    build_guest_boot_infos_all_partitions, GuestBootInfoBuildError, GuestBootInfoBuildErrorKind,
};
pub use parse::{
    GuestBootInfoParseError, GuestBootInfoParseErrorKind, GuestBootInfoView,
};
pub use smoke::{GUEST_SMOKE_IMAGE, GUEST_SMOKE_RUNNING_MARKER};
