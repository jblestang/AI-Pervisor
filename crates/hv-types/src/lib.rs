//! Strongly typed identifiers and overflow-safe arithmetic for the hypervisor.
//!
//! All address and identifier newtypes are intentionally not interchangeable.

#![no_std]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]

mod arith;
mod ids;
mod units;

pub use arith::{
    align_down, align_up, checked_add_usize, checked_mul_usize, is_aligned, ArithmeticError,
};
pub use ids::{
    ApicId, GuestPhysAddr, GuestVirtAddr, HostPhysAddr, HostVirtAddr, InterruptVector,
    IommuDomainId, Iova, IpcChannelId, LogicalCpuId, PackageId, PciBdf, PciBus, PciDevice,
    PciFunction, PciSegment, PhysicalCoreId, VcpuId, VmId,
};
pub use units::{ByteSize, Gibibyte, PageCount};
