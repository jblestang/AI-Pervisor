//! Minimal Intel e1000 MMIO register model for datapath smoke.

use crate::error::{DatapathError, DatapathErrorKind};

/// MMIO offset for transmit descriptor head.
pub const E1000_REG_TDH: u64 = 0x3810;
/// MMIO offset for transmit descriptor tail.
pub const E1000_REG_TDT: u64 = 0x3818;
/// MMIO offset for receive descriptor head.
pub const E1000_REG_RDH: u64 = 0x2810;
/// MMIO offset for receive descriptor tail.
pub const E1000_REG_RDT: u64 = 0x2818;

/// Mutable e1000 MMIO state for smoke datapath.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct E1000MmioState {
    /// Transmit descriptor head.
    pub tdh: u32,
    /// Transmit descriptor tail.
    pub tdt: u32,
    /// Receive descriptor head.
    pub rdh: u32,
    /// Receive descriptor tail.
    pub rdt: u32,
    /// Whether a TX doorbell write was observed.
    pub tx_doorbell: bool,
    /// Whether an RX doorbell write was observed.
    pub rx_doorbell: bool,
}

/// Handles one e1000 MMIO read.
pub fn handle_e1000_mmio_read(state: &E1000MmioState, offset: u64) -> Result<u64, DatapathError> {
    match offset {
        E1000_REG_TDH => Ok(u64::from(state.tdh)),
        E1000_REG_TDT => Ok(u64::from(state.tdt)),
        E1000_REG_RDH => Ok(u64::from(state.rdh)),
        E1000_REG_RDT => Ok(u64::from(state.rdt)),
        _ => Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "unsupported e1000 mmio read offset",
        )),
    }
}

/// Handles one e1000 MMIO write.
pub fn handle_e1000_mmio_write(
    state: &mut E1000MmioState,
    offset: u64,
    value: u64,
) -> Result<(), DatapathError> {
    match offset {
        E1000_REG_TDT => {
            state.tdt = u32::try_from(value).map_err(|_| {
                DatapathError::new(DatapathErrorKind::InvalidInput, "tdt value overflow")
            })?;
            state.tx_doorbell = true;
            Ok(())
        }
        E1000_REG_RDT => {
            state.rdt = u32::try_from(value).map_err(|_| {
                DatapathError::new(DatapathErrorKind::InvalidInput, "rdt value overflow")
            })?;
            state.rx_doorbell = true;
            Ok(())
        }
        E1000_REG_TDH | E1000_REG_RDH => Err(DatapathError::new(
            DatapathErrorKind::IpcViolation,
            "guest attempted to write read-only e1000 head register",
        )),
        _ => Err(DatapathError::new(
            DatapathErrorKind::InvalidInput,
            "unsupported e1000 mmio write offset",
        )),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn tdt_write_sets_tx_doorbell() {
        let mut state = E1000MmioState::default();
        handle_e1000_mmio_write(&mut state, E1000_REG_TDT, 1).expect("write");
        assert!(state.tx_doorbell);
    }
}
