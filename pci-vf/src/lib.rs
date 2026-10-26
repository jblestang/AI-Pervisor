//! SR-IOV / Inject NIC backends (`IF-NIC-*`, Task 4).
//!
//! - [`InjectNic`]: real in-process frame queues (degraded lab path — not a Sriov mock).
//! - [`SriovNic`]: config-driven VF open; fails fast when BDF/device missing (SWR-023).
//! - [`open_from_config`]: selects backend per ICD open policy.

#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod inject;
mod sriov;
mod traits;
mod verus_stub;

pub use inject::InjectNic;
pub use sriov::{probe_vf_status, SriovNic, VfStatus};
pub use traits::{BackendKind, FlattenedNicConfig, NicBackend, NicError};

/// Concrete backend returned by [`open_from_config`].
#[derive(Debug)]
pub enum Backend {
    Inject(InjectNic),
    Sriov(SriovNic),
}

impl NicBackend for Backend {
    fn rx_ready(&self) -> bool {
        match self {
            Backend::Inject(n) => n.rx_ready(),
            Backend::Sriov(n) => n.rx_ready(),
        }
    }

    fn try_recv(&mut self, buf: &mut [u8]) -> Result<usize, NicError> {
        match self {
            Backend::Inject(n) => n.try_recv(buf),
            Backend::Sriov(n) => n.try_recv(buf),
        }
    }

    fn try_send(&mut self, frame: &[u8]) -> Result<(), NicError> {
        match self {
            Backend::Inject(n) => n.try_send(frame),
            Backend::Sriov(n) => n.try_send(frame),
        }
    }
}

/// Open a NIC backend from flattened config (`IF-NIC-003`).
///
/// | Config | Device present | Result |
/// |--------|----------------|--------|
/// | `Inject` | N/A | [`Backend::Inject`] |
/// | `Sriov`, fallback off | no / missing BDF | **Error** |
/// | `Sriov`, fallback on | no | [`Backend::Inject`] |
/// | `Sriov` | yes | [`Backend::Sriov`] |
pub fn open_from_config(cfg: &FlattenedNicConfig<'_>) -> Result<Backend, NicError> {
    match cfg.kind {
        BackendKind::Inject => Ok(Backend::Inject(InjectNic::with_capacity(
            cfg.inject_queue_capacity,
        ))),
        BackendKind::Sriov => {
            let bdf = match cfg.vf_bdf.map(str::trim).filter(|s| !s.is_empty()) {
                Some(b) => b,
                None => return Err(NicError::MissingBdf),
            };
            match SriovNic::open(bdf) {
                Ok(nic) => Ok(Backend::Sriov(nic)),
                Err(NicError::DeviceMissing { .. }) if cfg.allow_inject_fallback => Ok(
                    Backend::Inject(InjectNic::with_capacity(cfg.inject_queue_capacity)),
                ),
                Err(e) => Err(e),
            }
        }
    }
}

/// Map `demo_config`-style backend name to [`BackendKind`] (no hard dependency).
///
/// Accepts `"sriov"` / `"inject"` (case-insensitive), matching `datapath.backend`.
pub fn backend_kind_from_str(s: &str) -> Option<BackendKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "sriov" => Some(BackendKind::Sriov),
        "inject" => Some(BackendKind::Inject),
        _ => None,
    }
}
