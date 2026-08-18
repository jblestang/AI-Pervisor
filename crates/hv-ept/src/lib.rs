//! EPT initialization planning and backend abstraction.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod backend;
mod constants;
mod error;
mod init;
mod plan;
mod program;

pub use backend::{FailingEptBackend, MockEptBackend, EptBackend};
pub use constants::{EPT_PAGE_SIZE_BYTES, EPT_ROOT_TABLE_BYTES};
pub use error::{EptError, EptErrorKind};
pub use init::{ept_init_required, init_ept};
pub use plan::{plan_ept_init, EptIdentityMapping, EptInitPlan};
pub use program::{
    encode_identity_ept_entry, program_ept_tables, EptProgrammedMapping, EptProgrammedTables,
    ProgrammingEptBackend, EPT_ENTRY_EXECUTE, EPT_ENTRY_MEMORY_TYPE_WB, EPT_ENTRY_READ,
    EPT_ENTRY_WRITE,
};
