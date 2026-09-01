# Phase 49 expert review — QEMU e1000 host net + MMIO relay fixes

## Scope

First production-facing steps toward a real QEMU/KVM datapath experiment:

1. Config-driven outer-QEMU e1000 + netdev wiring
2. Guest/hypervisor e1000 semantic alignment (OUT uses RDT)
3. VM-exit MMIO relay mirrors emulated registers into the identity-mapped guest view

## Changes

| Area | Change |
|------|--------|
| `configs/qemu.yaml` | `qemu.network` with user netdev + e1000 at BDF `00:03.0` / `00:04.0` |
| `hv-config-model` | `RawQemuNetwork`, normalization |
| `xtask/src/qemu_network.rs` | Plans `-netdev` / `-device e1000` CLI args from config |
| `xtask/src/live_qemu_smoke.rs` | Uses network plan by default; `--no-host-net` override |
| `guests/guest-common/src/e1000.rs` | OUT `rx_advance()` writes RDT (not read-only RDH) |
| `hv-x86-cpu/src/vmexit_mmio_relay.rs` | Propagate `guest_rax` write values; mirror guest MMIO view |
| `crates/hv-hypervisor-boot/src/gate_d.rs` | Initialize e1000 guest MMIO view at relay install |

## Usage

```bash
cargo xtask build-boot-chain-live
cargo xtask live-qemu-smoke --require-executed --no-skip --build
```

## Still deferred

- Nested-guest BAR discovery from outer PCI (nested guests still use synthetic MMIO GPAs)
- Descriptor rings, UDP/IPv4, DMA, hypervisor proxy to outer QEMU NIC
- Concurrent in/mid/out partition scheduling
- Full 200 Mbit/s host-side throughput proof

## Verification

- `cargo test -p xtask --lib` — 102 tests
- `cargo test -p hv-config-model`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement vmexit_mmio`
