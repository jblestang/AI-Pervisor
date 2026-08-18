//! VT-d initialization planning and backend abstraction.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod backend;
mod error;
mod init;
mod plan;

pub use backend::{FailingVtdBackend, MockVtdBackend, VtdBackend};
pub use error::{VtdError, VtdErrorKind};
pub use init::{init_vtd, vtd_init_required};
pub use plan::{plan_vtd_init, VtdDeviceAssignment, VtdInitPlan};
