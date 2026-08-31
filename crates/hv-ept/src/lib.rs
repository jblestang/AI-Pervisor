//! EPT initialization planning and backend abstraction.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod backend;
mod constants;
mod error;
mod init;
mod paging;
mod plan;
mod program;
mod resolve;

pub use paging::{
    count_synthetic_entries, ept_maps_guest_page, ept_resolve_guest_page, materialize_ept_paging,
    patch_ept_table_host_phys,
};

pub use backend::{FailingEptBackend, MockEptBackend, EptBackend};
pub use constants::{
    EPT_PAGE_OFFSET_MASK, EPT_PAGE_SIZE_BYTES, EPT_POINTER_MEMORY_TYPE_SHIFT,
    EPT_POINTER_MEMORY_TYPE_WB, EPT_POINTER_PAGE_WALK_LENGTH,
    EPT_POINTER_PAGE_WALK_LENGTH_SHIFT, EPT_ROOT_TABLE_BYTES,
};
pub use error::{EptError, EptErrorKind};
pub use init::{ept_init_required, init_ept};
pub use plan::{plan_ept_init, EptIdentityMapping, EptInitPlan};
pub use program::{
    append_ept_guest_mapping, encode_identity_ept_entry, program_ept_tables, EptProgrammedMapping,
    EptProgrammedTables, ProgrammingEptBackend, EPT_ENTRY_EXECUTE, EPT_ENTRY_MEMORY_TYPE_WB,
    EPT_ENTRY_READ, EPT_ENTRY_WRITE,
};
pub use resolve::{resolve_guest_phys_range_to_host, resolve_guest_phys_to_host};
