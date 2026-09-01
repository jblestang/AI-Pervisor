# Phase 48 expert review — live QEMU/KVM without validate-only mocks

## Scope

Strict REAL_HW live smoke harness for running the boot chain under KVM without accepting validate-only mock proof.

## Changes

| Area | Change |
|------|--------|
| `xtask/src/live_qemu_smoke.rs` | `LiveQemuSmokeOptions`, OVMF/KVM serial probe, strict marker evaluation |
| `xtask/src/lib.rs` | `--require-executed`, `--no-skip` CLI flags |
| `docs/ovmf-boot.md` | Document strict experiment command |

## Strict mode

```bash
cargo xtask live-qemu-smoke --require-executed --no-skip --build
```

Requires:

- OVMF/KVM serial probe success (non-empty serial under KVM)
- All Gate D validate markers
- `Gate D: guest throughput measured under live VMX`
- `GUEST: datapath relay benchmark complete`
- All three REAL_HW VMX markers (VMXON, EPT, VMLAUNCH)
- `Gate D: guest source-tree code executed under VMX for all partitions`

## Environment result (this cloud VM)

- `/dev/kvm` present, host VMX in `/proc/cpuinfo`, nested=Y
- OVMF under KVM produces **zero serial bytes** (hangs; dmesg shows `kvm_spurious_fault` on nested VMX)
- Strict mode fails fast in ~9s with a clear probe error instead of 90s timeout + empty log
- TCG boot chain loads REAL_HW firmware but fails platform check (`vmx` absent under emulated CPU)

**Conclusion:** Full without-mocks execution requires bare metal or a nested-virt-capable host where OVMF serial works under KVM.

## Verification

- `cargo test -p xtask --lib` — 99 tests
- `cargo xtask live-qemu-smoke --require-executed --no-skip --no-build` — fails fast with probe error on this host (expected)
