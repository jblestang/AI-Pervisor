# Phase 54 expert review — QEMU live test harness

## Scope

Make REAL_HW live QEMU smoke reproducible on a physical or nested-KVM test machine by automating host tap setup and documenting the end-to-end boot workflow.

## Changes

| Area | Change |
|------|--------|
| `xtask` | `setup-host-net-taps`: create/bring up taps from config host network plan |
| `xtask` | `host_net_taps.rs`: config-driven tap names, existence checks, preflight errors |
| `xtask` | `live-qemu-smoke`: fail fast when required taps are missing |
| `docs/ovmf-boot.md` | Quick-start for QEMU test machines; BAR0 discovery marker in expected serial output |

## Usage

```bash
cargo xtask setup-host-net-taps
cargo xtask build-guests
cargo xtask build-boot-chain-live
cargo xtask live-qemu-smoke --require-executed --no-skip --build
```

Requires `/dev/kvm`, host VMX, OVMF, QEMU, and nested virt. IN/OUT taps (`hvdp-in0`, `hvdp-out0` in the reference config) must exist before host-network smoke unless `--no-host-net` is passed.

## Still deferred

- QEMU `fixed-bars` / `pci-bars` pinning
- Descriptor rings, DMA, live tap frame I/O
- Concurrent in/mid/out partition scheduling

## Verification

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features`
- Opt-in live: `cargo xtask live-qemu-smoke --require-executed --no-skip --build`
