//! Type-1 hypervisor library: config-driven layout, EPT intent, boot blob, VMX.
//!
//! Host unit tests cover the EPT map builder and boot-info fill (no QEMU).
//! VT-x / EPT table load paths are thin `unsafe` TCB — see `docs/TCB.md`.

#![no_std]

extern crate alloc;

pub mod boot_blob;
pub mod config;
pub mod ept;
pub mod ept_install;
pub mod ept_tables;
pub mod guest_load;
pub mod guest_mem;
pub mod l2;
pub mod multiboot;
pub mod smoke_l1;
pub mod uart;
pub mod vmx;

/// 4 KiB page size used for EPT / guest GPA packing (architectural).
pub const PAGE_SIZE: u64 = 4096;
