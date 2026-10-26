//! VM-exit decode: EPT violation → log + halt; UART I/O; HLT.

use crate::config::FlatHvConfig;
use crate::ept::{EptFault, EptGuest, EptIntent};
use crate::uart::{format_ept_fault_msg, handle_io_out, UartSink};
use crate::vmx::vmcs::{Vcpu, VcpuId};

/// Basic VM-exit reason numbers (SDM Vol. 3C Table 25-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum VmxExitReason {
    EptViolation = 48,
    IoInstruction = 30,
    Hlt = 12,
    Other(u32),
}

impl VmxExitReason {
    pub fn from_raw(reason: u32) -> Self {
        // Basic exit reason is bits 15:0.
        match reason & 0xffff {
            48 => Self::EptViolation,
            30 => Self::IoInstruction,
            12 => Self::Hlt,
            other => Self::Other(other),
        }
    }
}

/// Action requested after handling an exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitAction {
    /// Resume the same guest.
    Resume,
    /// Guest is halted; schedule peers if any.
    Halted,
    /// Fatal hypervisor condition.
    TripleFault,
}

/// Run-state for both demonstrator vCPUs.
pub struct GuestRunState {
    pub wan: Vcpu,
    pub lan: Vcpu,
}

impl GuestRunState {
    pub fn new() -> Self {
        Self {
            wan: Vcpu::new(VcpuId::Wan),
            lan: Vcpu::new(VcpuId::Lan),
        }
    }

    pub fn vcpu_mut(&mut self, id: VcpuId) -> &mut Vcpu {
        match id {
            VcpuId::Wan => &mut self.wan,
            VcpuId::Lan => &mut self.lan,
        }
    }

    pub fn both_halted(&self) -> bool {
        self.wan.halted && self.lan.halted
    }
}

impl Default for GuestRunState {
    fn default() -> Self {
        Self::new()
    }
}

/// Pure exit policy (host-testable): apply EPT intent + UART config.
pub fn handle_exit<S: UartSink>(
    cfg: &FlatHvConfig,
    intent: &EptIntent,
    state: &mut GuestRunState,
    current: VcpuId,
    reason: VmxExitReason,
    qual: u64,
    gpa: u64,
    sink: &mut S,
) -> ExitAction {
    match reason {
        VmxExitReason::EptViolation => {
            let guest = current.ept_guest();
            let fault = intent.classify_gpa_fault(guest, gpa).unwrap_or(EptFault {
                guest,
                gpa,
                halt_guest: true,
            });
            log_ept_fault(sink, fault);
            if fault.halt_guest {
                state.vcpu_mut(current).halt();
                ExitAction::Halted
            } else {
                ExitAction::Resume
            }
        }
        VmxExitReason::IoInstruction => {
            // Qualification: bits 15:0 port? Actually bits 31:16 = port, bit 0 = direction (0=out).
            let port = ((qual >> 16) & 0xffff) as u16;
            let is_out = (qual & 1) == 0;
            let size = ((qual >> 4) & 7) + 1;
            if is_out && size >= 1 {
                // Guest RAX low byte would be supplied by the VMX asm path; for the
                // policy helper, `gpa` is overloaded as the data byte when testing.
                let byte = (gpa & 0xff) as u8;
                let _ = handle_io_out(cfg, sink, port, byte);
            }
            ExitAction::Resume
        }
        VmxExitReason::Hlt => {
            state.vcpu_mut(current).halt();
            ExitAction::Halted
        }
        VmxExitReason::Other(_) => {
            // Unexpected exit: log + halt faulting guest (design §8).
            let mut buf = [0u8; 64];
            let tag = match current {
                VcpuId::Wan => &b"wan"[..],
                VcpuId::Lan => &b"lan"[..],
            };
            let n = format_ept_fault_msg(tag, gpa, &mut buf);
            // Reuse formatter prefix space: write a short marker.
            for &b in b"EXIT " {
                sink.write_byte(b);
            }
            for &b in tag {
                sink.write_byte(b);
            }
            sink.write_byte(b'\n');
            let _ = n;
            state.vcpu_mut(current).halt();
            ExitAction::Halted
        }
    }
}

fn log_ept_fault<S: UartSink>(sink: &mut S, fault: EptFault) {
    let tag: &[u8] = match fault.guest {
        EptGuest::Wan => b"wan",
        EptGuest::Lan => b"lan",
    };
    let mut buf = [0u8; 48];
    let n = format_ept_fault_msg(tag, fault.gpa, &mut buf);
    for &b in &buf[..n] {
        sink.write_byte(b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot_blob::fill_boot_info;
    use crate::config::FlatHvConfig;
    use crate::ept::build_ept_intent;
    use crate::guest_mem::build_layout;
    use crate::uart::CaptureUart;
    use guest_common::GuestRole;

    fn sample_cfg() -> FlatHvConfig {
        FlatHvConfig {
            wan_memory_mb: 2,
            lan_memory_mb: 2,
            wan_vcpus: 1,
            lan_vcpus: 1,
            ring_capacity_pow2: 8,
            frame_capacity: 64,
            gpa_window_base: 0x1000_0000,
            uart_io_port: 0x3F8,
            _uart_pad: 0,
            inject_queue_capacity: 8,
            nic_backend_inject: 1,
        }
    }

    #[test]
    fn ept_violation_halts_only_faulting_guest() {
        let cfg = sample_cfg();
        let layout = build_layout(&cfg).unwrap();
        let intent = build_ept_intent(&layout).unwrap();
        let mut state = GuestRunState::new();
        let mut uart = CaptureUart::default();

        let action = handle_exit(
            &cfg,
            &intent,
            &mut state,
            VcpuId::Wan,
            VmxExitReason::EptViolation,
            0,
            layout.lan_private.base,
            &mut uart,
        );
        assert_eq!(action, ExitAction::Halted);
        assert!(state.wan.halted);
        assert!(!state.lan.halted);
        assert!(uart.bytes.windows(9).any(|w| w == b"EPT fault"));
    }

    #[test]
    fn uart_io_out_captured() {
        let cfg = sample_cfg();
        let layout = build_layout(&cfg).unwrap();
        let intent = build_ept_intent(&layout).unwrap();
        let mut state = GuestRunState::new();
        let mut uart = CaptureUart::default();
        // qual: OUT to port 0x3F8
        let qual = (u64::from(cfg.uart_io_port) << 16) | (0 << 0);
        let action = handle_exit(
            &cfg,
            &intent,
            &mut state,
            VcpuId::Lan,
            VmxExitReason::IoInstruction,
            qual,
            u64::from(b'Z'),
            &mut uart,
        );
        assert_eq!(action, ExitAction::Resume);
        assert_eq!(uart.bytes, b"Z");
        let _ = fill_boot_info(&cfg, &layout, GuestRole::Lan);
    }
}
