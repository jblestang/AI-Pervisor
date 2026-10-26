//! SR-IOV VF NicBackend — real config-driven open path; MMIO thin until VF present.
//!
//! Opening probes for a bound VF at the configured BDF. When hardware is absent
//! (typical CI), [`SriovNic::open`] returns [`NicError::DeviceMissing`]. Full BAR
//! MMIO / descriptor rings are `unsafe` TCB and belong in `docs/TCB.md` once wired.
//!
//! Task 10 lab path: host probe requires a `vfio-pci` driver symlink under the
//! sysfs device (refuse unbound / wrong-driver VFs). Guests never probe sysfs —
//! they consume the backend tip from the hv-filled boot-info blob.

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
    /// Fail-fast when the device is not present or not usable — never substitutes
    /// Inject here; fallback policy lives in [`crate::open_from_config`].
    pub fn open(bdf: &str) -> Result<Self, NicError> {
        let bdf = bdf.trim();
        if bdf.is_empty() {
            return Err(NicError::MissingBdf);
        }
        match probe_vf_status(bdf) {
            VfStatus::Ready => {}
            VfStatus::Missing | VfStatus::NotVfioBound { .. } => {
                return Err(NicError::DeviceMissing {
                    bdf: String::from(bdf),
                });
            }
        }
        // SAFETY boundary (future): map VFIO region / BARs for `bdf`.
        // Remaining gap: descriptor-ring MMIO + MSI-X — see docs/HARNESS.md
        // "Remaining Sriov E2E gaps".
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

/// Host probe outcome for a configured VF BDF.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VfStatus {
    /// Sysfs node absent (or empty BDF).
    Missing,
    /// Device enumerated but driver is missing / not `vfio-pci`.
    NotVfioBound {
        /// Current driver leaf name if any (`None` = unbound).
        driver: Option<String>,
    },
    /// Device present and bound to `vfio-pci` (or test tree with vfio symlink).
    Ready,
}

/// Probe whether a VF at `bdf` is present and usable for Sriov open.
///
/// On host (`std`), checks sysfs PCI path + `vfio-pci` bind. Under `no_std`
/// (guests), always reports [`VfStatus::Missing`] — guests receive an
/// already-selected backend via boot info.
///
/// Host tests may set `HV_TEST_VF_SYSFS_ROOT` to a temporary directory that
/// mirrors `/sys/bus/pci/devices/<bdf>` so the real probe path runs without lab
/// hardware (not a NicBackend mock).
pub fn probe_vf_status(bdf: &str) -> VfStatus {
    #[cfg(feature = "std")]
    {
        probe_vf_sysfs(bdf)
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = bdf;
        VfStatus::Missing
    }
}

#[cfg(feature = "std")]
fn probe_vf_sysfs(bdf: &str) -> VfStatus {
    let root = std::env::var_os("HV_TEST_VF_SYSFS_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/sys/bus/pci/devices"));
    let path = root.join(bdf);
    if !path.is_dir() {
        return VfStatus::Missing;
    }
    let driver_link = path.join("driver");
    if !driver_link.exists() {
        // Enumerated but unbound — not ready for vfio-pci passthrough.
        return VfStatus::NotVfioBound { driver: None };
    }
    match std::fs::read_link(&driver_link) {
        Ok(target) => {
            let leaf = target
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if leaf == "vfio-pci" {
                VfStatus::Ready
            } else {
                VfStatus::NotVfioBound {
                    driver: Some(String::from(leaf)),
                }
            }
        }
        Err(_) => {
            // Non-symlink driver node (unusual); treat as not vfio-bound.
            VfStatus::NotVfioBound {
                driver: Some(String::from("?")),
            }
        }
    }
}
