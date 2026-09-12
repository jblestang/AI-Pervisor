# Phase 55 expert review — live QEMU environment preflight

## Scope

Give operators setting up a QEMU/KVM test machine a fast, actionable checklist before running the full REAL_HW live smoke boot.

## Changes

| Area | Change |
|------|--------|
| `xtask` | `live-qemu-preflight`: KVM, VMX, QEMU, timeout, OVMF, host tap checks |
| `xtask` | Optional `--run-kvm-probe` reuses OVMF/KVM serial probe from live smoke |
| `xtask` | CI fix: trim unused marker imports in `live_qemu_smoke.rs` |
| `docs/ovmf-boot.md` | Preflight step in QEMU test machine quick start |

## Usage

```bash
cargo xtask live-qemu-preflight
cargo xtask live-qemu-preflight --run-kvm-probe
```

Exit code 0 when every check passes; each line is `[ok]` or `[FAIL]` with remediation hints (for example missing taps → `setup-host-net-taps`).

## Still deferred

- QEMU `fixed-bars` / `pci-bars` pinning
- Descriptor rings, DMA, live tap frame I/O
- Concurrent in/mid/out partition scheduling

## Verification

- `cargo test --workspace`
- `cargo clippy --all-targets --all-features`
- Manual: `cargo xtask live-qemu-preflight` on a KVM host
