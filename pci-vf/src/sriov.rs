//! SR-IOV VF NicBackend — config-driven open + BAR/MMIO tip wiring.
//!
//! Host open probes for a bound VF at the configured BDF. When hardware is absent
//! (typical CI), [`SriovNic::open`] returns [`NicError::DeviceMissing`]. Guests
//! never probe sysfs — they open via [`SriovNic::from_mmio_tips`] using BootInfo
//! `nic_mmio_*` filled by `hv` from config.
//!
//! BAR MMIO / descriptor rings are intentional TCB (`docs/TCB.md` TCB-NIC-001).
//! Without a non-zero MMIO window the datapath refuses TX and reports empty RX
//! (honest pre-hardware / absent-VF behaviour — not a silent Inject substitute).

use alloc::string::String;

use crate::traits::{NicBackend, NicError};

/// Opened SR-IOV virtual function handle.
#[derive(Debug)]
pub struct SriovNic {
    bdf: String,
    /// Guest-physical (or host-mapped) BAR window tip; 0 = not programmed.
    mmio_gpa: u64,
    /// BAR window length in bytes; 0 = no usable MMIO.
    mmio_len: u64,
    /// When non-null + `mmio_len > 0`, points at the first byte of the BAR window.
    mmio: Option<*mut u8>,
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
        // Host path: VF is vfio-bound but BAR tips come from BootInfo / config
        // when guests run. Until VFIO region map is wired, leave MMIO unset —
        // datapath fails honestly (empty RX / IoFailure TX).
        Ok(Self {
            bdf: String::from(bdf),
            mmio_gpa: 0,
            mmio_len: 0,
            mmio: None,
        })
    }

    /// Guest / hv path: open from BootInfo MMIO tips (`IF-BOOT-003`).
    ///
    /// - `mmio_len == 0` or null `mmio` → [`NicError::DeviceMissing`] (lab VF BAR
    ///   absent / not programmed — no silent Inject).
    /// - Non-zero window → handle holds the tip; descriptor rings still require
    ///   real VF hardware (RX empty / TX `IoFailure` until rings are live).
    pub fn from_mmio_tips(mmio_gpa: u64, mmio_len: u64, mmio: *mut u8) -> Result<Self, NicError> {
        if mmio_len == 0 || mmio.is_null() {
            return Err(NicError::DeviceMissing {
                bdf: String::from("bootinfo-mmio"),
            });
        }
        Ok(Self {
            bdf: String::from("bootinfo-mmio"),
            mmio_gpa,
            mmio_len,
            mmio: Some(mmio),
        })
    }

    #[inline]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }

    #[inline]
    pub fn mmio_gpa(&self) -> u64 {
        self.mmio_gpa
    }

    #[inline]
    pub fn mmio_len(&self) -> u64 {
        self.mmio_len
    }

    /// True when a non-zero BAR window tip is attached (rings may still be absent).
    #[inline]
    pub fn has_mmio_window(&self) -> bool {
        self.mmio_len > 0 && self.mmio.is_some()
    }

    /// Read a little-endian `u32` at `offset` within the BAR window (TCB).
    ///
    /// Returns `None` when the window is missing or the offset is out of range.
    pub fn mmio_read_u32(&self, offset: u64) -> Option<u32> {
        let ptr = self.mmio?;
        if offset
            .checked_add(4)
            .map(|e| e > self.mmio_len)
            .unwrap_or(true)
        {
            return None;
        }
        // SAFETY: caller opened with a valid mapped window of `mmio_len` bytes.
        let val = unsafe { core::ptr::read_volatile(ptr.add(offset as usize) as *const u32) };
        Some(val)
    }

    /// Host-only: construct a handle without sysfs probe to exercise the
    /// pre-MMIO stub `NicBackend` path (empty RX / refuse TX) until VF BARs
    /// are mapped. Not used by freestanding guests (`std` feature off).
    #[cfg(feature = "std")]
    pub fn stub_pre_mmio(bdf: &str) -> Self {
        Self {
            bdf: String::from(bdf),
            mmio_gpa: 0,
            mmio_len: 0,
            mmio: None,
        }
    }
}

impl NicBackend for SriovNic {
    fn rx_ready(&self) -> bool {
        // Descriptor rings not yet programmed against a real VF BAR layout.
        // A live window still reports empty until HW rings are wired.
        let _ = self.has_mmio_window();
        false
    }

    fn try_recv(&mut self, _buf: &mut [u8]) -> Result<usize, NicError> {
        Err(NicError::QueueEmpty)
    }

    fn try_send(&mut self, _frame: &[u8]) -> Result<(), NicError> {
        // Without mapped TX descriptor rings, refuse rather than pretend success.
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
