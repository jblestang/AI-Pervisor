# Phase 39 expert review

INVEPT after runtime EPT updates for relay measurement page install (`cursor/phase-39-invept-runtime-ept-reload-0b4f`).

## Changes

| Component | Change |
|-----------|--------|
| Live asm | `invept_single_context()` issues INVEPT type-1 with 128-bit descriptor |
| EPT instructions | `execute_invept_single_context()` gated like EPT pointer VMWRITE |
| CPU seam | `run_ept_pointer_reload_cpu_seam_batch()` — INVEPT once, then EPT pointer VMWRITE per VMCS |
| Gate D | Replaces per-VMCS `run_ept_pointer_cpu_seam` loop after measurement page EPT install |

## Closes Phase 35–37 deferral

Runtime EPT table mutation (measurement page append + reinstall) now invalidates derived EPT caches before reloading the EPT pointer on each partition VMCS.

## Verification

- `cargo test -p hv-ept`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
