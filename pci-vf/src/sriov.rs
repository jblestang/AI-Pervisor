//! SR-IOV VF NicBackend — real config-driven open path; MMIO thin until VF present.
//!
//! Opening probes for a bound VF at the configured BDF. When hardware is absent
//! (typical CI), [`SriovNic::open`] returns [`NicError::DeviceMissing`]. Full BAR
//! MMIO / descriptor rings are `unsafe` TCB and belong in `docs/TCB.md` once wired.

use alloc::string::String;

use crate::traits::{NicBackend, NicError};

/// Opened SR-IOV virtual function handle.
///
/// Fields are reserved for BAR mapping / queue state once a VF is bound.
#[derive(Debug)]
pub struct SriovNic {
    bdf: String,
    /// Placeholder until MMIO rings are mapped (Task 10 lab path).
    _mmio_reserved: (),
}

impl SriovNic {
    /// Attempt to open the VF at `bdf` (e.g. `"0000:01:10.0"` from config).
    ///
    /// Fail-fast when the device is not present — never substitutes Inject here;
    /// fallback policy lives in [`crate::open_from_config`].
    pub fn open(bdf: &str) -> Result<Self, NicError> {
        let bdf = bdf.trim();
        if bdf.is_empty() {
            return Err(NicError::MissingBdf);
        }
        if !probe_vf_present(bdf) {
            return Err(NicError::DeviceMissing {
                bdf: String::from(bdf),
            });
        }
        // SAFETY boundary (future): map VFIO region / BARs for `bdf`.
        Ok(Self {
            bdf: String::from(bdf),
            _mmio_reserved: (),
        })
    }

    #[inline]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }

    /// Host-only: construct a handle without sysfs probe to exercise the
    /// pre-MMIO stub `NicBackend` path (empty RX / refuse TX) until VF BARs
    /// are mapped. Not used by freestanding guests (`std` feature off).
    #[cfg(feature = "std")]
    pub fn stub_pre_mmio(bdf: &str) -> Self {
        Self {
            bdf: String::from(bdf),
            _mmio_reserved: (),
        }
    }
}

impl NicBackend for SriovNic {
    fn rx_ready(&self) -> bool {
        // No MMIO rings yet; a successfully opened VF would poll RX descriptors here.
        false
    }

    fn try_recv(&mut self, _buf: &mut [u8]) -> Result<usize, NicError> {
        Err(NicError::QueueEmpty)
    }

    fn try_send(&mut self, _frame: &[u8]) -> Result<(), NicError> {
        // Without mapped TX rings, refuse rather than pretend success.
        Err(NicError::IoFailure)
    }
}

/// Probe whether a VF at `bdf` is present and usable.
///
/// On host (`std`), checks sysfs PCI path. Under `no_std` (guests), always
/// reports absent — guests receive an already-selected backend via boot info.
///
/// Host tests may set `HV_TEST_VF_SYSFS_ROOT` to a temporary directory that
/// mirrors `/sys/bus/pci/devices/<bdf>` so the real probe path runs without lab
/// hardware (not a NicBackend mock).
fn probe_vf_present(bdf: &str) -> bool {
    #[cfg(feature = "std")]
    {
        probe_vf_sysfs(bdf)
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = bdf;
        false
    }
}

#[cfg(feature = "std")]
fn probe_vf_sysfs(bdf: &str) -> bool {
    let root = std::env::var_os("HV_TEST_VF_SYSFS_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/sys/bus/pci/devices"));
    // Canonical sysfs PCI device node; present iff the kernel enumerated the BDF
    // (or a host-test tree created the same layout under HV_TEST_VF_SYSFS_ROOT).
    let path = root.join(bdf);
    path.exists()
}
