//! UART emulation via VM exit (I/O port from boot / flat config).

use crate::config::FlatHvConfig;

/// I/O exit decoded as a UART data write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UartByte {
    pub port: u16,
    pub byte: u8,
}

/// Host-side serial sink used when an I/O exit hits the configured UART port.
///
/// Freestanding `hv` installs a sink that writes to the platform UART MMIO/port
/// (see TCB). Host tests can use [`CaptureUart`].
pub trait UartSink {
    fn write_byte(&mut self, byte: u8);
}

/// Capturing sink for host tests / bring-up logs.
#[derive(Clone, Debug, Default)]
pub struct CaptureUart {
    pub bytes: alloc::vec::Vec<u8>,
}

impl UartSink for CaptureUart {
    fn write_byte(&mut self, byte: u8) {
        self.bytes.push(byte);
    }
}

/// True when `port` matches the UART tip from config (data port).
#[inline]
pub fn is_uart_data_port(cfg: &FlatHvConfig, port: u16) -> bool {
    port == cfg.uart_io_port
}

/// Handle an OUT to the UART data port; returns `Some` when consumed.
pub fn handle_io_out<S: UartSink>(
    cfg: &FlatHvConfig,
    sink: &mut S,
    port: u16,
    value: u8,
) -> Option<UartByte> {
    if !is_uart_data_port(cfg, port) {
        return None;
    }
    sink.write_byte(value);
    Some(UartByte {
        port,
        byte: value,
    })
}

/// Format a short EPT-fault banner into `out` (no_std, no alloc formatting crate).
pub fn format_ept_fault_msg(guest_tag: &[u8], gpa: u64, out: &mut [u8]) -> usize {
    // "EPT fault <tag> gpa=0x................\n"
    let prefix = b"EPT fault ";
    let mid = b" gpa=0x";
    let mut n = 0usize;
    for &b in prefix {
        if n >= out.len() {
            return n;
        }
        out[n] = b;
        n += 1;
    }
    for &b in guest_tag {
        if n >= out.len() {
            return n;
        }
        out[n] = b;
        n += 1;
    }
    for &b in mid {
        if n >= out.len() {
            return n;
        }
        out[n] = b;
        n += 1;
    }
    // 16 hex digits
    for i in (0..16).rev() {
        if n >= out.len() {
            return n;
        }
        let nib = ((gpa >> (i * 4)) & 0xf) as u8;
        out[n] = if nib < 10 { b'0' + nib } else { b'a' + (nib - 10) };
        n += 1;
    }
    if n < out.len() {
        out[n] = b'\n';
        n += 1;
    }
    n
}
