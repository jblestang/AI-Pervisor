//! Guest boot info construction and smoke guest images for VMX launch bring-up.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod boot_info;
mod smoke;

pub use boot_info::{
    build_guest_boot_info_for_partition, GuestBootInfoBuildError, GuestBootInfoBuildErrorKind,
};
pub use smoke::{GUEST_SMOKE_IMAGE, GUEST_SMOKE_RUNNING_MARKER};
