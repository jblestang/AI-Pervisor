//! Guest datapath planning for IPC and device MMIO descriptors.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod constants;
mod error;
mod plan;

pub use constants::{
    E1000_MMIO_GUEST_PHYS_BASE, E1000_MMIO_GUEST_PHYS_STRIDE, E1000_MMIO_SIZE_BYTES,
};
pub use error::{DatapathError, DatapathErrorKind};
pub use plan::{plan_datapath_for_partition, plan_datapath_for_vm_id, DatapathPartitionPlan};
