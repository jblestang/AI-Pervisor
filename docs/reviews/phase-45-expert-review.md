# Phase 45 expert review

VM-exit/resume relay loop for e1000 MMIO datapath (`cursor/phase-45-vmexit-mmio-relay-loop-0b4f`).

## Changes

| Component | Change |
|-----------|--------|
| EPT permissions | `set_ept_mapping_guest_writable()` patches guest write on existing mappings |
| MMIO relay | `VmexitE1000MmioConfig`; EPT write trap + hypervisor-side `E1000MmioState` emulation |
| Dispatch | `VmexitRelayDispatchPlan` combines measurement-page and per-VM e1000 MMIO handlers |
| Guest run loop | `run_vmx_guest_until_halt_with_relay_dispatch()` tracks relay frames and MMIO events |
| Execution seam | `vmexit_mmio_relay_events` on `DatapathGuestExecutionCpuSeamOutcome` |
| Gate D | Read-only e1000 EPT traps, state page install, INVEPT reload before guest execution |

## Trust model

Guest e1000 MMIO writes now trap via read-only EPT mappings; the hypervisor emulates doorbell updates in host-owned state pages on VM-exit instead of allowing direct guest writes to identity-mapped MMIO. IPC relay remains guest-side for this phase; MMIO is the first live VM-exit/resume datapath relay slice.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test -p hv-x86-cpu --features datapath-guest-relay-measurement`
- `cargo test -p hv-hypervisor-boot --features datapath-guest-relay-measurement`
- `cargo test -p hv-ept`
- `cargo xtask build-guests && cargo xtask build-boot-chain-live`
